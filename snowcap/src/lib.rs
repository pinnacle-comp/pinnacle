pub mod api;
pub mod clipboard;
pub mod compositor;
pub mod decoration;
pub mod handlers;
pub mod input;
pub mod layer;
pub mod popup;
pub mod runtime;
pub mod server;
pub mod state;
pub mod surface;
pub mod util;
pub mod wgpu;
pub mod widget;

use iced::mouse::Interaction;
use server::socket_dir;
use smithay_client_toolkit::reexports::{
    calloop::{self, EventLoop},
    client::protocol::wl_pointer::WlPointer,
    protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::WpCursorShapeDeviceV1,
};
use state::State;
use tracing::info;

use crate::handlers::pointer::iced_interaction_to_shape;

/// A handle to the running Snowcap instance.
#[derive(Debug, Clone)]
pub struct SnowcapHandle {
    stop_signal: calloop::ping::Ping,
    close_all_widgets: calloop::ping::Ping,
}

impl SnowcapHandle {
    /// Send the stop signal to Snowcap.
    pub fn stop(&self) {
        self.stop_signal.ping();
    }

    pub fn close_all_widgets(&self) {
        self.close_all_widgets.ping();
    }
}

pub fn start(stop_signal_sender: Option<tokio::sync::oneshot::Sender<SnowcapHandle>>) {
    info!("Snowcap starting up");

    let mut event_loop = EventLoop::<State>::try_new().unwrap();

    let mut state = State::new(event_loop.handle(), event_loop.get_signal()).unwrap();

    state.start_grpc_server(socket_dir()).unwrap();

    if let Some(sender) = stop_signal_sender {
        let (stop_ping, stop_ping_source) = calloop::ping::make_ping().unwrap();
        let loop_signal = event_loop.get_signal();

        event_loop
            .handle()
            .insert_source(stop_ping_source, move |_, _, _| {
                loop_signal.stop();
                loop_signal.wakeup();
            })
            .unwrap();

        let (close_ping, close_ping_source) = calloop::ping::make_ping().unwrap();

        event_loop
            .handle()
            .insert_source(close_ping_source, move |_, _, state| {
                state.layers.clear();
                state.decorations.clear();
                state.popups.clear();
            })
            .unwrap();

        sender
            .send(SnowcapHandle {
                stop_signal: stop_ping,
                close_all_widgets: close_ping,
            })
            .unwrap();
    }

    event_loop
        .run(None, &mut state, |state| {
            let _span = tracy_client::span!("snowcap event loop idle callback");

            let keyboard_focus_is_dead =
                state
                    .keyboard_focus
                    .as_ref()
                    .is_some_and(|focus| match focus {
                        handlers::keyboard::KeyboardFocus::Layer(layer) => {
                            !state.layers.iter().any(|sn_layer| &sn_layer.layer == layer)
                        }
                        handlers::keyboard::KeyboardFocus::Popup(popup) => {
                            !state.popups.iter().any(|p| &p.popup == popup)
                        }
                    });
            if keyboard_focus_is_dead {
                state.keyboard_focus = None;
            }

            state.update_surfaces();
        })
        .unwrap();
}

impl State {
    fn update_surfaces(&mut self) {
        for layer in self.layers.iter_mut() {
            let interaction_changed =
                layer.update(&mut self.runtime, self.compositor.as_mut().unwrap());

            if interaction_changed
                && let Some(pointer_focus) = self.pointer_focus.as_ref()
                && &layer.surface.wl_surface == pointer_focus
                && let Some(pointer) = self.pointer.as_ref()
                && let Some(device) = self.cursor_shape_device.as_ref()
                && let Some(serial) = self.last_pointer_enter_serial
            {
                Self::set_cursor_shape(layer.surface.mouse_interaction, device, serial, pointer);
            }

            layer.draw_if_scheduled();
        }

        for deco in self.decorations.iter_mut() {
            // INFO: Currently forcing tiny-skia on decorations because vulkan
            // uses a lot of vram
            let interaction_changed =
                deco.update(&mut self.runtime, self.tiny_skia.as_mut().unwrap());

            if interaction_changed
                && let Some(pointer_focus) = self.pointer_focus.as_ref()
                && &deco.surface.wl_surface == pointer_focus
                && let Some(pointer) = self.pointer.as_ref()
                && let Some(device) = self.cursor_shape_device.as_ref()
                && let Some(serial) = self.last_pointer_enter_serial
            {
                Self::set_cursor_shape(deco.surface.mouse_interaction, device, serial, pointer);
            }

            deco.draw_if_scheduled();
        }

        for popup in self.popups.iter_mut() {
            let interaction_changed =
                popup.update(&mut self.runtime, self.compositor.as_mut().unwrap());

            if interaction_changed
                && let Some(pointer_focus) = self.pointer_focus.as_ref()
                && &popup.surface.wl_surface == pointer_focus
                && let Some(pointer) = self.pointer.as_ref()
                && let Some(device) = self.cursor_shape_device.as_ref()
                && let Some(serial) = self.last_pointer_enter_serial
            {
                Self::set_cursor_shape(popup.surface.mouse_interaction, device, serial, pointer);
            }

            popup.draw_if_scheduled();
        }
    }

    fn set_cursor_shape(
        interaction: Interaction,
        device: &WpCursorShapeDeviceV1,
        last_enter_serial: u32,
        wl_pointer: &WlPointer,
    ) {
        let shape = iced_interaction_to_shape(interaction);
        match shape {
            Some(shape) => device.set_shape(last_enter_serial, shape),
            None => wl_pointer.set_cursor(last_enter_serial, None, 0, 0),
        }
    }
}
