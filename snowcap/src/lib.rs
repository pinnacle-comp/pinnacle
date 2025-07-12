pub mod api;
pub mod clipboard;
pub mod compositor;
pub mod handlers;
pub mod input;
pub mod layer;
pub mod runtime;
pub mod server;
pub mod state;
pub mod util;
pub mod wgpu;
pub mod widget;

use server::socket_dir;
use smithay_client_toolkit::reexports::calloop::{self, EventLoop};
use state::State;
use tracing::info;

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
            let keyboard_focus_is_dead =
                state
                    .keyboard_focus
                    .as_ref()
                    .is_some_and(|focus| match focus {
                        handlers::keyboard::KeyboardFocus::Layer(layer) => {
                            !state.layers.iter().any(|sn_layer| &sn_layer.layer == layer)
                        }
                    });
            if keyboard_focus_is_dead {
                state.keyboard_focus = None;
            }

            for layer in state.layers.iter_mut() {
                if layer.widgets.has_events_queued() {
                    layer.update(&state.queue_handle, &mut state.runtime);
                }

                if layer.redraw_requested {
                    layer.redraw_requested = false;
                    layer.draw();
                }
            }
        })
        .unwrap();
}
