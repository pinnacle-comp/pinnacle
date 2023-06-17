use smithay::{
    input::{
        pointer::{GrabStartData, PointerHandle},
        SeatHandler,
    },
    reexports::wayland_server::{protocol::wl_surface::WlSurface, Resource},
    utils::Serial,
};

/// Returns the [GrabStartData] from a pointer grab, if any.
pub fn pointer_grab_start_data<S>(
    pointer: &PointerHandle<S>,
    surface: &WlSurface,
    serial: Serial,
) -> Option<GrabStartData<S>>
where
    S: SeatHandler<PointerFocus = WlSurface> + 'static,
{
    println!("start of pointer_grab_start_data");
    if !pointer.has_grab(serial) {
        println!("pointer doesn't have grab");
        return None;
    }

    let start_data = pointer.grab_start_data()?;

    let (focus_surface, _point) = start_data.focus.as_ref()?;

    if !focus_surface.id().same_client_as(&surface.id()) {
        println!("surface isn't the same");
        return None;
    }

    Some(start_data)
}
