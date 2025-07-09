use std::{any::Any, collections::HashMap, num::NonZeroU32, ptr::NonNull};

use iced::{Color, Size, Theme, window::RedrawRequest};
use iced_futures::Runtime;
use iced_graphics::Compositor;
use iced_runtime::{UserInterface, user_interface};
use iced_wgpu::graphics::Viewport;
use raw_window_handle::{
    HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle,
    WaylandWindowHandle,
};
use smithay_client_toolkit::{
    reexports::{
        calloop::{self, LoopHandle, timer::Timer},
        client::{Proxy, QueueHandle},
    },
    shell::{
        WaylandSurface,
        wlr_layer::{self, Anchor, LayerSurface},
    },
};
use snowcap_api_defs::snowcap::input::v0alpha1::{KeyboardKeyResponse, PointerButtonResponse};
use tokio::sync::mpsc::UnboundedSender;
use tonic::Status;

use crate::{
    clipboard::WaylandClipboard,
    runtime::{CalloopSenderSink, CurrentTokioExecutor},
    state::State,
    util::BlockOnTokio,
    widget::{SnowcapMessage, SnowcapWidgetProgram, WidgetFn, WidgetId},
};

pub struct SnowcapLayer {
    // SAFETY: Drop order: surface needs to be dropped before the layer
    surface: <iced_renderer::Compositor as iced_graphics::Compositor>::Surface,

    pub layer: LayerSurface,
    pub loop_handle: LoopHandle<'static, State>,

    pub renderer: iced_renderer::Renderer,

    pub width: u32,
    pub height: u32,
    pub scale: i32,
    pub viewport: Viewport,

    pub redraw_requested: bool,
    pub widgets: SnowcapWidgetProgram,
    pub clipboard: WaylandClipboard,

    pub pointer_location: Option<(f64, f64)>,

    pub runtime: Runtime<CurrentTokioExecutor, CalloopSenderSink<SnowcapMessage>, SnowcapMessage>,

    pub widget_id: WidgetId,

    pub keyboard_key_sender: Option<UnboundedSender<Result<KeyboardKeyResponse, Status>>>,
    pub pointer_button_sender: Option<UnboundedSender<Result<PointerButtonResponse, Status>>>,

    pub initial_configure: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ExclusiveZone {
    /// This layer surface wants an exclusive zone of the given size.
    Exclusive(NonZeroU32),
    /// This layer surface does not have an exclusive zone but wants to be placed respecting any.
    Respect,
    /// This layer surface does not have an exclusive zone and wants to be placed ignoring any.
    Ignore,
}

#[derive(Clone, Copy)]
struct LayerWindowHandle {
    display: RawDisplayHandle,
    window: RawWindowHandle,
}

// SAFETY: The objects that the pointers are derived from are Send and Sync
unsafe impl Send for LayerWindowHandle {}
unsafe impl Sync for LayerWindowHandle {}

impl HasDisplayHandle for LayerWindowHandle {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        // SAFETY: The raw display pointer remains valid as long as
        // the iced renderer surface is dropped first (at the top
        // of `SnowcapLayer` in declaration order)
        Ok(unsafe { raw_window_handle::DisplayHandle::borrow_raw(self.display) })
    }
}

impl HasWindowHandle for LayerWindowHandle {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        // SAFETY: The raw window pointer remains valid as long as
        // the iced renderer surface is dropped first (at the top
        // of `SnowcapLayer` in declaration order)
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(self.window) })
    }
}

impl SnowcapLayer {
    pub fn new(
        state: &mut State,
        width: u32,
        height: u32,
        layer: wlr_layer::Layer,
        anchor: Anchor,
        exclusive_zone: ExclusiveZone,
        keyboard_interactivity: wlr_layer::KeyboardInteractivity,
        widgets: WidgetFn,
        states: HashMap<u32, Box<dyn Any + Send>>,
    ) -> Self {
        let surface = state.compositor_state.create_surface(&state.queue_handle);
        let layer = state.layer_shell_state.create_layer_surface(
            &state.queue_handle,
            surface,
            layer,
            Some("snowcap"),
            None,
        );

        layer.set_size(width, height);
        layer.set_anchor(anchor);
        layer.set_keyboard_interactivity(keyboard_interactivity);
        layer.set_exclusive_zone(match exclusive_zone {
            ExclusiveZone::Exclusive(size) => size.get() as i32,
            ExclusiveZone::Respect => 0,
            ExclusiveZone::Ignore => -1,
        });

        layer.commit();

        let raw_display_handle = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(
            NonNull::new(state.conn.backend().display_ptr() as *mut _).unwrap(),
        ));
        let raw_window_handle = RawWindowHandle::Wayland(WaylandWindowHandle::new(
            NonNull::new(layer.wl_surface().id().as_ptr() as *mut _).unwrap(),
        ));

        let layer_window_handle = LayerWindowHandle {
            display: raw_display_handle,
            window: raw_window_handle,
        };

        let compositor = state.compositor.get_or_insert_with(|| {
            crate::compositor::Compositor::new(
                iced_graphics::Settings {
                    default_font: Default::default(),
                    default_text_size: iced::Pixels(16.0),
                    antialiasing: None,
                },
                layer_window_handle,
            )
            .block_on_tokio()
            .unwrap()
        });

        let mut renderer = compositor.create_renderer();

        let iced_surface = compositor.create_surface(layer_window_handle, width, height);

        let clipboard =
            unsafe { WaylandClipboard::new(state.conn.backend().display_ptr() as *mut _) };

