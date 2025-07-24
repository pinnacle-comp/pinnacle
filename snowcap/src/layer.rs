use std::{num::NonZeroU32, ptr::NonNull, time::Instant};

use iced::{Color, Size, window::RedrawRequest};
use iced_graphics::Compositor;
use iced_runtime::user_interface;
use raw_window_handle::{
    HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle,
    WaylandWindowHandle,
};
use smithay_client_toolkit::{
    reexports::{
        calloop::{self, LoopHandle, timer::Timer},
        client::{Proxy, QueueHandle, protocol::wl_output::WlOutput},
        protocols::wp::{
            fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1,
            viewporter::client::wp_viewport::WpViewport,
        },
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
    pub surface: <iced_renderer::Compositor as iced_graphics::Compositor>::Surface,

    pub layer: LayerSurface,
    pub loop_handle: LoopHandle<'static, State>,

    pub renderer: iced_renderer::Renderer,

    /// The logical size of the output this layer is on.
    pub output_size: iced::Size<u32>,
    /// The scale of the output this layer is on.
    pub output_scale: f32,
    pub pending_size: Option<iced::Size<u32>>,
    pub pending_output_scale: Option<f32>,
    // COMPAT: 0.1
    pub max_size: Option<iced::Size<u32>>,

    redraw_requested: bool,
    pub widgets: SnowcapWidgetProgram,
    pub clipboard: WaylandClipboard,

    pub pointer_location: Option<(f64, f64)>,

    pub layer_id: LayerId,
    pub window_id: iced::window::Id,

    pub wl_output: Option<WlOutput>,
    pub viewport: WpViewport,
    fractional_scale: WpFractionalScaleV1,

    pub keyboard_key_sender: Option<UnboundedSender<KeyboardKey>>,
    pub pointer_button_sender: Option<UnboundedSender<Result<PointerButtonResponse, Status>>>,
    pub widget_event_sender: Option<UnboundedSender<(WidgetId, WidgetEvent)>>,

    pub initial_configure: InitialConfigureState,
}

impl Drop for SnowcapLayer {
    fn drop(&mut self) {
        self.fractional_scale.destroy();
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum InitialConfigureState {
    PreConfigure(Option<iced::Size<u32>>),
    PostConfigure,
    PostOutputSize,
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
        // COMPAT: 0.1
        max_size: Option<(u32, u32)>,
        layer: wlr_layer::Layer,
        anchor: Anchor,
        exclusive_zone: ExclusiveZone,
        keyboard_interactivity: wlr_layer::KeyboardInteractivity,
        widgets: ViewFn,
    ) -> Self {
        let surface = state.compositor_state.create_surface(&state.queue_handle);
        let viewport = state
            .viewporter
            .get_viewport(&surface, &state.queue_handle, ());
        let fractional_scale = state.fractional_scale_manager.get_fractional_scale(
            &surface,
            &state.queue_handle,
            surface.clone(),
        );
        let layer = state.layer_shell_state.create_layer_surface(
            &state.queue_handle,
            surface,
            layer,
            Some("snowcap"),
            None,
        );

        layer.set_size(1, 1);
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

        let iced_surface = compositor.create_surface(layer_window_handle, 1, 1);

        let clipboard =
            unsafe { WaylandClipboard::new(state.conn.backend().display_ptr() as *mut _) };

        let next_id = state.layer_id_counter.next();

        let widgets = SnowcapWidgetProgram::new(widgets, Size::new(1.0, 1.0), &mut renderer);

        Self {
            surface: iced_surface,
            loop_handle: state.loop_handle.clone(),
            layer,
            max_size: max_size.map(|(w, h)| iced::Size::new(w, h)),
            output_size: iced::Size::new(1, 1),
            pending_size: None,
            output_scale: 1.0,
            pending_output_scale: None,
            widgets,
            renderer,
            clipboard,
            pointer_location: None,
            wl_output: None,
            viewport,
            fractional_scale,
            layer_id: next_id,
            window_id: iced::window::Id::unique(),
            keyboard_key_sender: None,
            pointer_button_sender: None,
            widget_event_sender: None,
            initial_configure: InitialConfigureState::PreConfigure(None),
            redraw_requested: false,
        }
    }

    pub fn schedule_redraw(&mut self) {
        if self.redraw_requested {
            return;
        }

        self.redraw_requested = true;
        self.widgets
            .queue_event(iced::Event::Window(iced::window::Event::RedrawRequested(
                Instant::now(),
            )));
    }

    pub fn output_size_changed(&mut self, output_size: iced::Size<u32>, output_scale: f32) {
        if output_size != self.output_size {
            self.pending_size = Some(output_size);
        }

        if output_scale != self.output_scale {
            self.pending_output_scale = Some(output_scale);
        }
    }

    pub fn update_properties(
        &mut self,
        layer: Option<wlr_layer::Layer>,
        anchor: Option<Anchor>,
        exclusive_zone: Option<ExclusiveZone>,
        keyboard_interactivity: Option<wlr_layer::KeyboardInteractivity>,
        widgets: Option<ViewFn>,

        queue_handle: &QueueHandle<State>,
    ) {
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

        if let Some(widgets) = widgets {
            self.widgets
                .update_view(widgets, self.widget_bounds(), &mut self.renderer);
        }

        self.request_frame(queue_handle);
    }

    pub fn draw_if_scheduled(&mut self, compositor: &mut crate::compositor::Compositor) {
        if !self.redraw_requested {
            return;
        }
        self.redraw_requested = false;

        let cursor = match self.pointer_location {
            Some((x, y)) => iced::mouse::Cursor::Available(iced::Point {
                x: x as f32,
                y: y as f32,
            }),
            None => iced::mouse::Cursor::Unavailable,
        };

        if self.initial_configure == InitialConfigureState::PostOutputSize {
            self.widgets.draw(&mut self.renderer, cursor);
        }

        compositor
            .present(
                &mut self.renderer,
                &mut self.surface,
                &self.widgets.viewport(self.output_scale),
                Color::TRANSPARENT,
                || {},
            )
            .unwrap();
    }

    pub fn update(
        &mut self,
        queue_handle: &QueueHandle<State>,
        runtime: &mut crate::runtime::Runtime,
        compositor: &mut crate::compositor::Compositor,
    ) {
        if self.pending_output_scale.is_some() || self.pending_size.is_some() {
            if let Some(scale) = self.pending_output_scale.take() {
                self.output_scale = scale;
            }
            if let Some(size) = self.pending_size.take() {
                self.output_size = size;
            }

            self.widgets
                .rebuild_ui(self.widget_bounds(), &mut self.renderer);

            self.layer
                .set_size(self.widgets.size().width, self.widgets.size().height);
            self.viewport.set_destination(
                self.widgets.size().width as i32,
                self.widgets.size().height as i32,
            );

            let buffer_size = self.widgets.viewport(self.output_scale).physical_size();

            compositor.configure_surface(&mut self.surface, buffer_size.width, buffer_size.height);
        }

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
            request_frame = true;

            for message in messages {
                if let SnowcapMessage::WidgetEvent(id, widget_event) = message
                    && let Some(sender) = self.widget_event_sender.as_ref()
                {
                    let _ = sender.send((id, widget_event));
                }
            }

            self.widgets
                .rebuild_ui(self.widget_bounds(), &mut self.renderer);
        }

        if request_frame {
            self.request_frame(queue_handle);
        }
    }

    pub fn widget_bounds(&self) -> iced::Size<u32> {
        if let Some(max_size) = self.max_size {
            iced::Size::new(
                self.output_size.width.min(max_size.width),
                self.output_size.height.min(max_size.height),
            )
        } else {
            self.output_size
        }
    }

    pub fn request_frame(&self, queue_handle: &QueueHandle<State>) {
        self.layer
            .wl_surface()
            .frame(queue_handle, self.layer.wl_surface().clone());
        self.layer.wl_surface().commit();
    }
}
