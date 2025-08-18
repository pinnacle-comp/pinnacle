// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::RefCell;

use indexmap::IndexSet;
use smithay::{
    backend::renderer::damage::OutputDamageTracker,
    desktop::layer_map_for_output,
    output::{Mode, Output, Scale},
    reexports::{drm, wayland_server::backend::GlobalId},
    utils::{Logical, Point, Size, Transform},
    wayland::session_lock::LockSurface,
};
use tracing::debug;

use crate::{
    api::signal::Signal,
    backend::BackendData,
    config::ConnectorSavedState,
    protocol::screencopy::Screencopy,
    state::{Pinnacle, State, WithState},
    tag::Tag,
    util::centered_loc,
};

/// A unique identifier for an output.
///
/// An empty string represents an invalid output.
// TODO: maybe encode that in the type
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct OutputName(pub String);

impl OutputName {
    /// Get the output with this name.
    pub fn output(&self, pinnacle: &Pinnacle) -> Option<Output> {
        let _span = tracy_client::span!("OutputName::output");

        pinnacle
            .outputs
            .iter()
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

    pub enabled_global_id: Option<GlobalId>,

    pub screencopies: Vec<Screencopy>,
    // This monitor's edid serial. "Unknown" if it doesn't have one.
    pub serial: String,
    pub modes: Vec<Mode>,
    pub lock_surface: Option<LockSurface>,
    pub blanking_state: BlankingState,
    /// Whether the monitor is powered.
    ///
    /// Unpowered monitors aren't drawn to but their tags and windows
    /// still exist and can be interacted with.
    pub powered: bool,
    /// Damage tracker for debugging damage.
    pub debug_damage_tracker: OutputDamageTracker,
}

impl Default for OutputState {
    fn default() -> Self {
        Self {
            tags: Default::default(),
            enabled_global_id: Default::default(),
            screencopies: Default::default(),
            serial: Default::default(),
            modes: Default::default(),
            lock_surface: Default::default(),
            blanking_state: Default::default(),
            powered: true,
            debug_damage_tracker: OutputDamageTracker::new(
                Size::default(),
                1.0,
                Default::default(),
            ),
        }
    }
}

impl WithState for Output {
    type State = OutputState;

    fn with_state<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&Self::State) -> T,
    {
        let _span = tracy_client::span!("Output: WithState::with_state");

        let state = self
            .user_data()
            .get_or_insert(RefCell::<Self::State>::default);

        func(&state.borrow())
    }

    fn with_state_mut<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&mut Self::State) -> T,
    {
        let _span = tracy_client::span!("Output: WithState::with_state_mut");

        let state = self
            .user_data()
            .get_or_insert(RefCell::<Self::State>::default);

        func(&mut state.borrow_mut())
    }
}

