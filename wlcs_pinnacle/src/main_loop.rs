use std::sync::{atomic::Ordering, Arc};

use pinnacle::{
    backend::dummy::setup_dummy,
    state::{ClientState, State},
};
use smithay::reexports::calloop::channel::{Channel, Event};
use tracing::warn;

use crate::WlcsEvent;

pub(crate) fn run(channel: Channel<WlcsEvent>) {
    let (mut pinnacle_state, mut event_loop) =
        setup_dummy(true, None).expect("failed to setup dummy backend");

    event_loop
        .handle()
        .insert_source(channel, move |event, &mut (), data| match event {
            Event::Msg(msg) => handle_event(msg, data),
            Event::Closed => handle_event(WlcsEvent::Stop, data),
        })
        .expect("failed to add wlcs event handler");

    event_loop
        .run(None, &mut pinnacle_state, |state| {
            state.update_pointer_focus();
            state.fixup_z_layering();
            state.space.refresh();
            state.popup_manager.cleanup();

            state
                .display_handle
                .flush_clients()
                .expect("failed to flush client buffers");
        })
        .expect("failed to run event_loop");
}

fn handle_event(event: WlcsEvent, state: &mut State) {
    match event {
        WlcsEvent::Stop => state.loop_signal.stop(),
        WlcsEvent::NewClient { stream, client_id } => {
            let client: smithay::reexports::wayland_server::Client = state
                .display_handle
                .insert_client(stream, Arc::new(ClientState::default()))
                .expect("failed to insert new client");
            state.backend.wlcs_mut().clients.insert(client_id, client);
        }
        WlcsEvent::PositionWindow {
            client_id,
            surface_id,
            location,
        } => warn!("PositionWindow"),
        WlcsEvent::PointerMoveAbsolute {
            device_id,
            location,
        } => warn!("PointerMoveAbsolute"),
        WlcsEvent::PointerMoveRelative {
            device_id,
            location,
        } => warn!("PointerMoveRelative"),
        WlcsEvent::PointerButtonUp {
            device_id,
            button_id,
        } => warn!("PointerButtonUp"),
        WlcsEvent::PointerButtonDown {
            device_id,
            button_id,
        } => warn!("PointerButtonDown"),
        WlcsEvent::TouchDown {
            device_id,
            location,
        } => warn!("TouchDown"),
        WlcsEvent::TouchMove {
            device_id,
            location,
        } => warn!("TouchMove"),
        WlcsEvent::TouchUp { device_id } => warn!("TouchUp"),
    }
}
