use anyhow::Context;
use iced::keyboard::key::{NativeCode, Physical};
use iced_futures::Runtime;
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    reexports::{
        calloop::{self, Dispatcher, LoopHandle, LoopSignal},
        calloop_wayland_source::WaylandSource,
        client::{
            Connection, QueueHandle,
            globals::registry_queue_init,
            protocol::{wl_keyboard::WlKeyboard, wl_pointer::WlPointer, wl_surface::WlSurface, wl_seat::WlSeat},
        },
        protocols::{
            ext::foreign_toplevel_list::v1::client::{
                ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
                ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1,
            },
            wp::{
                fractional_scale::v1::client::wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
                viewporter::client::wp_viewporter::WpViewporter,
            },
        },
    },
    registry::RegistryState,
    seat::{SeatState, keyboard::Modifiers},
    shell::{WaylandSurface, wlr_layer::LayerShell},
};
use snowcap_protocols::snowcap_decoration_v1::client::snowcap_decoration_manager_v1::SnowcapDecorationManagerV1;
use xkbcommon::xkb::Keysym;

use crate::{
    decoration::{DecorationIdCounter, SnowcapDecoration},
    handlers::{foreign_toplevel_list::ForeignToplevelListHandleData, keyboard::KeyboardFocus},
    layer::{LayerIdCounter, SnowcapLayer},
    runtime::{CalloopSenderSink, CurrentTokioExecutor},
    server::GrpcServerState,
    surface::{self, CalloopNotifier},
    widget::SnowcapMessage,
};

pub struct State {
    pub loop_handle: LoopHandle<'static, State>,
    pub loop_signal: LoopSignal,
    pub conn: Connection,
    pub wayland_source: Dispatcher<'static, WaylandSource<State>, State>,
    pub shell: iced_graphics::Shell,

    pub runtime: crate::runtime::Runtime,
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub layer_shell_state: LayerShell,
    pub fractional_scale_manager: WpFractionalScaleManagerV1,
    pub viewporter: WpViewporter,
    pub snowcap_decoration_manager: SnowcapDecorationManagerV1,
    pub foreign_toplevel_list: ExtForeignToplevelListV1,

    pub grpc_server_state: Option<GrpcServerState>,

    pub queue_handle: QueueHandle<State>,

    pub compositor: Option<crate::compositor::Compositor>,
    pub tiny_skia: Option<crate::compositor::Compositor>,

    pub layers: Vec<SnowcapLayer>,
    pub decorations: Vec<SnowcapDecoration>,

    pub seat: Option<WlSeat>,
    // TODO: per wl_keyboard
    pub keyboard_focus: Option<KeyboardFocus>,
    pub keyboard_modifiers: Modifiers,
    pub keyboard: Option<WlKeyboard>, // TODO: multiple

    pub pointer: Option<WlPointer>, // TODO: multiple

    pub layer_id_counter: LayerIdCounter,
    pub decoration_id_counter: DecorationIdCounter,

    pub foreign_toplevel_list_handles:
        Vec<(ExtForeignToplevelHandleV1, ForeignToplevelListHandleData)>,
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
        let snowcap_decoration_manager: SnowcapDecorationManagerV1 =
            globals.bind(&queue_handle, 1..=1, ()).unwrap();
        let foreign_toplevel_list: ExtForeignToplevelListV1 =
            globals.bind(&queue_handle, 1..=1, ()).unwrap();

        let wayland_source = WaylandSource::new(conn.clone(), event_queue);

        let dispatcher = Dispatcher::new(wayland_source, |_, queue, data| {
            queue.dispatch_pending(data)
        });

        loop_handle.register_dispatcher(dispatcher.clone())?;

        let (request_redraw_ping, request_redraw_ping_source) = calloop::ping::make_ping().unwrap();
        let (invalidate_layout_ping, invalidate_layout_ping_source) =
            calloop::ping::make_ping().unwrap();

        loop_handle
            .insert_source(request_redraw_ping_source, |_, _, state| {
                for layer in state.layers.iter_mut() {
                    layer.schedule_redraw();
                }
                for deco in state.decorations.iter_mut() {
                    deco.schedule_redraw();
                }
            })
            .unwrap();

        loop_handle
            .insert_source(invalidate_layout_ping_source, |_, _, state| {
                for layer in state.layers.iter_mut() {
                    layer.surface.invalidate_layout();
                }
                for deco in state.decorations.iter_mut() {
                    deco.surface.invalidate_layout();
                }
            })
            .unwrap();

        let notifier = CalloopNotifier::new(request_redraw_ping, invalidate_layout_ping);
        let shell = iced_graphics::Shell::new(notifier);

        // Attempt to create a wgpu renderer upfront; this takes a non-trivial amount of time to do
        let compositor = crate::wgpu::Compositor::new(shell.clone())
            .ok()
            .map(crate::compositor::Compositor::Primary);

        let (sender, recv) = calloop::channel::channel::<(iced::window::Id, SnowcapMessage)>();
        let mut runtime = Runtime::new(CurrentTokioExecutor, CalloopSenderSink::new(sender));

        loop_handle
            .insert_source(recv, move |event, _, state| match event {
                calloop::channel::Event::Msg((id, msg)) => {
                    let Some(layer) = state
                        .layers
                        .iter()
                        .find(|layer| layer.surface.window_id == id)
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
                let captured = status == iced::event::Status::Captured;
                match event {
                    iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                        modifiers,
                        physical_key: Physical::Unidentified(NativeCode::Xkb(raw)),
                        text,
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
                            captured,
                            text: text.map(|s| s.into()),
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
                            captured,
                            text: None,
                        }),
                    )),
                    _ => None,
                }
            }),
        ));

        let seat = seat_state.seats().next();

        let state = State {
            loop_handle,
            loop_signal,
            conn: conn.clone(),
            wayland_source: dispatcher,
            shell,
            runtime,

            registry_state,
            seat_state,
            output_state,
            compositor_state,
            layer_shell_state,
            fractional_scale_manager,
            viewporter,
            snowcap_decoration_manager,
            foreign_toplevel_list,

            grpc_server_state: None,
            queue_handle,
            compositor,
            tiny_skia: None,
            layers: Vec::new(),
            decorations: Vec::new(),
            seat,
            keyboard_focus: None,
            keyboard_modifiers: smithay_client_toolkit::seat::keyboard::Modifiers::default(),
            keyboard: None,
            pointer: None,
            layer_id_counter: LayerIdCounter::default(),
            decoration_id_counter: DecorationIdCounter::default(),
            foreign_toplevel_list_handles: Vec::new(),
        };

        Ok(state)
    }

    pub(crate) fn find_surface_mut(
        &mut self,
        wl_surface: &WlSurface,
    ) -> Option<&mut surface::SnowcapSurface> {
        if let Some(surface) = self
            .layers
            .iter_mut()
            .filter_map(|l| {
                if l.layer.wl_surface() == wl_surface {
                    Some(&mut l.surface)
                } else {
                    None
                }
            })
            .next()
        {
            return Some(surface);
        }

        if let Some(surface) = self
            .decorations
            .iter_mut()
            .filter_map(|d| {
                if &d.surface.wl_surface == wl_surface {
                    Some(&mut d.surface)
                } else {
                    None
                }
            })
            .next()
        {
            return Some(surface);
        }

        None
    }
}
