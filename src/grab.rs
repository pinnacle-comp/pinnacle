// SPDX-License-Identifier: GPL-3.0-or-later

pub mod move_grab;
pub mod resize_grab;

use smithay::{
    input::{
        SeatHandler,
        pointer::{self, PointerHandle},
        touch::{self, TouchHandle},
    },
    reexports::wayland_server::{Resource, protocol::wl_surface::WlSurface},
    utils::{Logical, Point, Serial},
    wayland::seat::WaylandFocus,
};

use crate::state::State;

pub enum InputGrabStartData<D: SeatHandler> {
    Pointer(pointer::GrabStartData<D>),
    Touch(touch::GrabStartData<D>),
}

impl<D: SeatHandler> InputGrabStartData<D> {
    pub fn location(&self) -> Point<f64, Logical> {
        match self {
            Self::Pointer(g) => g.location,
            Self::Touch(g) => g.location,
        }
    }

    pub fn as_pointer(&self) -> Option<&pointer::GrabStartData<D>> {
        match self {
            Self::Pointer(g) => Some(g),
            _ => None,
        }
    }

    pub fn as_touch(&self) -> Option<&touch::GrabStartData<D>> {
        match self {
            Self::Touch(g) => Some(g),
            _ => None,
        }
    }
}

/// Returns the [GrabStartData] from a pointer grab, if any.
pub fn pointer_grab_start_data(
    pointer: &PointerHandle<State>,
    surface: &WlSurface,
    serial: Serial,
) -> Option<pointer::GrabStartData<State>> {
    tracing::debug!("start of pointer_grab_start_data");
    if !pointer.has_grab(serial) {
        tracing::debug!("pointer doesn't have grab");
        return None;
    }

    let start_data = pointer.grab_start_data()?;

    let (focus_surface, _point) = start_data.focus.as_ref()?;

    if !focus_surface.same_client_as(&surface.id()) {
        tracing::debug!("surface isn't the same");
        return None;
    }

    Some(start_data)
}

pub fn touch_grab_start_data(
    touch: &TouchHandle<State>,
    surface: &WlSurface,
    serial: Serial,
) -> Option<touch::GrabStartData<State>> {
    tracing::debug!("start of touch_grab_start_data");
    if !touch.has_grab(serial) {
        tracing::debug!("touch doesn't have grab");
        return None;
    }

    let start_data = touch.grab_start_data()?;

    let (focus_surface, _) = start_data.focus.as_ref()?;

    if !focus_surface.same_client_as(&surface.id()) {
        tracing::debug!("surface isn't the same");
        return None;
    }

    Some(start_data)
}

impl<D: SeatHandler> From<pointer::GrabStartData<D>> for InputGrabStartData<D> {
    fn from(value: pointer::GrabStartData<D>) -> Self {
        Self::Pointer(value)
    }
}

impl<D: SeatHandler> From<touch::GrabStartData<D>> for InputGrabStartData<D> {
    fn from(value: touch::GrabStartData<D>) -> Self {
        Self::Touch(value)
    }
}
