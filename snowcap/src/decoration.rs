use std::{ptr::NonNull, time::Instant};

use iced::{Color, Size, window::RedrawRequest};
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
        protocols::{
            ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
            wp::{
                fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1,
                viewporter::client::wp_viewport::WpViewport,
            },
        },
    },
};
use snowcap_api_defs::snowcap::input::v0alpha1::PointerButtonResponse;
use snowcap_protocols::snowcap_decoration_v1::client::snowcap_decoration_surface_v1::SnowcapDecorationSurfaceV1;
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
pub struct DecorationId(pub u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct DecorationIdCounter(DecorationId);

impl DecorationIdCounter {
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> DecorationId {
        let ret = self.0;
        self.0.0 += 1;
        ret
    }
}

impl State {
    pub fn decoration_for_id(&mut self, id: DecorationId) -> Option<&mut SnowcapDecoration> {
        self.decorations
            .iter_mut()
            .find(|deco| deco.decoration_id == id)
    }
}

pub struct SnowcapDecoration {
    // SAFETY: Drop order: surface needs to be dropped before the wl surface
    pub surface: <iced_renderer::Compositor as iced_graphics::Compositor>::Surface,

    pub decoration: SnowcapDecorationSurfaceV1,
    pub wl_surface: WlSurface,
    pub loop_handle: LoopHandle<'static, State>,
    pub foreign_toplevel_list_handle: ExtForeignToplevelHandleV1,

    pub renderer: iced_renderer::Renderer,

    /// The scale of the output this layer is on.
    pub output_scale: f32,
    pub pending_output_scale: Option<f32>,

    redraw_requested: bool,
    pub widgets: SnowcapWidgetProgram,
    pub clipboard: WaylandClipboard,

    pub pointer_location: Option<(f64, f64)>,

    pub decoration_id: DecorationId,
    pub window_id: iced::window::Id,

    pub viewport: WpViewport,
    fractional_scale: WpFractionalScaleV1,

    pub keyboard_key_sender: Option<UnboundedSender<KeyboardKey>>,
    pub pointer_button_sender: Option<UnboundedSender<Result<PointerButtonResponse, Status>>>,
    pub widget_event_sender: Option<UnboundedSender<(WidgetId, WidgetEvent)>>,

    pub initial_configure_received: bool,

    pub extents: Bounds,
    pub pending_toplevel_size: Option<iced::Size<u32>>,
    pub toplevel_size: iced::Size<u32>,

    pub bounds: Bounds,
}

impl Drop for SnowcapDecoration {
    fn drop(&mut self) {
        self.fractional_scale.destroy();
    }
}

#[derive(Clone, Copy)]
struct DecorationWindowHandle {
    display: RawDisplayHandle,
    window: RawWindowHandle,
}

// SAFETY: The objects that the pointers are derived from are Send and Sync
unsafe impl Send for DecorationWindowHandle {}
unsafe impl Sync for DecorationWindowHandle {}

impl HasDisplayHandle for DecorationWindowHandle {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        // SAFETY: The raw display pointer remains valid as long as
        // the iced renderer surface is dropped first (at the top
        // of `SnowcapLayer` in declaration order)
        Ok(unsafe { raw_window_handle::DisplayHandle::borrow_raw(self.display) })
    }
}

