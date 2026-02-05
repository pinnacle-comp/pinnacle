use smithay_client_toolkit::reexports::client::Dispatch;
use snowcap_protocols::snowcap_decoration_v1::client::{
    snowcap_decoration_manager_v1::SnowcapDecorationManagerV1,
    snowcap_decoration_surface_v1::{self, SnowcapDecorationSurfaceV1},
};

use crate::state::State;

impl Dispatch<SnowcapDecorationManagerV1, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &SnowcapDecorationManagerV1,
        _event: <SnowcapDecorationManagerV1 as smithay_client_toolkit::reexports::client::Proxy>::Event,
        _data: &(),
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qhandle: &smithay_client_toolkit::reexports::client::QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<SnowcapDecorationSurfaceV1, ()> for State {
    fn event(
        state: &mut Self,
        surface: &SnowcapDecorationSurfaceV1,
        event: <SnowcapDecorationSurfaceV1 as smithay_client_toolkit::reexports::client::Proxy>::Event,
        _data: &(),
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qhandle: &smithay_client_toolkit::reexports::client::QueueHandle<Self>,
    ) {
        match event {
            snowcap_decoration_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                surface.ack_configure(serial);

                let Some(deco) = state
                    .decorations
                    .iter_mut()
                    .find(|deco| &deco.decoration == surface)
                else {
                    return;
                };

                deco.toplevel_size_changed(iced::Size::new(width, height));
                deco.initial_configure_received = true;
                deco.schedule_redraw();
            }
            snowcap_decoration_surface_v1::Event::Closed => {
                if let Some(deco_id) = state.decorations.iter().find_map(|d| {
                    if &d.decoration == surface {
                        Some(d.decoration_id)
                    } else {
                        None
                    }
                }) {
                    state.decoration_destroy(deco_id);
                }
            }
            _ => todo!(),
        }
    }
}
