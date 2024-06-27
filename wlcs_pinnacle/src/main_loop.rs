use std::{path::PathBuf, sync::Arc, time::Duration};

use pinnacle::{
    state::{ClientState, State, WithState},
    tag::TagId,
};
use smithay::{
    backend::input::{ButtonState, DeviceCapability, InputEvent},
    reexports::{
        calloop::{
            channel::{Channel, Event},
            EventLoop,
        },
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
    let mut event_loop = EventLoop::<State>::try_new().unwrap();
    let mut state = State::new(
        pinnacle::cli::Backend::Dummy,
        event_loop.handle(),
        event_loop.get_signal(),
        PathBuf::from(""),
        None,
    )
    .unwrap();

    TagId::reset();

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

    {
        let temp_dir = tempfile::tempdir().expect("failed to setup temp dir for socket");
        let socket_dir = temp_dir.path().to_owned();

        state.pinnacle.start_grpc_server(&socket_dir).unwrap();

        std::thread::spawn(move || {
            crate::config::start_config();
            drop(temp_dir);
        });
    }

    // wait for the config to connect to the layout service
    //
    // Ottatop: this probably doesn't do a whole lot because
    // everything else is in the event loop so this pretty much
    // just runs everything
    while state.pinnacle.layout_state.layout_request_sender.is_none() {
        event_loop
            .dispatch(Some(Duration::from_millis(10)), &mut state)
            .expect("event_loop error while waiting for config");
    }

    event_loop
        .run(None, &mut state, |state| {
            state.on_event_loop_cycle_completion();
        })
        .expect("failed to run event_loop");
}

fn handle_event(event: WlcsEvent, state: &mut State) {
    tracing::debug!("handle_event {:?}", event);
    match event {
        WlcsEvent::Stop => state.pinnacle.shutdown(),
        WlcsEvent::NewClient { stream, client_id } => {
            let client: Client = state
                .pinnacle
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
            let window = state
                .pinnacle
                .space
                .elements()
                .find(|w| {
                    if let Some(surface) = w.wl_surface() {
                        state
                            .pinnacle
                            .display_handle
                            .get_client(surface.id())
                            .ok()
                            .as_ref()
                            == client
                            && surface.id().protocol_id() == surface_id
                    } else {
                        false
                    }
                })
                .cloned();

            if let Some(window) = window {
                window.with_state_mut(|state| {
                    state.floating_loc = Some(location.to_f64());
                    // state.window_state.set_floating(true);
                });

                // state.pinnacle.set_window_floating(&window, true);

                state
                    .pinnacle
                    .space
                    .map_element(window.clone(), location, false);

                for output in state.pinnacle.space.outputs_for_element(&window) {
                    state.schedule_render(&output);
                }
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
                time: Duration::from(state.pinnacle.clock.now()).as_millis() as u64,
                position,
            }
            .into(),
        ),
        WlcsEvent::PointerMoveRelative { device_id, delta } => state.process_input_event(
            WlcsPointerMotionEvent {
                device_id,
                time: Duration::from(state.pinnacle.clock.now()).as_millis() as u64,
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
                time: Duration::from(state.pinnacle.clock.now()).as_millis() as u64,
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
                time: Duration::from(state.pinnacle.clock.now()).as_millis() as u64,
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
                time: Duration::from(state.pinnacle.clock.now()).as_millis() as u64,
                position,
            }
            .into(),
        ),
        WlcsEvent::TouchUp { device_id } => state.process_input_event(
            WlcsTouchUpEvent {
                device_id,
                time: Duration::from(state.pinnacle.clock.now()).as_millis() as u64,
            }
            .into(),
        ),
    }
}
