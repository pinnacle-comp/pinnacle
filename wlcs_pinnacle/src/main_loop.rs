use std::{sync::Arc, time::Duration};

use pinnacle::{
    backend::wlcs::setup_wlcs_dummy,
    state::{ClientState, State, WithState},
    window::window_state::FloatingOrTiled,
};
use smithay::{
    backend::input::{ButtonState, DeviceCapability, InputEvent},
    reexports::{
        calloop::channel::{Channel, Event},
        wayland_server::{Client, Resource},
    },
    utils::Rectangle,
    wayland::seat::WaylandFocus,
};

use crate::{
    config::run_config,
    input_backend::{
        WlcsDevice, WlcsInputBackend, WlcsPointerButtonEvent, WlcsPointerMotionAbsoluteEvent,
        WlcsPointerMotionEvent, WlcsTouchDownEvent, WlcsTouchUpEvent,
    },
    WlcsEvent,
};

pub(crate) fn run(channel: Channel<WlcsEvent>) {
    let (mut state, mut event_loop) = setup_wlcs_dummy().expect("failed to setup dummy backend");

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
    state.pinnacle.xdisplay = Some(u32::MAX);
    run_config(&mut state.pinnacle);

    // wait for the config to connect to the layout service
    while state.pinnacle.layout_state.layout_request_sender.is_none() {
        event_loop
            .dispatch(Some(Duration::from_millis(10)), &mut state)
            .expect("event_loop error while waiting for config");
    }

    event_loop
        .run(None, &mut state, |state| {
            state.update_pointer_focus();
            state.pinnacle.fixup_z_layering();
            state.pinnacle.space.refresh();
            state.pinnacle.popup_manager.cleanup();

            state
                .pinnacle
                .display_handle
                .flush_clients()
                .expect("failed to flush client buffers");
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
                state
                    .pinnacle
                    .space
                    .map_element(window.clone(), location, false);

                let size = state
                    .pinnacle
                    .space
                    .element_geometry(&window)
                    .expect("window to be positioned was not mapped")
                    .size;

                if window.with_state(|state| state.floating_or_tiled.is_tiled()) {
                    window.toggle_floating();
                }

                window.with_state_mut(|state| {
                    state.floating_or_tiled =
                        FloatingOrTiled::Floating(Rectangle::from_loc_and_size(location, size));
                });

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