impl OutputState {
    pub fn focused_tags(&self) -> impl Iterator<Item = &Tag> {
        let _span = tracy_client::span!("OutputState::focused_tags");

        self.tags.iter().filter(|tag| tag.active())
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
        let _span = tracy_client::span!("Pinnacle::change_output_state");

        // Calculate the ratio that the pointer location was over the output's size
        // so we can warp it if the output moves
        let pointer_loc_ratio = self.seat.get_pointer().and_then(|ptr| {
            let current_loc = ptr.current_location();
            let output_geo = self.space.output_geometry(output)?;
            if !output_geo.to_f64().contains(current_loc) {
                return None;
            }
            let relative_loc = current_loc - output_geo.loc.to_f64();
            Some((
                relative_loc.x / output_geo.size.w as f64,
                relative_loc.y / output_geo.size.h as f64,
            ))
        });

        let old_scale = output.current_scale().fractional_scale();

        output.change_current_state(None, transform, scale, location);

        if let Some(location) = location {
            self.space.map_output(output, location);
            self.signal_state.output_move.signal(output);
        }

        if let Some(mode) = mode {
            backend.set_output_mode(output, mode);
        }

        let new_output_geo = self.space.output_geometry(output);

        if let (Some(pointer_loc_ratio), Some(new_output_geo)) = (pointer_loc_ratio, new_output_geo)
        {
            // Warp the pointer if the output moved and the pointer was inside it.
            // This is necessary on startup when outputs are moved around to make
            // sure the initial focused output is the one the pointer is on.

            let new_pointer_x =
                new_output_geo.loc.x as f64 + new_output_geo.size.w as f64 * pointer_loc_ratio.0;
            let new_pointer_y =
                new_output_geo.loc.y as f64 + new_output_geo.size.h as f64 * pointer_loc_ratio.1;
            let new_pointer_loc = Point::from((new_pointer_x, new_pointer_y));
            // Because of the horrible way I've structured everything
            // we don't have the `State` here, so into an idle this goes
            self.loop_handle.insert_idle(move |state| {
                state.warp_cursor_to_global_loc(new_pointer_loc);
            });
        }

        if mode.is_some() || transform.is_some() || scale.is_some() {
            layer_map_for_output(output).arrange();
            if let Some(output_geo) = new_output_geo {
                self.signal_state.output_resize.signal((
                    output,
                    output_geo.size.w.try_into().unwrap_or_default(),
                    output_geo.size.h.try_into().unwrap_or_default(),
                ));
            }
        }

        if let Some(scale) = scale {
            // Move floating windows so they stay in the same place after a scale change

            let pos_multiplier = old_scale / scale.fractional_scale();

            let output_loc = output.current_location();

            for win in self
                .windows
                .iter()
                .filter(|win| win.output(self).as_ref() == Some(output))
                .filter(|win| win.with_state(|state| state.layout_mode.is_floating()))
                .cloned()
                .collect::<Vec<_>>()
            {
                let old_floating_loc = win.with_state(|state| state.floating_loc());

                let loc = self
                    .space
                    .element_location(&win)
                    .or(old_floating_loc)
                    .map(|loc| {
                        let rescaled_loc = (loc - output_loc)
                            .to_f64()
                            .upscale(pos_multiplier)
                            .to_i32_round()
                            + output_loc;
                        rescaled_loc
                    })
                    .or_else(|| new_output_geo.map(|geo| centered_loc(geo, win.geometry().size)));

                if let Some(loc) = loc {
                    self.map_window_to(&win, loc);
                    win.with_state_mut(|state| state.set_floating_loc(loc));
                }
            }

            // FIXME: why is this in an idle
            self.loop_handle.insert_idle(|state| {
                state.pinnacle.update_xwayland_scale();
            });
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

            output.with_state_mut(|state| {
                if state.enabled_global_id.is_none() {
                    state.enabled_global_id =
                        Some(output.create_global::<State>(&self.display_handle));
                    should_signal = true;
                }
            });

            self.space.map_output(output, output.current_location());

            // Trigger the connect signal here for configs to reposition outputs
            //
            // TODO: Create a new output_disable/enable signal and trigger it here
            // instead of connect and disconnect
            if should_signal {
                self.signal_state.output_connect.signal(output);
            }
        } else {
            if let Some(global) = output.with_state_mut(|state| state.enabled_global_id.take()) {
                self.display_handle.remove_global::<State>(global);
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
        let _span = tracy_client::span!("Pinnacle::remove_output");

        debug!("Removing output {}", output.name());

        if let Some(global) = output.with_state_mut(|state| state.enabled_global_id.take()) {
            self.display_handle.remove_global::<State>(global);
        }

        self.outputs.retain(|op| op != output);

        for layer in layer_map_for_output(output).layers() {
            layer.layer_surface().send_close();
        }

        self.space.unmap_output(output);

        self.output_focus_stack.remove(output);
        if let Some(new_focused_output) = self.output_focus_stack.current_focus() {
            self.signal_state.output_focused.signal(new_focused_output);
        }

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

        self.layout_state.remove_output(output);
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
