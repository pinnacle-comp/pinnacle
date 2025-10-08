// SPDX-License-Identifier: GPL-3.0-or-later

pub mod tree;

use std::{
    collections::{HashMap, HashSet, hash_map::Entry},
    rc::Rc,
};

use indexmap::IndexSet;
use smithay::{
    desktop::layer_map_for_output,
    output::{Output, WeakOutput},
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel,
    utils::{Logical, Rectangle, Size},
};
use tokio::sync::mpsc::UnboundedSender;
use tracing::warn;
use tree::{LayoutNode, LayoutTree, ResizeDir};

use crate::{
    backend::Backend,
    output::OutputName,
    state::{Pinnacle, State, WithState},
    tag::TagId,
    util::transaction::{Location, PendingTransaction, TransactionBuilder},
    window::{UnmappingWindow, WindowElement},
};

impl Pinnacle {
    fn update_windows_from_tree(
        &mut self,
        output: &Output,
        backend: &mut Backend,
        is_resize: bool,
    ) {
        let Some(tree) = self.layout_state.current_tree_for_output(output) else {
            warn!("no layout tree for output");
            return;
        };

        let (output_width, output_height) = {
            let map = layer_map_for_output(output);
            let zone = map.non_exclusive_zone();
            (zone.size.w, zone.size.h)
        };

        let (geometries, nodes): (Vec<_>, Vec<_>) = tree
            .compute_geos(output_width as u32, output_height as u32)
            .into_iter()
            .unzip();

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
            if win.with_state(|state| state.layout_mode.is_floating())
                && let Some(loc) = self.space.element_location(&win)
            {
                win.with_state_mut(|state| state.set_floating_loc(loc));
            }

            if let Some(unmapping) = self.unmap_window(backend, &win, output) {
                snapshot_windows.push(unmapping);
            }
        }

        let tiled_windows = windows_on_foc_tags
            .iter()
            .filter(|win| !win.is_x11_override_redirect())
            .filter(|win| {
                win.with_state(|state| {
                    state.layout_mode.is_tiled() || state.layout_mode.is_spilled()
                })
            })
            .cloned();

        let Some(output_geo) = self.space.output_geometry(output) else {
            warn!("Cannot update_windows_from_tree without output geo");
            return;
        };

        let non_exclusive_geo = layer_map_for_output(output).non_exclusive_zone();

        let spilled_windows = tiled_windows
            .clone()
            .skip(geometries.len())
            .map(|w| {
                w.with_state_mut(|s| s.layout_mode.set_spilled(true));
                let geo = self
                    .compute_window_geometry(&w, output_geo, non_exclusive_geo)
                    .expect("compute_window_geometry cannot fail for spilled window");
                (w, geo, false)
            })
            .collect::<Vec<_>>();

        let mut zipped = tiled_windows.zip(geometries.into_iter().map(|mut geo| {
            geo.loc += output_geo.loc + non_exclusive_geo.loc;
            geo
        }));

        let wins_and_geos_tiled = zipped
            .by_ref()
            .map(|(win, geo)| (win, geo, true))
            .collect::<Vec<_>>();

        let just_wins = wins_and_geos_tiled.iter().map(|(win, ..)| win);

        for (win, node) in just_wins.zip(nodes) {
            win.with_state_mut(|state| state.layout_node = Some(node));
        }

        let wins_and_geos_other = self
            .layout_state
            .pending_window_updates
            .take_next_for_output(output)
            .unwrap_or_default()
            .into_iter()
            .map(|(win, geo)| (win, geo, false));

        let wins_and_geos = wins_and_geos_tiled
            .into_iter()
            .chain(spilled_windows)
            .chain(wins_and_geos_other)
            .collect::<Vec<_>>();

        let mut transaction_builder = TransactionBuilder::new();

        for (win, geo, is_tiled) in wins_and_geos {
            if is_tiled {
                win.with_state_mut(|s| s.layout_mode.set_spilled(false));
            }

            self.configure_window_and_add_map(&mut transaction_builder, &win, output, geo);
        }

        let mut unmapping = self
            .layout_state
            .pending_unmaps
            .take_next_for_output(output)
            .unwrap_or_default();

        unmapping.extend(snapshot_windows);

        self.layout_state.pending_transactions.add_for_output(
            output,
            transaction_builder.into_pending(unmapping, self.layout_state.pending_swap, is_resize),
        );

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
    pub pending_resize: bool,
    current_id: LayoutRequestId,

    pub current_layout_tree_ids: HashMap<WeakOutput, u32>,
    pub layout_trees: HashMap<WeakOutput, HashMap<u32, LayoutTree>>,

    pub pending_transactions: PendingTransactions,
    pub pending_unmaps: PendingUnmaps,
    pub pending_window_updates: PendingWindowUpdates,
}

/// Currently pending transactions.
#[derive(Debug, Default)]
pub struct PendingTransactions {
    pending: HashMap<WeakOutput, Vec<PendingTransaction>>,
}

impl PendingTransactions {
    /// Adds a pending transaction.
    pub fn add_for_output(&mut self, output: &Output, pending: PendingTransaction) {
        self.pending
            .entry(output.downgrade())
            .or_default()
            .push(pending);
    }

    /// Takes the next completed or cancelled transaction.
    pub fn take_next_for_output(&mut self, output: &Output) -> Option<PendingTransaction> {
        let entry = self.pending.entry(output.downgrade()).or_default();

        let next = entry.first()?;

        if next.is_completed() || next.is_cancelled() {
            return Some(entry.remove(0));
        }

        None
    }

