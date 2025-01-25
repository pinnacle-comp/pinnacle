// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::RefCell;

use indexmap::IndexSet;
use smithay::{
    desktop::layer_map_for_output,
    output::{Mode, Output, Scale},
    reexports::{calloop::LoopHandle, drm},
    utils::{Logical, Point, Transform},
    wayland::session_lock::LockSurface,
};
use tracing::debug;

use crate::{
    api::signal::Signal,
    backend::BackendData,
    config::ConnectorSavedState,
    focus::WindowKeyboardFocusStack,
    layout::transaction::{LayoutTransaction, SnapshotTarget},
    protocol::screencopy::Screencopy,
    render::util::snapshot::OutputSnapshots,
    state::{Pinnacle, State, WithState},
    tag::Tag,
};

/// A unique identifier for an output.
///
/// An empty string represents an invalid output.
// TODO: maybe encode that in the type
#[derive(Debug, Hash, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct OutputName(pub String);

impl OutputName {
    /// Get the output with this name.
    pub fn output(&self, pinnacle: &Pinnacle) -> Option<Output> {
        pinnacle
            .outputs
            .keys()
            .find(|output| output.name() == self.0)
            .cloned()
    }
}

/// State of an output's blanking status for session lock.
#[derive(Debug, Default, Copy, Clone)]
pub enum BlankingState {
    /// The output is not blanked and is displaying normal content.
    #[default]
    NotBlanked,
    /// A blank frame has been queued up.
    Blanking,
    /// A blank frame has been displayed.
    Blanked,
}

/// The state of an output
#[derive(Debug)]
pub struct OutputState {
    /// The tags on this output.
    pub tags: IndexSet<Tag>,

    pub focus_stack: WindowKeyboardFocusStack,
    pub screencopy: Option<Screencopy>,
    // This monitor's edid serial. "Unknown" if it doesn't have one.
    pub serial: String,
    pub modes: Vec<Mode>,
    pub lock_surface: Option<LockSurface>,
    pub blanking_state: BlankingState,
    /// A pending layout transaction.
    pub layout_transaction: Option<LayoutTransaction>,
    /// Whether the monitor is powered.
    ///
    /// Unpowered monitors aren't drawn to but their tags and windows
    /// still exist and can be interacted with.
    pub powered: bool,
    pub snapshots: OutputSnapshots,
}

impl Default for OutputState {
    fn default() -> Self {
        Self {
            tags: Default::default(),
            focus_stack: Default::default(),
            screencopy: Default::default(),
            serial: Default::default(),
            modes: Default::default(),
            lock_surface: Default::default(),
            blanking_state: Default::default(),
            layout_transaction: Default::default(),
            powered: true,
            snapshots: OutputSnapshots::default(),
        }
    }
}

impl WithState for Output {
    type State = OutputState;

    fn with_state<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&Self::State) -> T,
    {
        let state = self
            .user_data()
            .get_or_insert(RefCell::<Self::State>::default);

        func(&state.borrow())
    }

    fn with_state_mut<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&mut Self::State) -> T,
    {
        let state = self
            .user_data()
            .get_or_insert(RefCell::<Self::State>::default);

        func(&mut state.borrow_mut())
    }
}

impl OutputState {
    pub fn focused_tags(&self) -> impl Iterator<Item = &Tag> {
        self.tags.iter().filter(|tag| tag.active())
    }

    fn new_wait_layout_transaction(
        &mut self,
        loop_handle: LoopHandle<'static, State>,
        fullscreen_and_up_snapshots: impl IntoIterator<Item = SnapshotTarget>,
        under_fullscreen_snapshots: impl IntoIterator<Item = SnapshotTarget>,
    ) {
        if let Some(ts) = self.layout_transaction.as_mut() {
            ts.wait();
        } else {
            self.layout_transaction = Some(LayoutTransaction::new_and_wait(
                loop_handle,
                fullscreen_and_up_snapshots,
                under_fullscreen_snapshots,
            ));
        }
    }

    /// Add tags to this output, replacing defunct ones first.
    pub fn add_tags(&mut self, tags: impl IntoIterator<Item = Tag>) {
        let defunct_tags = self.tags.iter().skip_while(|tag| !tag.defunct());
        let mut new_tags = tags.into_iter();

        for defunct_tag in defunct_tags {
            let Some(new_tag) = new_tags.next() else {
                return;
            };
            defunct_tag.replace(new_tag);
        }

        self.tags.extend(new_tags);
    }
}

