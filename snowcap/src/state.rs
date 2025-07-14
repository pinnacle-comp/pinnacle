use anyhow::Context;
use iced::keyboard::key::{NativeCode, Physical};
use iced_futures::Runtime;
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    reexports::{
        calloop::{self, LoopHandle, LoopSignal},
        calloop_wayland_source::WaylandSource,
        client::{
            Connection, QueueHandle,
            globals::registry_queue_init,
            protocol::{wl_keyboard::WlKeyboard, wl_pointer::WlPointer},
        },
        protocols::wp::{
            fractional_scale::v1::client::wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
            viewporter::client::wp_viewporter::WpViewporter,
        },
    },
    registry::RegistryState,
    seat::{SeatState, keyboard::Modifiers},
    shell::wlr_layer::LayerShell,
};
use xkbcommon::xkb::Keysym;

use crate::{
    handlers::keyboard::KeyboardFocus,
    layer::{LayerIdCounter, SnowcapLayer},
    runtime::{CalloopSenderSink, CurrentTokioExecutor},
    server::GrpcServerState,
    widget::SnowcapMessage,
};

pub struct State {
    pub loop_handle: LoopHandle<'static, State>,
    pub loop_signal: LoopSignal,
    pub conn: Connection,

    pub runtime: crate::runtime::Runtime,
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub layer_shell_state: LayerShell,
    pub fractional_scale_manager: WpFractionalScaleManagerV1,
    pub viewporter: WpViewporter,

    pub grpc_server_state: Option<GrpcServerState>,

    pub queue_handle: QueueHandle<State>,

    pub compositor: Option<crate::compositor::Compositor>,

    pub layers: Vec<SnowcapLayer>,

    // TODO: per wl_keyboard
    pub keyboard_focus: Option<KeyboardFocus>,
    pub keyboard_modifiers: Modifiers,
    pub keyboard: Option<WlKeyboard>, // TODO: multiple

    pub pointer: Option<WlPointer>, // TODO: multiple

    pub layer_id_counter: LayerIdCounter,
}

impl State {
    pub fn new(
        loop_handle: LoopHandle<'static, State>,
        loop_signal: LoopSignal,
    ) -> anyhow::Result<Self> {
        let conn =
            Connection::connect_to_env().context("failed to establish wayland connection")?;

        let (globals, event_queue) =
            registry_queue_init::<State>(&conn).context("failed to init registry queue")?;
        let queue_handle = event_queue.handle();

        let layer_shell_state = LayerShell::bind(&globals, &queue_handle).unwrap();
        let seat_state = SeatState::new(&globals, &queue_handle);
        let registry_state = RegistryState::new(&globals);
        let output_state = OutputState::new(&globals, &queue_handle);
        let compositor_state = CompositorState::bind(&globals, &queue_handle).unwrap();
        let fractional_scale_manager: WpFractionalScaleManagerV1 =
            globals.bind(&queue_handle, 1..=1, ()).unwrap();
        let viewporter: WpViewporter = globals.bind(&queue_handle, 1..=1, ()).unwrap();

        WaylandSource::new(conn.clone(), event_queue)
            .insert(loop_handle.clone())
            .unwrap();

        // Attempt to create a wgpu renderer upfront; this takes a non-trivial amount of time to do
        let compositor = crate::wgpu::Compositor::new()
            .ok()
            .map(crate::compositor::Compositor::Primary);

        let (sender, recv) = calloop::channel::channel::<(iced::window::Id, SnowcapMessage)>();
        let mut runtime = Runtime::new(CurrentTokioExecutor, CalloopSenderSink::new(sender));

        loop_handle
            .insert_source(recv, move |event, _, state| match event {
                calloop::channel::Event::Msg((id, msg)) => {
                    let Some(layer) = state.layers.iter().find(|layer| layer.window_id == id)
                    else {
                        return;
                    };

                    match msg {
                        SnowcapMessage::Noop => (),
                        SnowcapMessage::Close => (),
                        SnowcapMessage::KeyboardKey(key) => {
                            if let Some(sender) = layer.keyboard_key_sender.as_ref() {
                                let _ = sender.send(key);
                            }
                        }
                        SnowcapMessage::WidgetEvent(..) => (),
                    }
                }
                calloop::channel::Event::Closed => (),
            })
            .unwrap();

        runtime.track(iced_futures::subscription::into_recipes(
            iced::event::listen_with(|event, status, id| {
                if status == iced::event::Status::Captured {
                    return None;
                }

                match event {
                    iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                        modifiers,
                        physical_key: Physical::Unidentified(NativeCode::Xkb(raw)),
                        ..
                    }) => Some((
                        id,
                        SnowcapMessage::KeyboardKey(crate::handlers::keyboard::KeyboardKey {
                            key: Keysym::new(raw),
                            modifiers: Modifiers {
                                ctrl: modifiers.control(),
                                alt: modifiers.alt(),
                                shift: modifiers.shift(),
                                caps_lock: false,
                                logo: modifiers.logo(),
                                num_lock: false,
                            },
                            pressed: true,
                        }),
                    )),
                    iced::Event::Keyboard(iced::keyboard::Event::KeyReleased {
                        modifiers,
                        physical_key: Physical::Unidentified(NativeCode::Xkb(raw)),
                        ..
                    }) => Some((
                        id,
                        SnowcapMessage::KeyboardKey(crate::handlers::keyboard::KeyboardKey {
                            key: Keysym::new(raw),
                            modifiers: Modifiers {
                                ctrl: modifiers.control(),
                                alt: modifiers.alt(),
                                shift: modifiers.shift(),
                                caps_lock: false,
                                logo: modifiers.logo(),
                                num_lock: false,
                            },
                            pressed: false,
                        }),
                    )),
                    _ => None,
                }
            }),
        ));

        let state = State {
            loop_handle,
            loop_signal,
            conn: conn.clone(),
            runtime,

            registry_state,
            seat_state,
            output_state,
            compositor_state,
            layer_shell_state,
            fractional_scale_manager,
            viewporter,

            grpc_server_state: None,
            queue_handle,
            compositor,
            layers: Vec::new(),
            keyboard_focus: None,
            keyboard_modifiers: smithay_client_toolkit::seat::keyboard::Modifiers::default(),
            keyboard: None,
            pointer: None,
            layer_id_counter: LayerIdCounter::default(),
        };

        Ok(state)
    }
}
