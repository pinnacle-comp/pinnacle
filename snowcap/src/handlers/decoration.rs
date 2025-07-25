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
        // TODO:
    }
}

impl Dispatch<SnowcapDecorationSurfaceV1, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &SnowcapDecorationSurfaceV1,
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
                proxy.ack_configure(serial);

                let Some(deco) = state
                    .decorations
                    .iter_mut()
                    .find(|deco| &deco.decoration == proxy)
                else {
                    return;
                };

                deco.pending_toplevel_size = Some(iced::Size::new(width, height));

                deco.initial_configure_received = true;
                deco.schedule_redraw();
            }
            _ => unreachable!(),
        }
    }
}
