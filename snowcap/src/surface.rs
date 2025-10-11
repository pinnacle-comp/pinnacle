use std::{ptr::NonNull, time::Instant};

use iced::window::RedrawRequest;
use iced_graphics::Compositor;
use iced_runtime::user_interface;
use raw_window_handle::{
    HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle,
    WaylandWindowHandle,
};
use smithay_client_toolkit::{
    compositor::CompositorState,
    reexports::{
        calloop::{self, LoopHandle, timer::Timer},
        client::{Proxy, QueueHandle, protocol::wl_surface::WlSurface},
        protocols::wp::{
            fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1,
            viewporter::client::wp_viewport::WpViewport,
        },
    },
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    clipboard::WaylandClipboard,
    compositor::{Renderer, Surface},
    state::State,
    util::BlockOnTokio,
    widget::{SnowcapMessage, SnowcapWidgetProgram, ViewFn, WidgetEvent, WidgetId},
};

pub struct SnowcapSurface {
    // This is an option so we can drop it first
    surface: Option<<crate::compositor::Compositor as iced_graphics::Compositor>::Surface>,
    pub wl_surface: WlSurface,
    compositor_state: CompositorState,
    queue_handle: QueueHandle<State>,

    loop_handle: LoopHandle<'static, State>,

    /// The scale of the output this layer is on.
    output_scale: f32,
    pending_output_scale: Option<f32>,
    bounds: iced::Size<u32>,
    pending_bounds: Option<iced::Size<u32>>,

    renderer: Renderer,

    redraw_scheduled: bool,
    pending_view: Option<ViewFn>,
    waiting_view: bool,
    pub widgets: SnowcapWidgetProgram,
    clipboard: WaylandClipboard,

    pub pointer_location: Option<(f64, f64)>,

    pub window_id: iced::window::Id,

    viewport: WpViewport,
    fractional_scale: WpFractionalScaleV1,

    pub widget_event_sender: Option<UnboundedSender<Vec<(WidgetId, WidgetEvent)>>>,
}

impl Drop for SnowcapSurface {
    fn drop(&mut self) {
        // SAFETY: This needs to be dropped first, it implicitly borrows the wl_surface
        self.surface.take();

        self.fractional_scale.destroy();
        self.wl_surface.destroy();
        self.viewport.destroy();
    }
}

impl SnowcapSurface {
    pub fn new(state: &mut State, widgets: ViewFn, force_tiny_skia: bool) -> Self {
        let wl_surface = state.compositor_state.create_surface(&state.queue_handle);
        let viewport = state
            .viewporter
            .get_viewport(&wl_surface, &state.queue_handle, ());
        let fractional_scale = state.fractional_scale_manager.get_fractional_scale(
            &wl_surface,
            &state.queue_handle,
            wl_surface.clone(),
        );
        let compositor_state = state.compositor_state.clone();

        let window_handle = WindowHandle::new(&wl_surface);

        let compositor = if force_tiny_skia {
            state.tiny_skia.get_or_insert_with(|| {
                let tiny_skia = iced_tiny_skia::window::compositor::new(
                    iced_graphics::Settings {
                        default_font: Default::default(),
                        default_text_size: iced::Pixels(16.0),
                        antialiasing: None,
                    }
                    .into(),
                    window_handle,
                );

                crate::compositor::Compositor::Secondary(tiny_skia)
            })
        } else {
            state.compositor.get_or_insert_with(|| {
                crate::compositor::Compositor::new(
                    iced_graphics::Settings {
                        default_font: Default::default(),
                        default_text_size: iced::Pixels(16.0),
                        antialiasing: None,
                    },
                    window_handle,
                )
                .block_on_tokio()
                .unwrap()
            })
        };

        let renderer = compositor.create_renderer();

        let iced_surface = compositor.create_surface(window_handle, 1, 1);

        let clipboard = WaylandClipboard;

        let widgets = SnowcapWidgetProgram::new(widgets);

        Self {
            surface: Some(iced_surface),
            wl_surface,
            compositor_state,
            queue_handle: state.queue_handle.clone(),
            loop_handle: state.loop_handle.clone(),
            output_scale: 1.0,
            pending_output_scale: None,
            bounds: iced::Size::default(),
            pending_bounds: None,
            pending_view: None,
            waiting_view: false,
            widgets,
            renderer,
            clipboard,
            pointer_location: None,
            viewport,
            fractional_scale,
            window_id: iced::window::Id::unique(),
            widget_event_sender: None,
            redraw_scheduled: false,
        }
    }

    pub fn scale_changed(&mut self, new_scale: f32) {
        self.pending_output_scale = Some(new_scale);
    }

    pub fn bounds_changed(&mut self, new_bounds: iced::Size<u32>) {
        self.pending_bounds = Some(new_bounds);
    }

    pub fn view_changed(&mut self, new_view: ViewFn) {
        self.pending_view = Some(new_view);
    }

    pub fn schedule_redraw(&mut self) {
        if self.redraw_scheduled {
            return;
        }

        self.redraw_scheduled = true;
        self.widgets
            .queue_event(iced::Event::Window(iced::window::Event::RedrawRequested(
                Instant::now(),
            )));
    }

