// SPDX-License-Identifier: GPL-3.0-or-later

use std::{cell::RefCell, num::NonZeroU32};

use pinnacle_api_defs::pinnacle::signal::v0alpha1::{
    OutputDisconnectResponse, OutputMoveResponse, OutputResizeResponse,
};
use smithay::{
    desktop::layer_map_for_output,
    output::{Mode, Output, Scale},
    reexports::{calloop::LoopHandle, drm},
    utils::{Logical, Point, Transform},
    wayland::session_lock::LockSurface,
};

use crate::{
    backend::BackendData,
    config::ConnectorSavedState,
    focus::WindowKeyboardFocusStack,
    layout::transaction::{LayoutTransaction, SnapshotTarget},
    protocol::screencopy::Screencopy,
    state::{Pinnacle, State, WithState},
    tag::Tag,
    window::window_state::FloatingOrTiled,
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
    pub tags: Vec<Tag>,
    pub focus_stack: WindowKeyboardFocusStack,
    pub screencopy: Option<Screencopy>,
    pub serial: Option<NonZeroU32>,
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

    pub fn new_wait_layout_transaction(
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
            self.signal_state.output_move.signal(|buf| {
                buf.push_back(OutputMoveResponse {
                    output_name: Some(output.name()),
                    x: Some(location.x),
                    y: Some(location.y),
                });
            });
        }

        if let Some(mode) = mode {
            backend.set_output_mode(output, mode);
        }

        if mode.is_some() || transform.is_some() || scale.is_some() {
            layer_map_for_output(output).arrange();
            self.signal_state.output_resize.signal(|buf| {
                let geo = self.space.output_geometry(output);
                buf.push_back(OutputResizeResponse {
                    output_name: Some(output.name()),
                    logical_width: geo.map(|geo| geo.size.w as u32),
                    logical_height: geo.map(|geo| geo.size.h as u32),
                });
            });
        }

        if let Some(scale) = scale {
            let pos_multiplier = old_scale / scale.fractional_scale();

            for win in self
                .windows
                .iter()
                .filter(|win| win.output(self).as_ref() == Some(output))
                .filter(|win| win.with_state(|state| state.floating_or_tiled.is_floating()))
                .cloned()
                .collect::<Vec<_>>()
            {
                let Some(output) = win.output(self) else { unreachable!() };

                let output_loc = output.current_location();

                // FIXME: get everything out of this with_state
                win.with_state_mut(|state| {
                    let FloatingOrTiled::Floating(rect) = &mut state.floating_or_tiled else {
                        unreachable!()
                    };

                    let loc = rect.loc;

                    let mut loc_relative_to_output = loc - output_loc;
                    loc_relative_to_output = loc_relative_to_output
                        .to_f64()
                        .upscale(pos_multiplier)
                        .to_i32_round();

                    rect.loc = loc_relative_to_output + output_loc;
                    self.space.map_element(win.clone(), rect.loc, false);
                });
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
            match self.outputs.entry(output.clone()) {
                indexmap::map::Entry::Occupied(entry) => {
                    let global = entry.into_mut();
                    if global.is_none() {
                        *global = Some(output.create_global::<State>(&self.display_handle));
                    }
                }
                indexmap::map::Entry::Vacant(entry) => {
                    let global = output.create_global::<State>(&self.display_handle);
                    entry.insert(Some(global));
                }
            }
            self.space.map_output(output, output.current_location());
        } else {
            let global = self.outputs.get_mut(output);
            if let Some(global) = global {
                if let Some(global) = global.take() {
                    self.display_handle.remove_global::<State>(global);
                }
            }
            self.space.unmap_output(output);

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

        self.gamma_control_manager_state.output_removed(output);

        self.output_power_management_state.output_removed(output);

        self.output_management_manager_state.remove_head(output);
        self.output_management_manager_state.update::<State>();

        self.signal_state.output_disconnect.signal(|buffer| {
            buffer.push_back(OutputDisconnectResponse {
                output_name: Some(output.name()),
            })
        });

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