impl Pinnacle {
    pub fn begin_layout_transaction(&self, output: &Output) {
        output.with_state_mut(|state| {
            let (fullscreen_and_up, under) = (
                std::mem::take(&mut state.snapshots.under_fullscreen),
                std::mem::take(&mut state.snapshots.fullscreen_and_above),
            );
            state.new_wait_layout_transaction(self.loop_handle.clone(), fullscreen_and_up, under);
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum OutputMode {
    Smithay(Mode),
    Drm(drm::control::Mode),
}

impl From<OutputMode> for Mode {
    fn from(value: OutputMode) -> Self {
        match value {
            OutputMode::Smithay(mode) => mode,
            OutputMode::Drm(mode) => Mode::from(mode),
        }
    }
}

impl Pinnacle {
    pub fn change_output_state(
        &mut self,
        backend: &mut impl BackendData,
        output: &Output,
        mode: Option<OutputMode>,
        transform: Option<Transform>,
        scale: Option<Scale>,
        location: Option<Point<i32, Logical>>,
    ) {
        let old_scale = output.current_scale().fractional_scale();

        output.change_current_state(None, transform, scale, location);
        if let Some(location) = location {
            self.space.map_output(output, location);
            self.signal_state.output_move.signal(output);
        }

        if let Some(mode) = mode {
            backend.set_output_mode(output, mode);
        }

        if mode.is_some() || transform.is_some() || scale.is_some() {
            layer_map_for_output(output).arrange();
            if let Some(geo) = self.space.output_geometry(output) {
                self.signal_state.output_resize.signal((
                    output,
                    geo.size.w.try_into().unwrap_or_default(),
                    geo.size.h.try_into().unwrap_or_default(),
                ));
            }
        }

        if let Some(scale) = scale {
            let pos_multiplier = old_scale / scale.fractional_scale();

            for win in self
                .windows
                .iter()
                .filter(|win| win.output(self).as_ref() == Some(output))
                .filter(|win| win.with_state(|state| state.window_state.is_floating()))
                .cloned()
                .collect::<Vec<_>>()
            {
                let Some(output) = win.output(self) else { unreachable!() };

                let output_loc = output.current_location();

                let mut loc = self.space.element_location(&win).unwrap_or(output_loc);

                // FIXME: space maps in i32
                let mut loc_relative_to_output = loc - output_loc;
                loc_relative_to_output = loc_relative_to_output
                    .to_f64()
                    .upscale(pos_multiplier)
                    .to_i32_round();

                loc = loc_relative_to_output + output_loc;
                self.space.map_element(win.clone(), loc, false);
            }
        }

        if let Some(lock_surface) = output.with_state(|state| state.lock_surface.clone()) {
            lock_surface.with_pending_state(|state| {
                let Some(new_geo) = self.space.output_geometry(output) else {
                    return;
                };
                state.size = Some((new_geo.size.w as u32, new_geo.size.h as u32).into());
            });

            lock_surface.send_configure();
        }
    }

    pub fn set_output_enabled(&mut self, output: &Output, enabled: bool) {
        if enabled {
            let mut should_signal = false;

            match self.outputs.entry(output.clone()) {
                indexmap::map::Entry::Occupied(entry) => {
                    let global = entry.into_mut();
                    if global.is_none() {
                        *global = Some(output.create_global::<State>(&self.display_handle));
                        should_signal = true;
                    }
                }
                indexmap::map::Entry::Vacant(entry) => {
                    let global = output.create_global::<State>(&self.display_handle);
                    entry.insert(Some(global));
                    should_signal = true;
                }
            }
            self.space.map_output(output, output.current_location());

            // Trigger the connect signal here for configs to reposition outputs
            //
            // TODO: Create a new output_disable/enable signal and trigger it here
            // instead of connect and disconnect
            if should_signal {
                self.signal_state.output_connect.signal(output);
            }
        } else {
            let global = self.outputs.get_mut(output);
            if let Some(global) = global {
                if let Some(global) = global.take() {
                    self.display_handle.remove_global::<State>(global);
                }
            }
            self.space.unmap_output(output);

            // Trigger the disconnect signal here for configs to reposition outputs
            //
            // TODO: Create a new output_disable/enable signal and trigger it here
            // instead of connect and disconnect
            self.signal_state.output_disconnect.signal(output);

            self.gamma_control_manager_state.output_removed(output);

            self.config.connector_saved_states.insert(
                OutputName(output.name()),
                ConnectorSavedState {
                    loc: output.current_location(),
                    tags: output.with_state(|state| state.tags.clone()),
                    scale: Some(output.current_scale()),
                },
            );

            for layer in layer_map_for_output(output).layers() {
                layer.layer_surface().send_close();
            }
        }
    }

    /// Completely remove an output, for example when a monitor is unplugged
    pub fn remove_output(&mut self, output: &Output) {
        debug!("Removing output {}", output.name());

        let global = self.outputs.shift_remove(output);
        if let Some(mut global) = global {
            if let Some(global) = global.take() {
                self.display_handle.remove_global::<State>(global);
            }
        }

        for layer in layer_map_for_output(output).layers() {
            layer.layer_surface().send_close();
        }

        self.space.unmap_output(output);

        self.output_focus_stack.remove(output);

        self.gamma_control_manager_state.output_removed(output);

        self.output_power_management_state.output_removed(output);

        self.output_management_manager_state.remove_head(output);
        self.output_management_manager_state.update::<State>();

        self.signal_state.output_disconnect.signal(output);

        self.config.connector_saved_states.insert(
            OutputName(output.name()),
            ConnectorSavedState {
                loc: output.current_location(),
                tags: output.with_state(|state| state.tags.clone()),
                scale: Some(output.current_scale()),
            },
        );
    }
}

/// Attempts to retrieve a known mode for the given output with the provided width and height.
///
/// If no refresh rate is provided, this tries to pick the one with the highest refresh rate.
pub fn try_pick_mode(
    output: &Output,
    width: u32,
    height: u32,
    refresh_rate_mhz: Option<u32>,
) -> Option<smithay::output::Mode> {
    output.with_state(|state| {
        state
            .modes
            .iter()
            .filter(|mode| mode.size.w == width as i32 && mode.size.h == height as i32)
            .filter(|mode| refresh_rate_mhz.is_none_or(|refresh| refresh as i32 == mode.refresh))
            .max_by_key(|mode| mode.refresh)
            .copied()
    })
}
