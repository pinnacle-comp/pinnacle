use std::{num::NonZeroU32, ptr::NonNull};

use iced::{Color, Size, window::RedrawRequest};
use iced_graphics::Compositor;
use iced_runtime::user_interface;
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
use snowcap_api_defs::snowcap::input::v0alpha1::PointerButtonResponse;
use tokio::sync::mpsc::UnboundedSender;
use tonic::Status;

use crate::{
    clipboard::WaylandClipboard,
    handlers::keyboard::KeyboardKey,
    state::State,
    util::BlockOnTokio,
    widget::{SnowcapMessage, SnowcapWidgetProgram, ViewFn, WidgetEvent, WidgetId},
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct LayerId(pub u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct LayerIdCounter(LayerId);

impl LayerIdCounter {
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> LayerId {
        let ret = self.0;
        self.0.0 += 1;
        ret
    }
}

impl State {
    pub fn layer_for_id(&mut self, id: LayerId) -> Option<&mut SnowcapLayer> {
        self.layers.iter_mut().find(|layer| layer.layer_id == id)
    }
}

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

    pub layer_id: LayerId,
    pub window_id: iced::window::Id,

    pub keyboard_key_sender: Option<UnboundedSender<KeyboardKey>>,
    pub pointer_button_sender: Option<UnboundedSender<Result<PointerButtonResponse, Status>>>,
    pub widget_event_sender: Option<UnboundedSender<(WidgetId, WidgetEvent)>>,

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
        widgets: ViewFn,
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

        let next_id = state.layer_id_counter.next();

        let viewport = Viewport::with_physical_size(Size::new(width, height), 1.0);

        let widgets = SnowcapWidgetProgram::new(widgets, viewport.logical_size(), &mut renderer);

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
            layer_id: next_id,
            window_id: iced::window::Id::unique(),
            keyboard_key_sender: None,
            pointer_button_sender: None,
            widget_event_sender: None,
            initial_configure: false,
            redraw_requested: false,
        }
    }

    pub fn update_properties(
        &mut self,
        width: Option<u32>,
        height: Option<u32>,
        layer: Option<wlr_layer::Layer>,
        anchor: Option<Anchor>,
        exclusive_zone: Option<ExclusiveZone>,
        keyboard_interactivity: Option<wlr_layer::KeyboardInteractivity>,
        widgets: Option<ViewFn>,

        queue_handle: &QueueHandle<State>,
        compositor: &mut crate::compositor::Compositor,
    ) {
        if width.is_some() || height.is_some() {
            self.width = width.unwrap_or(self.width);
            self.height = height.unwrap_or(self.height);
            compositor.configure_surface(
                &mut self.surface,
                self.width * self.scale as u32,
                self.height * self.scale as u32,
            );
        }

        if let Some(layer) = layer {
            self.layer.set_layer(layer);
        }

        if let Some(anchor) = anchor {
            self.layer.set_anchor(anchor);
        }

        if let Some(zone) = exclusive_zone {
            self.layer.set_exclusive_zone(match zone {
                ExclusiveZone::Exclusive(size) => size.get() as i32,
                ExclusiveZone::Respect => 0,
                ExclusiveZone::Ignore => -1,
            });
        }

        if let Some(keyboard_interactivity) = keyboard_interactivity {
            self.layer
                .set_keyboard_interactivity(keyboard_interactivity);
        }

        self.viewport = Viewport::with_physical_size(
            iced::Size::new(
                self.width * self.scale as u32,
                self.height * self.scale as u32,
            ),
            self.scale as f64,
        );

        if let Some(widgets) = widgets {
            self.widgets
                .update_view(widgets, self.viewport.logical_size(), &mut self.renderer);
        }

        self.layer
            .wl_surface()
            .frame(queue_handle, self.layer.wl_surface().clone());
        self.layer.wl_surface().commit();
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

        self.widgets.draw(&mut self.renderer, cursor);

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

    pub fn update(
        &mut self,
        queue_handle: &QueueHandle<State>,
        runtime: &mut crate::runtime::Runtime,
    ) {
        let cursor = match self.pointer_location {
            Some((x, y)) => iced::mouse::Cursor::Available(iced::Point {
                x: x as f32,
                y: y as f32,
            }),
            None => iced::mouse::Cursor::Unavailable,
        };

        let mut messages = Vec::new();

        let (state, statuses) = self.widgets.update(
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

        for (event, status) in self.widgets.drain_events().zip(statuses) {
            runtime.broadcast(iced_futures::subscription::Event::Interaction {
                window: self.window_id,
                event,
                status,
            });
        }

        // If there are messages, we'll need to recreate the UI with the new state.
        if !messages.is_empty() || ui_stale {
            // TODO: Update SnowcapWidgetProgram with messages
            request_frame = true;

            for message in messages {
                if let SnowcapMessage::WidgetEvent(id, widget_event) = message
                    && let Some(sender) = self.widget_event_sender.as_ref()
                {
                    let _ = sender.send((id, widget_event));
                }
            }

            self.widgets
                .rebuild_ui(self.viewport.logical_size(), &mut self.renderer);
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
