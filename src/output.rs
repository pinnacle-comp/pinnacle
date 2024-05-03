// SPDX-License-Identifier: GPL-3.0-or-later

use std::{cell::RefCell, num::NonZeroU32};

use pinnacle_api_defs::pinnacle::signal::v0alpha1::{OutputMoveResponse, OutputResizeResponse};
use smithay::{
    desktop::layer_map_for_output,
    output::{Mode, Output, Scale},
    utils::{Logical, Point, Transform},
};

use crate::{
    focus::WindowKeyboardFocusStack,
    protocol::screencopy::Screencopy,
    state::{Pinnacle, WithState},
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
            .space
            .outputs()
            .find(|output| output.name() == self.0)
            .cloned()
    }
}

/// The state of an output
#[derive(Default, Debug)]
pub struct OutputState {
    pub tags: Vec<Tag>,
    pub focus_stack: WindowKeyboardFocusStack,
    pub screencopy: Option<Screencopy>,
    pub serial: Option<NonZeroU32>,
    pub modes: Vec<Mode>,
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
}

impl Pinnacle {
    /// A wrapper around [`Output::change_current_state`] that additionally sends an output
    /// geometry signal.
    pub fn change_output_state(
        &mut self,
        output: &Output,
        mode: Option<Mode>,
        transform: Option<Transform>,
        scale: Option<Scale>,
        location: Option<Point<i32, Logical>>,
    ) {
        output.change_current_state(mode, transform, scale, location);
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
        if let Some(mode) = mode {
            output.set_preferred(mode);
            output.with_state_mut(|state| state.modes.push(mode));
        }
    }
}
