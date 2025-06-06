// SPDX-License-Identifier: GPL-3.0-or-later

pub mod tree;

use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    rc::Rc,
    time::Duration,
};

use indexmap::IndexSet;
use smithay::{
    desktop::{layer_map_for_output, utils::surface_primary_scanout_output, WindowSurface},
    output::{Output, WeakOutput},
    utils::{Logical, Rectangle},
};
use tokio::sync::mpsc::UnboundedSender;
use tracing::warn;
use tree::{LayoutNode, LayoutTree};

use crate::{
    backend::Backend,
    output::OutputName,
    state::{Pinnacle, State, WithState},
    tag::TagId,
    util::transaction::{PendingTransaction, TransactionBuilder},
    window::{UnmappingWindow, WindowElement, ZIndexElement},
};

impl Pinnacle {
    fn update_windows_with_geometries(
        &mut self,
        output: &Output,
        geometries: Vec<Rectangle<i32, Logical>>,
        backend: &mut Backend,
    ) {
        let (windows_on_foc_tags, to_unmap) = output.with_state(|state| {
            let focused_tags = state.focused_tags().cloned().collect::<IndexSet<_>>();
            self.windows
                .iter()
                .filter(|win| win.output(self).as_ref() == Some(output))
                .cloned()
                .partition::<Vec<_>, _>(|win| {
                    win.with_state(|state| state.tags.intersection(&focused_tags).next().is_some())
                })
        });

        let currently_mapped_wins = self.space.elements().collect::<HashSet<_>>();
        let maybe_unmap_wins = to_unmap.iter().collect::<HashSet<_>>();

        let to_unmap = currently_mapped_wins
            .intersection(&maybe_unmap_wins)
            .cloned()
            .cloned()
            .collect::<Vec<_>>();

        let mut snapshot_windows = Vec::new();

        for win in to_unmap {
            backend.with_renderer(|renderer| {
                win.capture_snapshot_and_store(
                    renderer,
                    output.current_scale().fractional_scale().into(),
                    1.0,
                );
            });

            if let Some(snap) = win.with_state_mut(|state| state.snapshot.take()) {
                let Some(loc) = self.space.element_location(&win) else {
                    unreachable!();
                };

                let unmapping = Rc::new(UnmappingWindow {
                    snapshot: snap,
                    fullscreen: win.with_state(|state| state.layout_mode.is_fullscreen()),
                    space_loc: loc,
                });

                let weak = Rc::downgrade(&unmapping);
                snapshot_windows.push(unmapping);

                let z_index = self
                    .z_index_stack
                    .iter()
                    .position(|z| matches!(z, crate::window::ZIndexElement::Window(w) if w == win))
                    .expect("window to be in the stack");

                self.z_index_stack
                    .insert(z_index, ZIndexElement::Unmapping(weak));
            }

            if win.with_state(|state| state.layout_mode.is_floating()) {
                if let Some(loc) = self.space.element_location(&win) {
                    win.with_state_mut(|state| state.set_floating_loc(loc));
                }
            }
            let to_schedule = self.space.outputs_for_element(&win);
            self.space.unmap_elem(&win);
            self.loop_handle.insert_idle(move |state| {
                for output in to_schedule {
                    state.schedule_render(&output);
                }
            });
        }

        let tiled_windows = windows_on_foc_tags
            .iter()
            .filter(|win| !win.is_x11_override_redirect())
            .filter(|win| win.with_state(|state| state.layout_mode.is_tiled()))
            .cloned();

        let output_geo = self.space.output_geometry(output).expect("no output geo");

        let non_exclusive_geo = {
            let map = layer_map_for_output(output);
            map.non_exclusive_zone()
        };

        let mut zipped = tiled_windows.zip(geometries.into_iter().map(|mut geo| {
            geo.loc += output_geo.loc + non_exclusive_geo.loc;
            geo
        }));

        let wins_and_geos_tiled = zipped.by_ref().map(|(win, geo)| (win, geo, true));
        let wins_and_geos_other = self
            .layout_state
            .pending_window_updates
            .take_next_for_output(output)
            .unwrap_or_default()
            .into_iter()
            .map(|(win, geo)| (win, geo, false));

        let wins_and_geos = wins_and_geos_tiled
            .chain(wins_and_geos_other)
            .collect::<Vec<_>>();

        for (win, geo, is_tiled) in wins_and_geos.iter() {
            if *is_tiled {
                win.set_tiled_states();
            } else {
                self.configure_window_if_nontiled(win);
            }
            match win.underlying_surface() {
                WindowSurface::Wayland(toplevel) => {
                    toplevel.with_pending_state(|state| {
                        state.size = Some(geo.size);
                    });
                }
                WindowSurface::X11(surface) => {
                    let _ = surface.configure(*geo);
                }
            }
        }

        let mut transaction_builder = TransactionBuilder::new(self.layout_state.pending_swap);

        for (win, geo, _) in wins_and_geos {
            if let WindowSurface::Wayland(toplevel) = win.underlying_surface() {
                let serial = toplevel.send_pending_configure();
                transaction_builder.add(&win, geo.loc, serial, &self.loop_handle);

                // Send a frame to get unmapped windows to update
                win.send_frame(
                    output,
                    self.clock.now(),
                    Some(Duration::ZERO),
                    surface_primary_scanout_output,
                );
            } else {
                transaction_builder.add(&win, geo.loc, None, &self.loop_handle);
            }
        }

        let mut unmapping = self
            .layout_state
            .pending_unmaps
            .take_next_for_output(output)
            .unwrap_or_default();

        unmapping.extend(snapshot_windows);

        self.layout_state
            .pending_transactions
            .entry(output.downgrade())
            .or_default()
            .push(transaction_builder.into_pending(unmapping));

        let (remaining_wins, _remaining_geos) = zipped.unzip::<_, _, Vec<_>, Vec<_>>();

        for win in remaining_wins {
            let to_schedule = self.space.outputs_for_element(&win);
            self.space.unmap_elem(&win);
            self.loop_handle.insert_idle(move |state| {
                for output in to_schedule {
                    state.schedule_render(&output);
                }
            });
        }
    }

