use anyhow::Context;
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    reexports::{
        calloop::{LoopHandle, LoopSignal},
        calloop_wayland_source::WaylandSource,
        client::{
            globals::registry_queue_init,
            protocol::{wl_keyboard::WlKeyboard, wl_pointer::WlPointer},
            Connection, QueueHandle,
        },
    },
    registry::RegistryState,
    seat::{keyboard::Modifiers, SeatState},
    shell::wlr_layer::LayerShell,
};

use crate::{
    handlers::keyboard::KeyboardFocus,
    layer::SnowcapLayer,
    server::GrpcServerState,
    wgpu::{setup_wgpu, Wgpu},
    widget::WidgetIdCounter,
};

pub struct State {
    pub loop_handle: LoopHandle<'static, State>,
    pub loop_signal: LoopSignal,
    pub conn: Connection,

    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub layer_shell_state: LayerShell,

    pub grpc_server_state: Option<GrpcServerState>,

    pub queue_handle: QueueHandle<State>,

    pub wgpu: Wgpu,

    pub layers: Vec<SnowcapLayer>,

    // TODO: per wl_keyboard
    pub keyboard_focus: Option<KeyboardFocus>,
    pub keyboard_modifiers: Modifiers,
    pub keyboard: Option<WlKeyboard>, // TODO: multiple

    pub pointer: Option<WlPointer>, // TODO: multiple

    pub widget_id_counter: WidgetIdCounter,
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

        WaylandSource::new(conn.clone(), event_queue)
            .insert(loop_handle.clone())
            .unwrap();

        let state = State {
            loop_handle,
            loop_signal,
            conn: conn.clone(),
            registry_state,
            seat_state,
            output_state,
            compositor_state,
            layer_shell_state,
            grpc_server_state: None,
            queue_handle,
            wgpu: setup_wgpu()?,
            layers: Vec::new(),
            keyboard_focus: None,
            keyboard_modifiers: smithay_client_toolkit::seat::keyboard::Modifiers::default(),
            keyboard: None,
            pointer: None,
            widget_id_counter: WidgetIdCounter::default(),
        };

        Ok(state)
    }
}