        let (sender, recv) = calloop::channel::channel::<SnowcapMessage>();
        let runtime = Runtime::new(CurrentTokioExecutor, CalloopSenderSink::new(sender));

        let layer_clone = layer.clone();
        state
            .loop_handle
            .insert_source(recv, move |event, _, state| match event {
                calloop::channel::Event::Msg(message) => {
                    let Some(layer) = state
                        .layers
                        .iter_mut()
                        .find(|sn_layer| sn_layer.layer == layer_clone)
                    else {
                        return;
                    };

                    match message {
                        SnowcapMessage::Close => {
                            state
                                .layers
                                .retain(|sn_layer| sn_layer.layer != layer_clone);
                        }
                        msg => {
                            layer.widgets.queued_messages.push(msg);
                        }
                    }
                }
                calloop::channel::Event::Closed => (),
            })
            .unwrap();

        let next_id = state.widget_id_counter.next_and_increment();

        let viewport = Viewport::with_physical_size(Size::new(width, height), 1.0);

        let widgets =
            SnowcapWidgetProgram::new(widgets, states, viewport.logical_size(), &mut renderer);

        Self {
            surface: iced_surface,
            loop_handle: state.loop_handle.clone(),
            layer,
            width,
            height,
            scale: 1,
            viewport,
            widgets,
            renderer,
            clipboard,
            pointer_location: None,
            runtime,
            widget_id: next_id,
            keyboard_key_sender: None,
            pointer_button_sender: None,
            initial_configure: false,
            redraw_requested: false,
        }
    }

    pub fn draw(&mut self) {
        use iced_renderer::fallback::Renderer;
        use iced_renderer::fallback::Surface;

        let cursor = match self.pointer_location {
            Some((x, y)) => iced::mouse::Cursor::Available(iced::Point {
                x: x as f32,
                y: y as f32,
            }),
            None => iced::mouse::Cursor::Unavailable,
        };

        self.widgets.user_interface.as_mut().unwrap().draw(
            &mut self.renderer,
            &Theme::CatppuccinFrappe,
            &iced_wgpu::core::renderer::Style {
                text_color: Color::WHITE,
            },
            cursor,
        );

        match &mut self.renderer {
            Renderer::Primary(wgpu) => {
                let Surface::Primary(surface) = &mut self.surface else {
                    unreachable!();
                };
                iced_wgpu::window::compositor::present(
                    wgpu,
                    surface,
                    &self.viewport,
                    Color::TRANSPARENT,
                    || {},
                )
                .unwrap();
            }
            Renderer::Secondary(skia) => {
                let Surface::Secondary(surface) = &mut self.surface else {
                    unreachable!();
                };
                iced_tiny_skia::window::compositor::present(
                    skia,
                    surface,
                    &self.viewport,
                    Color::TRANSPARENT,
                    || {},
                )
                .unwrap();
            }
        }
    }

    pub fn update(&mut self, queue_handle: &QueueHandle<State>) {
        let cursor = match self.pointer_location {
            Some((x, y)) => iced::mouse::Cursor::Available(iced::Point {
                x: x as f32,
                y: y as f32,
            }),
            None => iced::mouse::Cursor::Unavailable,
        };

        let mut messages = Vec::new();

        let (state, statuses) = self
            .widgets
            .user_interface
            .as_mut()
            .expect("ui should exist")
            .update(
                &self.widgets.queued_events,
                cursor,
                &mut self.renderer,
                &mut self.clipboard,
                &mut messages,
            );

        let mut ui_stale = false;
        let mut request_frame = false;

        match state {
            user_interface::State::Outdated => {
                ui_stale = true;
            }
            user_interface::State::Updated {
                mouse_interaction: _, // TODO:
                redraw_request,
                input_method: _,
            } => match redraw_request {
                RedrawRequest::NextFrame => {
                    request_frame = true;
                }
                RedrawRequest::At(instant) => {
                    let surface = self.layer.wl_surface().clone();
                    self.loop_handle
                        .insert_source(Timer::from_deadline(instant), move |_, _, state| {
                            surface.frame(&state.queue_handle, surface.clone());
                            surface.commit();
                            calloop::timer::TimeoutAction::Drop
                        })
                        .unwrap();
                }
                RedrawRequest::Wait => (),
            },
        }

        for (event, status) in self.widgets.queued_events.iter().zip(statuses) {
            self.runtime
                .broadcast(iced_futures::subscription::Event::Interaction {
                    window: iced::window::Id::unique(),
                    event: event.clone(),
                    status,
                });
        }

        self.widgets.queued_events.clear();
        messages.append(&mut self.widgets.queued_messages);

        // If there are messages, we'll need to recreate the UI with the new state.
        if !messages.is_empty() || ui_stale {
            // TODO: Update SnowcapWidgetProgram with messages
            request_frame = true;
            let cache = self.widgets.user_interface.take().unwrap().into_cache();
            let view = (self.widgets.widgets)(&self.widgets.widget_state);
            self.widgets.user_interface = Some(UserInterface::build(
                view,
                self.viewport.logical_size(),
                cache,
                &mut self.renderer,
            ));
        }

        if request_frame {
            self.layer
                .wl_surface()
                .frame(queue_handle, self.layer.wl_surface().clone());
            self.layer.wl_surface().commit();
        }
    }

    pub fn set_scale(&mut self, scale: i32, compositor: &mut crate::compositor::Compositor) {
        self.scale = scale;
        self.layer.wl_surface().set_buffer_scale(scale);

        compositor.configure_surface(
            &mut self.surface,
            self.width * scale as u32,
            self.height * scale as u32,
        );
    }
}