    #[cfg(feature = "testing")]
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty() || self.pending.iter().all(|(_, v)| v.is_empty())
    }
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
        self.pending_transactions
            .pending
            .remove(&output.downgrade());
        self.pending_unmaps.pending.remove(&output.downgrade());
        self.pending_window_updates
            .pending
            .remove(&output.downgrade());
        self.layout_trees.remove(&output.downgrade());
    }

    pub fn current_tree_for_output(&mut self, output: &Output) -> Option<&mut LayoutTree> {
        self.layout_trees
            .entry(output.downgrade())
            .or_default()
            .get_mut(self.current_layout_tree_ids.get(&output.downgrade())?)
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

        let mut outputs = HashSet::new();

        for output in self.pinnacle.outputs.clone() {
            let mut transactions = Vec::new();

            while let Some(tx) = self
                .pinnacle
                .layout_state
                .pending_transactions
                .take_next_for_output(&output)
            {
                if tx.is_swap {
                    self.pinnacle.layout_state.pending_swap = false;
                }
                if tx.is_resize {
                    self.pinnacle.layout_state.pending_resize = false;
                }
                if tx.is_completed() {
                    transactions.push(tx);
                }
            }

            let mut locs = HashMap::new();

            for transaction in transactions {
                for (window, loc) in transaction.target_locs {
                    if !window.is_on_active_tag() {
                        warn!("Attempted to map a window without active tags");
                        continue;
                    }

                    if !self.pinnacle.windows.contains(&window) {
                        // The window closed in the time between the transaction and here.
                        continue;
                    }

                    outputs.extend(window.output(&self.pinnacle));

                    let loc = match loc {
                        Location::MapTo(loc) => loc,
                        Location::FloatingResize { edges, initial_geo } => {
                            let mut loc = initial_geo.loc;

                            if let xdg_toplevel::ResizeEdge::Left
                            | xdg_toplevel::ResizeEdge::TopLeft
                            | xdg_toplevel::ResizeEdge::BottomLeft = edges.0
                            {
                                loc.x += initial_geo.size.w - window.geometry().size.w;
                            }
                            if let xdg_toplevel::ResizeEdge::Top
                            | xdg_toplevel::ResizeEdge::TopRight
                            | xdg_toplevel::ResizeEdge::TopLeft = edges.0
                            {
                                loc.y += initial_geo.size.h - window.geometry().size.h;
                            }

                            window.with_state_mut(|s| s.set_floating_loc(loc));
                            loc
                        }
                    };

                    locs.insert(window, loc);
                }
            }

            for (window, mut loc) in locs {
                if let Some(surface) = window.x11_surface() {
                    // FIXME: Don't do this here
                    // `loc` includes bounds but we need to configure the x11 surface
                    // with its actual location
                    if !window.should_not_have_ssd() {
                        let deco_offset =
                            window.with_state(|state| state.total_decoration_offset());
                        loc += deco_offset;
                    }
                    let _ = surface.configure(Rectangle::new(loc, surface.geometry().size));
                }

                // if the window moved out of an output, we want to get it first.
                outputs.extend(self.pinnacle.space.outputs_for_element(&window));

                self.pinnacle.space.map_element(window.clone(), loc, false);
                outputs.extend(self.pinnacle.space.outputs_for_element(&window));
            }
        }

        for output in outputs {
            self.schedule_render(&output);
        }

        let mut wins_to_update = Vec::new();

        // Update and map unmapped non-tiled windows
        // Probably a better way to do this
        for win in self.pinnacle.windows.iter() {
            let is_tiled = win.with_state(|state| state.layout_mode.is_tiled());
            let is_on_active_tag = win.is_on_active_tag();
            if !is_tiled && is_on_active_tag && !self.pinnacle.space.elements().any(|w| w == win) {
                wins_to_update.push(win.clone());
            }
        }

        for win in wins_to_update {
            self.pinnacle.update_window_geometry(&win, false);
        }
    }
}

impl Pinnacle {
    pub fn request_layout(&mut self, output: &Output) {
        if output.with_state(|state| state.enabled_global_id.is_none()) {
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
            .filter(|win| {
                win.with_state(|state| {
                    state.layout_mode.is_tiled() || state.layout_mode.is_spilled()
                })
            })
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

        let tree_entry = self
            .pinnacle
            .layout_state
            .layout_trees
            .entry(output.downgrade())
            .or_default()
            .entry(tree_id);
        match tree_entry {
            Entry::Occupied(occupied_entry) => {
                let tree = occupied_entry.into_mut();
                tree.diff(root_node);
                tree
            }
            Entry::Vacant(vacant_entry) => vacant_entry.insert(LayoutTree::new(root_node)),
        };

        *self
            .pinnacle
            .layout_state
            .current_layout_tree_ids
            .entry(output.downgrade())
            .or_default() = tree_id;

        self.pinnacle
            .update_windows_from_tree(&output, &mut self.backend, false);

        self.schedule_render(&output);

        Ok(())
    }

    /// Resizes the tile corresponding to the given tiled window to the new size.
    ///
    /// If the window is not tiled, does nothing.
    ///
    /// Will resize in the provided directions.
    pub fn resize_tile(
        &mut self,
        window: &WindowElement,
        new_size: Size<i32, Logical>,
        resize_x_dir: ResizeDir,
        resize_y_dir: ResizeDir,
    ) {
        if window.with_state(|state| !state.layout_mode.is_tiled()) {
            return;
        }

        if !window.is_on_active_tag() {
            return;
        }

        let Some(output) = window.output(&self.pinnacle) else {
            return;
        };

        let Some(node) = window.with_state(|state| state.layout_node) else {
            return;
        };

        let Some(tree) = self.pinnacle.layout_state.current_tree_for_output(&output) else {
            warn!("No layout tree for output");
            return;
        };

        tree.resize_tile(node, new_size, resize_x_dir, resize_y_dir);

        self.pinnacle
            .update_windows_from_tree(&output, &mut self.backend, true);
    }
}