    pub fn draw_if_scheduled(&mut self) {
        let _span = tracy_client::span!("SnowcapSurface::draw_if_scheduled");

        if !self.redraw_scheduled {
            return;
        }
        self.redraw_scheduled = false;

        let cursor = match self.pointer_location {
            Some((x, y)) => iced::mouse::Cursor::Available(iced::Point {
                x: x as f32,
                y: y as f32,
            }),
            None => iced::mouse::Cursor::Unavailable,
        };

        self.widgets.draw(&mut self.renderer, cursor);

        match (&mut self.renderer, self.surface.as_mut().unwrap()) {
            (Renderer::Primary(renderer), Surface::Primary(surface)) => {
                iced_wgpu::window::compositor::present(
                    renderer,
                    surface,
                    &self.widgets.viewport(self.output_scale),
                    iced::Color::TRANSPARENT,
                    || {},
                )
                .unwrap()
            }
            (Renderer::Secondary(renderer), Surface::Secondary(surface)) => {
                iced_tiny_skia::window::compositor::present(
                    renderer,
                    surface,
                    &self.widgets.viewport(self.output_scale),
                    iced::Color::TRANSPARENT,
                    || {},
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
    }

    /// Updates this surface.
    ///
    /// Returns whether the widgets have resized as a result.
    pub fn update(
        &mut self,
        runtime: &mut crate::runtime::Runtime,
        compositor: &mut crate::compositor::Compositor,
    ) -> bool {
        let _span = tracy_client::span!("SnowcapSurface::update");

        let mut needs_rebuild = false;
        if let Some(scale) = self.pending_output_scale.take()
            && scale != self.output_scale
        {
            // HACK: With exact fractional scaling, there's a small seam between
            // adjacent widgets with fractional scales like 1.125.
            // Rounding up to the nearest 0.25 seems to work around that issue.
            self.output_scale = (scale * 4.0).ceil() / 4.0;
            needs_rebuild = true;
        }
        if let Some(bounds) = self.pending_bounds.take()
            && bounds != self.bounds
        {
            self.bounds = bounds;
            needs_rebuild = true;
        }
        if self.pending_view.is_some() {
            needs_rebuild = true;
            self.waiting_view = false;
        }

        let mut resized = false;

        if needs_rebuild {
            let old_size = self.widgets.size();

            self.widgets
                .rebuild_ui(self.bounds, &mut self.renderer, self.pending_view.take())
                .update(&self.queue_handle, &self.compositor_state, &self.wl_surface);

            if self.widgets.size() != old_size {
                resized = true;
            }

            self.viewport.set_destination(
                self.widgets.size().width as i32,
                self.widgets.size().height as i32,
            );

            let buffer_size = self.widgets.viewport(self.output_scale).physical_size();

            compositor.configure_surface(
                self.surface.as_mut().unwrap(),
                buffer_size.width,
                buffer_size.height,
            );
        }

        let cursor = match self.pointer_location {
            Some((x, y)) => iced::mouse::Cursor::Available(iced::Point {
                x: x as f32,
                y: y as f32,
            }),
            None => iced::mouse::Cursor::Unavailable,
        };

        let mut messages = Vec::new();

        if self.waiting_view {
            return resized;
        }

        let Some((state, statuses)) = self.widgets.update(
            cursor,
            &mut self.renderer,
            &mut self.clipboard,
            &mut messages,
        ) else {
            return resized;
        };

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
                    let surface = self.wl_surface.clone();
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

        if !messages.is_empty()
            && let Some(sender) = self.widget_event_sender.as_ref()
        {
            let widget_events: Vec<_> = messages
                .into_iter()
                .filter_map(|message| {
                    if let SnowcapMessage::WidgetEvent(id, widget_event) = message {
                        Some((id, widget_event))
                    } else {
                        None
                    }
                })
                .collect();

            self.waiting_view = true;
            let _ = sender.send(widget_events);
        }

        // If there are messages, we'll need to recreate the UI with the new state.
        if ui_stale {
            request_frame = true;

            self.widgets
                .rebuild_ui(self.bounds, &mut self.renderer, None)
                .update(&self.queue_handle, &self.compositor_state, &self.wl_surface);
        }

        if request_frame {
            self.request_frame();
        }

        resized
    }

    pub fn request_frame(&self) {
        self.wl_surface
            .frame(&self.queue_handle, self.wl_surface.clone());
        self.wl_surface.commit();
    }
}

#[derive(Clone, Copy)]
struct WindowHandle {
    display: RawDisplayHandle,
    window: RawWindowHandle,
}

impl WindowHandle {
    fn new(surface: &WlSurface) -> Self {
        let raw_display_handle = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(
            NonNull::new(surface.backend().upgrade().unwrap().display_ptr() as *mut _).unwrap(),
        ));

        let raw_window_handle = RawWindowHandle::Wayland(WaylandWindowHandle::new(
            NonNull::new(surface.id().as_ptr() as *mut _).unwrap(),
        ));

        WindowHandle {
            display: raw_display_handle,
            window: raw_window_handle,
        }
    }
}

// SAFETY: The objects that the pointers are derived from are Send and Sync
unsafe impl Send for WindowHandle {}
unsafe impl Sync for WindowHandle {}

impl HasDisplayHandle for WindowHandle {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        // SAFETY: The raw display pointer remains valid as long as
        // the iced renderer surface is dropped first (at the top
        // of `SnowcapLayer` in declaration order)
        Ok(unsafe { raw_window_handle::DisplayHandle::borrow_raw(self.display) })
    }
}

impl HasWindowHandle for WindowHandle {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        // SAFETY: The raw window pointer remains valid as long as
        // the iced renderer surface is dropped first (at the top
        // of `SnowcapLayer` in declaration order)
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(self.window) })
    }
}