impl HasWindowHandle for DecorationWindowHandle {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        // SAFETY: The raw window pointer remains valid as long as
        // the iced renderer surface is dropped first (at the top
        // of `SnowcapLayer` in declaration order)
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(self.window) })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bounds {
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Geometry {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl SnowcapDecoration {
    pub fn new(
        state: &mut State,
        toplevel_identifier: String,
        bounds: Bounds,
        z_index: i32,
        extents: Bounds,
        widgets: ViewFn,
    ) -> Option<Self> {
        let foreign_toplevel_list_handle = state
            .foreign_toplevel_list_handles
            .iter()
            .find_map(|(handle, ident)| {
                (ident.identifier() == Some(&toplevel_identifier)).then_some(handle)
            })
            .cloned()?;

        let surface = state.compositor_state.create_surface(&state.queue_handle);
        let viewport = state
            .viewporter
            .get_viewport(&surface, &state.queue_handle, ());
        let fractional_scale = state.fractional_scale_manager.get_fractional_scale(
            &surface,
            &state.queue_handle,
            surface.clone(),
        );

        let deco = state.snowcap_decoration_manager.get_decoration_surface(
            &surface,
            &foreign_toplevel_list_handle,
            &state.queue_handle,
            (),
        );

        deco.set_bounds(bounds.left, bounds.right, bounds.top, bounds.bottom);
        deco.set_z_index(z_index);
        deco.set_location(
            bounds.left as i32 - extents.left as i32,
            bounds.top as i32 - extents.top as i32,
        );

        surface.commit();

        let raw_display_handle = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(
            NonNull::new(state.conn.backend().display_ptr() as *mut _).unwrap(),
        ));
        let raw_window_handle = RawWindowHandle::Wayland(WaylandWindowHandle::new(
            NonNull::new(surface.id().as_ptr() as *mut _).unwrap(),
        ));

        let deco_window_handle = DecorationWindowHandle {
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
                deco_window_handle,
            )
            .block_on_tokio()
            .unwrap()
        });

        let mut renderer = compositor.create_renderer();

        let iced_surface = compositor.create_surface(deco_window_handle, 1, 1);

        let clipboard =
            unsafe { WaylandClipboard::new(state.conn.backend().display_ptr() as *mut _) };

        let next_id = state.decoration_id_counter.next();

        let widgets = SnowcapWidgetProgram::new(widgets, Size::new(1.0, 1.0), &mut renderer);

        Some(Self {
            surface: iced_surface,
            loop_handle: state.loop_handle.clone(),
            decoration: deco,
            wl_surface: surface,
            foreign_toplevel_list_handle,
            output_scale: 1.0,
            pending_output_scale: None,
            widgets,
            renderer,
            clipboard,
            pointer_location: None,
            viewport,
            fractional_scale,
            decoration_id: next_id,
            window_id: iced::window::Id::unique(),
            keyboard_key_sender: None,
            pointer_button_sender: None,
            widget_event_sender: None,
            initial_configure_received: false,
            redraw_requested: false,
            extents,
            pending_toplevel_size: None,
            toplevel_size: iced::Size::new(1, 1),
            bounds,
        })
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

    pub fn output_scale_changed(&mut self, output_scale: f32) {
        if output_scale != self.output_scale {
            self.pending_output_scale = Some(output_scale);
        }
    }

    pub fn update_properties(
        &mut self,
        compositor_state: &CompositorState,
        widgets: Option<ViewFn>,
        bounds: Option<Bounds>,
        extents: Option<Bounds>,
        z_index: Option<i32>,
        queue_handle: &QueueHandle<State>,
    ) {
        // FIXME: make these pending, update on next draw + commit

        if let Some(widgets) = widgets {
            self.widgets
                .update_view(widgets, self.widget_bounds(), &mut self.renderer);
            self.widgets
                .update_input_region(queue_handle, compositor_state, &self.wl_surface);
        }

        if let Some(bounds) = bounds {
            self.decoration
                .set_bounds(bounds.left, bounds.right, bounds.top, bounds.bottom);
            self.bounds = bounds;
        }

        if let Some(extents) = extents {
            self.extents = extents;
        }

        if let Some(z_index) = z_index {
            self.decoration.set_z_index(z_index);
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

        if self.initial_configure_received {
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
        compositor_state: &CompositorState,
        queue_handle: &QueueHandle<State>,
        runtime: &mut crate::runtime::Runtime,
        compositor: &mut crate::compositor::Compositor,
    ) {
        if self.pending_output_scale.is_some() || self.pending_toplevel_size.is_some() {
            if let Some(scale) = self.pending_output_scale.take() {
                self.output_scale = scale;
            }
            if let Some(size) = self.pending_toplevel_size.take() {
                self.toplevel_size = size;
            }

            self.widgets
                .rebuild_ui(self.widget_bounds(), &mut self.renderer);

            self.widgets
                .update_input_region(queue_handle, compositor_state, &self.wl_surface);

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

            self.widgets
                .update_input_region(queue_handle, compositor_state, &self.wl_surface);
        }

        if request_frame {
            self.request_frame(queue_handle);
        }
    }

    pub fn widget_bounds(&self) -> iced::Size<u32> {
        iced::Size::new(
            self.toplevel_size.width + self.extents.left + self.extents.right,
            self.toplevel_size.height + self.extents.top + self.extents.bottom,
        )
    }

    pub fn request_frame(&self, queue_handle: &QueueHandle<State>) {
        self.wl_surface.frame(queue_handle, self.wl_surface.clone());
        self.wl_surface.commit();
    }
}
