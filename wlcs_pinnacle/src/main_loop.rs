use std::{sync::Arc, time::Duration};

use pinnacle::{
    backend::wlcs::setup_wlcs_dummy,
    state::{ClientState, State},
};
use smithay::{
    backend::input::{ButtonState, DeviceCapability, InputEvent},
    reexports::{
        calloop::channel::{Channel, Event},
        wayland_server::{Client, Resource},
    },
    wayland::seat::WaylandFocus,
};

use crate::{
    input_backend::{
        WlcsDevice, WlcsInputBackend, WlcsPointerButtonEvent, WlcsPointerMotionAbsoluteEvent,
        WlcsPointerMotionEvent, WlcsTouchDownEvent, WlcsTouchUpEvent,
    },
    WlcsEvent,
};

pub(crate) fn run(channel: Channel<WlcsEvent>) {
    let config_path =
        &std::env::var("PINNACLE_WLCS_CONFIG_PATH").expect("PINNACLE_WLCS_CONFIG_PATH not set");

    let (mut state, mut event_loop) =
        setup_wlcs_dummy(false, Some(config_path.into())).expect("failed to setup dummy backend");

    event_loop
        .handle()
        .insert_source(channel, move |event, &mut (), data| match event {
            Event::Msg(msg) => handle_event(msg, data),
            Event::Closed => handle_event(WlcsEvent::Stop, data),
        })
        .expect("failed to add wlcs event handler");

    // FIXME: a better way to deal with tokio here?
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    let _handle = rt.enter();

    // FIXME: once starting pinnacle without xwayland is a thing, handle this
    // |      properly; in this case, we probably no longer need to start the
    // |      config manually anymore either, as this is only needed now,
    // |      because the config is started after xwayland reports its ready

    // when xdiplay is None when starting the config, the grpc server is not
    // started, until it is set; this bypasses this for now
    state.xdisplay = Some(u32::MAX);
    if let Err(err) = state.start_config(config_path) {
        panic!("failed to start config: {err}");
    }

    // FIXME: use a custom socker_dir to avoid having to number sockets

    // wait for the config to connect to the layout service
    while state.layout_state.layout_request_sender.is_none() {
        event_loop
            .dispatch(Some(Duration::from_millis(10)), &mut state)
            .expect("event_loop error while waiting for config");
    }

    event_loop
        .run(None, &mut state, |state| {
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
        WlcsEvent::Stop => state.shutdown(),
        WlcsEvent::NewClient { stream, client_id } => {
            let client: Client = state
                .display_handle
                .insert_client(stream, Arc::new(ClientState::default()))
                .expect("failed to insert new client");
            state.backend.wlcs_mut().clients.insert(client_id, client);
        }
        WlcsEvent::PositionWindow {
            client_id,
            surface_id,
            location,
        } => {
            let client = state.backend.wlcs_mut().clients.get(&client_id);
            let toplevel = state.space.elements().find(|w| {
                if let Some(surface) = w.wl_surface() {
                    state.display_handle.get_client(surface.id()).ok().as_ref() == client
                        && surface.id().protocol_id() == surface_id
                } else {
                    false
                }
            });

            if let Some(toplevel) = toplevel {
                state.space.map_element(toplevel.clone(), location, false);
            }
        }

        WlcsEvent::NewPointer { device_id } => {
            state.process_input_event(InputEvent::<WlcsInputBackend>::DeviceAdded {
                device: WlcsDevice {
                    device_id,
                    capability: DeviceCapability::Pointer,
                },
            })
        }
        WlcsEvent::PointerMoveAbsolute {
            device_id,
            position,
        } => state.process_input_event(
            WlcsPointerMotionAbsoluteEvent {
                device_id,
                time: Duration::from(state.clock.now()).as_millis() as u64,
                position,
            }
            .into(),
        ),
        WlcsEvent::PointerMoveRelative { device_id, delta } => state.process_input_event(
            WlcsPointerMotionEvent {
                device_id,
                time: Duration::from(state.clock.now()).as_millis() as u64,
                delta,
            }
            .into(),
        ),
        WlcsEvent::PointerButton {
            device_id,
            button_id,
            pressed,
        } => state.process_input_event(
            WlcsPointerButtonEvent {
                device_id,
                time: Duration::from(state.clock.now()).as_millis() as u64,
                button_code: button_id as u32,
                state: if pressed {
                    ButtonState::Pressed
                } else {
                    ButtonState::Released
                },
            }
            .into(),
        ),
        WlcsEvent::NewTouch { device_id } => {
            state.process_input_event(InputEvent::<WlcsInputBackend>::DeviceAdded {
                device: WlcsDevice {
                    device_id,
                    capability: DeviceCapability::Pointer,
                },
            })
        }
        WlcsEvent::TouchDown {
            device_id,
            position,
        } => state.process_input_event(
            WlcsTouchDownEvent {
                device_id,
                time: Duration::from(state.clock.now()).as_millis() as u64,
                position,
            }
            .into(),
        ),
        WlcsEvent::TouchMove {
            device_id,
            position,
        } => state.process_input_event(
            WlcsTouchDownEvent {
                device_id,
                time: Duration::from(state.clock.now()).as_millis() as u64,
                position,
            }
            .into(),
        ),
        WlcsEvent::TouchUp { device_id } => state.process_input_event(
            WlcsTouchUpEvent {
                device_id,
                time: Duration::from(state.clock.now()).as_millis() as u64,
            }
            .into(),
        ),
    }
}
