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

    state.pinnacle.new_output(
        "pinnacle-1",
        "Pinnacle",
        "Dummy Output",
        (0, 0).into(),
        (1920, 1080).into(),
        60000,
        1.0,
        smithay::utils::Transform::Normal,
    );

    TagId::reset();

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

    // Pump the loop to start the config. This stops a race between wlcs opening a window before
    // the config finishes executing.
    for _ in 0..100 {
        event_loop
            .dispatch(Duration::from_millis(1), &mut state)
            .unwrap();
    }

    event_loop
        .handle()
        .insert_source(channel, move |event, &mut (), data| match event {
            Event::Msg(msg) => handle_event(msg, data),
            Event::Closed => handle_event(WlcsEvent::Stop, data),
        })
        .expect("failed to add wlcs event handler");

    event_loop
        .run(None, &mut state, |state| {
            // Send frames.
            //
            // Because nothing is actually rendering it's hard to use `send_frame_callbacks`
            // because the surface doesn't have a primary scanout output, because *that* needs
            // actual rendering to happen. So we just send frames here.
            let output = state.pinnacle.outputs.keys().next().cloned().unwrap();
            for window in state.pinnacle.space.elements_for_output(&output) {
                window.send_frame(
                    &output,
                    state.pinnacle.clock.now(),
                    Some(Duration::ZERO),
                    |_, _| Some(output.clone()),
                );
            }
            for layer in smithay::desktop::layer_map_for_output(&output).layers() {
                layer.send_frame(
                    &output,
                    state.pinnacle.clock.now(),
                    Some(Duration::from_millis(995)),
                    |_, _| Some(output.clone()),
                );
            }

            state.on_event_loop_cycle_completion();
        })
        .expect("failed to run event_loop");

    // Apparently the wayland socket doesn't want to get removed
    let base_dirs = state
        .pinnacle
        .xdg_base_dirs
        .get_runtime_directory()
        .unwrap();

    let _ = std::fs::remove_file(base_dirs.join(std::env::var("WAYLAND_DISPLAY").unwrap()));
    let _ =
        std::fs::remove_file(base_dirs.join(std::env::var("WAYLAND_DISPLAY").unwrap() + ".lock"));
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
                    state.set_floating_loc(location);
                    // state.window_state.set_floating(true);
                });

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