    pub fn swap_window_positions(&mut self, win1: &WindowElement, win2: &WindowElement) {
        let win1_index = self.windows.iter().position(|win| win == win1);
        let win2_index = self.windows.iter().position(|win| win == win2);

        if let (Some(first), Some(second)) = (win1_index, win2_index) {
            self.windows.swap(first, second);
        }
    }
}

/// A monotonically increasing identifier for layout requests.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct LayoutRequestId(u32);

impl LayoutRequestId {
    pub fn to_inner(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Default)]
pub struct LayoutState {
    pub layout_request_sender: Option<UnboundedSender<LayoutInfo>>,
    pub pending_swap: bool,
    current_id: LayoutRequestId,
    pub layout_trees: HashMap<u32, LayoutTree>,

    /// Currently pending transactions.
    pub pending_transactions: HashMap<WeakOutput, Vec<PendingTransaction>>,

    pub pending_unmaps: PendingUnmaps,
    pub pending_window_updates: PendingWindowUpdates,
}

/// Pending [`UnmappingWindow`][crate::window::UnmappingWindow]s from things like
/// windows closing.
///
/// Pending unmapping windows are picked up by the next requested layout.
/// Once that layout completes, these windows are dropped and no longer rendered.
#[derive(Debug, Default)]
pub struct PendingUnmaps {
    pending: HashMap<WeakOutput, Vec<Vec<Rc<UnmappingWindow>>>>,
}

impl PendingUnmaps {
    /// Adds a set of [`UnmappingWindow`]s that should be displayed until the next layout finishes.
    pub fn add_for_output(&mut self, output: &Output, pending: Vec<Rc<UnmappingWindow>>) {
        self.pending
            .entry(output.downgrade())
            .or_default()
            .push(pending);
    }

    /// Takes the next set of [`UnmappingWindow`]s.
    pub fn take_next_for_output(&mut self, output: &Output) -> Option<Vec<Rc<UnmappingWindow>>> {
        let entry = self.pending.entry(output.downgrade()).or_default();

        (!entry.is_empty()).then(|| entry.remove(0))
    }
}

/// Pending window updates.
///
/// These are sets of windows and target geometries that are meant to be
/// synchronized with the next incoming layout.
#[derive(Debug, Default)]
pub struct PendingWindowUpdates {
    pending: HashMap<WeakOutput, Vec<Vec<(WindowElement, Rectangle<i32, Logical>)>>>,
}

impl PendingWindowUpdates {
    /// Adds a set of windows and target geometries that should be updated in tandem
    /// with the next incoming layout.
    pub fn add_for_output(
        &mut self,
        output: &Output,
        latched: Vec<(WindowElement, Rectangle<i32, Logical>)>,
    ) {
        self.pending
            .entry(output.downgrade())
            .or_default()
            .push(latched);
    }

    /// Takes the next set of windows and target geometries.
    pub fn take_next_for_output(
        &mut self,
        output: &Output,
    ) -> Option<Vec<(WindowElement, Rectangle<i32, Logical>)>> {
        let entry = self.pending.entry(output.downgrade()).or_default();

        (!entry.is_empty()).then(|| entry.remove(0))
    }
}

