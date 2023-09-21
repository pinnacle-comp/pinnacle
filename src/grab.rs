// SPDX-License-Identifier: GPL-3.0-or-later

pub mod move_grab;
pub mod resize_grab;

use smithay::{
    input::{
        pointer::{GrabStartData, PointerHandle},
        SeatHandler,
    },
    reexports::wayland_server::{protocol::wl_surface::WlSurface, Resource},
    utils::Serial,
    wayland::seat::WaylandFocus,
};

use crate::focus::FocusTarget;

/// Returns the [GrabStartData] from a pointer grab, if any.
pub fn pointer_grab_start_data<S>(
    pointer: &PointerHandle<S>,
    surface: &WlSurface,
    serial: Serial,
) -> Option<GrabStartData<S>>
where
    S: SeatHandler<PointerFocus = FocusTarget> + 'static,
{
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
