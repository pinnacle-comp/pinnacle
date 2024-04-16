use std::{sync::Arc, time::Duration};

use pinnacle::{
    backend::dummy::setup_dummy,
    focus::{keyboard::KeyboardFocusTarget, pointer::PointerFocusTarget},
    state::{ClientState, State},
};
use smithay::{
    backend::input::ButtonState,
    input::pointer::{ButtonEvent, MotionEvent, RelativeMotionEvent},
    reexports::{
        calloop::channel::{Channel, Event},
        wayland_server::Resource,
    },
    utils::SERIAL_COUNTER,
    wayland::seat::WaylandFocus,
};
use tracing::warn;

use crate::WlcsEvent;

pub(crate) fn run(channel: Channel<WlcsEvent>) {
    let config_path =
        &std::env::var("PINNACLE_WLCS_CONFIG_PATH").expect("PINNACLE_WLCS_CONFIG_PATH not set");

    let (mut state, mut event_loop) =
        setup_dummy(false, Some(config_path.into())).expect("failed to setup dummy backend");

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

    if let Err(err) = state.start_config(config_path) {
        panic!("failed to start config: {err}");
    }

    // FIXME: different sock_dir per instance?
    while state.layout_state.layout_request_sender.is_none() {
        event_loop
            .dispatch(Some(Duration::from_millis(10)), &mut state)
            .expect("event_loop error while waiting for config");
    }

    // TODO: handle no-xwayland properly

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
        WlcsEvent::PointerMove {
            device_id: _,
            position,
            absolute,
        } => {
            let serial = SERIAL_COUNTER.next_serial();
            let ptr = state.seat.get_pointer().unwrap();
            let ptr_location = ptr.current_location();

            let location = if absolute { position } else { ptr_location + position };

            let under = state
                .space
                .element_under(location)
                .and_then(|(w, _)| w.wl_surface())
                .map(|surf| (PointerFocusTarget::WlSurface(surf), location.to_i32_round()));

            let time = Duration::from(state.clock.now()).as_millis() as u32;
            ptr.motion(
                state,
                under.clone(),
                &MotionEvent {
                    location,
                    serial,
                    time,
                },
            );

            if !absolute {
                let utime = Duration::from(state.clock.now()).as_micros() as u64;
                ptr.relative_motion(
                    state,
                    under,
                    &RelativeMotionEvent {
                        delta: position,
                        delta_unaccel: position,
                        utime,
                    },
                )
            }
            ptr.frame(state);
        }
        WlcsEvent::PointerButton {
            device_id: _,
            button_id,
            pressed,
        } => {
            let serial = SERIAL_COUNTER.next_serial();
            let ptr = state.seat.get_pointer().unwrap();
            if !ptr.is_grabbed() {
                let ptr_location = ptr.current_location();
                let under = state
                    .space
                    .element_under(ptr_location)
                    .map(|(w, _)| w.clone());
                if let Some(win) = &under {
                    state.space.raise_element(win, true);
                }
                state.seat.get_keyboard().unwrap().set_focus(
                    state,
                    under.map(|w| KeyboardFocusTarget::Window(w)),
                    serial,
                );
            }
            let time = Duration::from(state.clock.now()).as_millis() as u32;
            ptr.button(
                state,
                &ButtonEvent {
                    serial,
                    time,
                    button: button_id as u32,
                    state: if pressed {
                        ButtonState::Pressed
                    } else {
                        ButtonState::Released
                    },
                },
            );
            ptr.frame(state);
        }
        WlcsEvent::TouchDown { .. } => warn!("TouchDown"),
        WlcsEvent::TouchMove { .. } => warn!("TouchMove"),
        WlcsEvent::TouchUp { .. } => warn!("TouchUp"),
    }
}