impl LayoutState {
    fn next_id(&mut self) -> LayoutRequestId {
        self.current_id.0 += 1;
        self.current_id
    }

    pub fn remove_output(&mut self, output: &Output) {
        self.pending_transactions.remove(&output.downgrade());
        self.pending_unmaps.pending.remove(&output.downgrade());
    }
}

#[derive(Debug)]
pub struct LayoutInfo {
    pub request_id: LayoutRequestId,
    pub output_name: OutputName,
    pub window_count: u32,
    pub tag_ids: Vec<TagId>,
}

impl State {
    /// Updates the layouts of outputs whose transactions have completed.
    pub fn update_layout(&mut self) {
        let _span = tracy_client::span!("State::update_layout");

        for output in self.pinnacle.outputs.keys().cloned().collect::<Vec<_>>() {
            let mut transactions = Vec::new();

            let txs = self
                .pinnacle
                .layout_state
                .pending_transactions
                .entry(output.downgrade())
                .or_default();

            while txs
                .first()
                .is_some_and(|t| t.is_completed() || t.is_cancelled())
            {
                let tx = txs.remove(0);
                if tx.is_swap {
                    self.pinnacle.layout_state.pending_swap = false;
                }
                if tx.is_completed() {
                    transactions.push(tx);
                }
            }

            for transaction in transactions {
                let mut outputs = Vec::new();
                for (window, loc) in transaction.target_locs {
                    if !window.is_on_active_tag() {
                        warn!("Attempted to map a window without active tags");
                        continue;
                    }
                    outputs.extend(window.output(&self.pinnacle));
                    self.pinnacle.space.map_element(window, loc, false);
                }
                for output in outputs {
                    self.schedule_render(&output);
                }
            }
        }

        let mut wins_to_update = Vec::new();

        for win in self.pinnacle.windows.iter() {
            let is_tiled = win.with_state(|state| state.layout_mode.is_tiled());
            let is_on_active_tag = win.is_on_active_tag();
            if !is_tiled && is_on_active_tag && !self.pinnacle.space.elements().any(|w| w == win) {
                wins_to_update.push(win.clone());
            }
        }

        for win in wins_to_update {
            self.update_window_layout_mode_and_layout(&win, |_| ());
        }
    }
}

impl Pinnacle {
    pub fn request_layout(&mut self, output: &Output) {
        if self
            .outputs
            .get(output)
            .is_some_and(|global| global.is_none())
        {
            return;
        }

        let id = self.layout_state.next_id();
        let Some(sender) = self.layout_state.layout_request_sender.as_ref() else {
            warn!("Layout requested but no client has connected to the layout service");
            return;
        };

        let windows_on_foc_tags = output.with_state(|state| {
            let focused_tags = state.focused_tags().cloned().collect::<IndexSet<_>>();
            self.windows
                .iter()
                .filter(|win| !win.is_x11_override_redirect())
                .filter(|win| {
                    win.with_state(|state| state.tags.intersection(&focused_tags).next().is_some())
                })
                .cloned()
                .collect::<Vec<_>>()
        });

        let window_count = windows_on_foc_tags
            .iter()
            .filter(|win| win.with_state(|state| state.layout_mode.is_tiled()))
            .count();

        let tag_ids = output.with_state(|state| state.focused_tags().map(|tag| tag.id()).collect());

        let _ = sender.send(LayoutInfo {
            request_id: id,
            output_name: OutputName(output.name()),
            window_count: window_count as u32,
            tag_ids,
        });
    }
}

impl State {
    pub fn apply_layout_tree(
        &mut self,
        tree_id: u32,
        root_node: LayoutNode,
        _request_id: u32,
        output_name: String,
    ) -> anyhow::Result<()> {
        let Some(output) = OutputName(output_name).output(&self.pinnacle) else {
            anyhow::bail!("Output was invalid");
        };

        let tree_entry = self.pinnacle.layout_state.layout_trees.entry(tree_id);
        let tree = match tree_entry {
            Entry::Occupied(occupied_entry) => {
                let tree = occupied_entry.into_mut();
                tree.diff(root_node);
                tree
            }
            Entry::Vacant(vacant_entry) => vacant_entry.insert(LayoutTree::new(root_node)),
        };

        let (output_width, output_height) = {
            let map = layer_map_for_output(&output);
            let zone = map.non_exclusive_zone();
            (zone.size.w, zone.size.h)
        };

        let geometries = tree.compute_geos(output_width as u32, output_height as u32);

        self.pinnacle
            .update_windows_with_geometries(&output, geometries, &mut self.backend);

        self.schedule_render(&output);

        Ok(())
    }
}
