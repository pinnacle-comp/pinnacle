use std::{num::NonZeroU32, ptr::NonNull};

use iced::{Color, Size, Theme};
use iced_futures::Runtime;
use iced_runtime::Debug;
use iced_wgpu::{graphics::Viewport, wgpu::SurfaceTargetUnsafe};
use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use smithay_client_toolkit::{
    reexports::{
        calloop,
        client::{Proxy, QueueHandle},
    },
    shell::{
        wlr_layer::{self, Anchor, LayerSurface},
        WaylandSurface,
    },
};
use snowcap_api_defs::snowcap::input::v0alpha1::{KeyboardKeyResponse, PointerButtonResponse};
use tokio::sync::mpsc::UnboundedSender;
use tonic::Status;

use crate::{
    clipboard::WaylandClipboard,
    runtime::{CalloopSenderSink, CurrentTokioExecutor},
    state::State,
    widget::{SnowcapMessage, SnowcapWidgetProgram, WidgetId},
};

pub struct SnowcapLayer {
    // SAFETY: Drop order: surface needs to be dropped before the layer
    surface: iced_wgpu::wgpu::Surface<'static>,
    pub layer: LayerSurface,

    pub width: u32,
    pub height: u32,
    pub scale: i32,
    pub viewport: Viewport,

    pub widgets: iced_runtime::program::State<SnowcapWidgetProgram>,
    pub clipboard: WaylandClipboard,

    pub pointer_location: Option<(f64, f64)>,

    pub runtime: Runtime<CurrentTokioExecutor, CalloopSenderSink<SnowcapMessage>, SnowcapMessage>,

    pub widget_id: WidgetId,

    pub keyboard_key_sender: Option<UnboundedSender<Result<KeyboardKeyResponse, Status>>>,
    pub pointer_button_sender: Option<UnboundedSender<Result<PointerButtonResponse, Status>>>,
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

impl SnowcapLayer {
    pub fn new(
        state: &mut State,
        width: u32,
        height: u32,
        layer: wlr_layer::Layer,
        anchor: Anchor,
        exclusive_zone: ExclusiveZone,
        keyboard_interactivity: wlr_layer::KeyboardInteractivity,
        program: SnowcapWidgetProgram,
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

        let wgpu_surface = unsafe {
            state
                .wgpu
                .instance
                .create_surface_unsafe(SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle,
                    raw_window_handle,
                })
                .unwrap()
        };

        let surface_config = iced_wgpu::wgpu::SurfaceConfiguration {
            usage: iced_wgpu::wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: iced_wgpu::wgpu::TextureFormat::Rgba8UnormSrgb,
            width,
            height,
            present_mode: iced_wgpu::wgpu::PresentMode::Mailbox,
            desired_maximum_frame_latency: 1,
            alpha_mode: iced_wgpu::wgpu::CompositeAlphaMode::PreMultiplied,
            view_formats: vec![iced_wgpu::wgpu::TextureFormat::Rgba8UnormSrgb],
        };

        wgpu_surface.configure(&state.wgpu.device, &surface_config);

        let widgets = iced_runtime::program::State::new(
            program,
            [width as f32, height as f32].into(),
            &mut state.wgpu.renderer,
            &mut iced_runtime::Debug::new(),
        );

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
                            layer.widgets.queue_message(msg);
                        }
                    }
                }
                calloop::channel::Event::Closed => (),
            })
            .unwrap();

        // runtime.track(
        //     iced::keyboard::on_key_press(|key, _mods| {
        //         if matches!(
        //             key,
        //             iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape)
        //         ) {
        //             Some(SnowcapMessage::Close)
        //         } else {
        //             None
        //         }
        //     })
        //     .into_recipes(),
        // );

        let next_id = state.widget_id_counter.next_and_increment();

        Self {
            surface: wgpu_surface,
            layer,
            width,
            height,
            scale: 1,
            viewport: Viewport::with_physical_size(Size::new(width, height), 1.0),
            widgets,
            clipboard,
            pointer_location: None,
            runtime,
            widget_id: next_id,
            keyboard_key_sender: None,
            pointer_button_sender: None,
        }
    }

    pub fn draw(
        &self,
        device: &iced_wgpu::wgpu::Device,
        queue: &iced_wgpu::wgpu::Queue,
        renderer: &mut iced_wgpu::Renderer,
        _qh: &QueueHandle<State>,
    ) {
        let Ok(frame) = self.surface.get_current_texture() else {
            return;
        };

        let mut encoder =
            device.create_command_encoder(&iced_wgpu::wgpu::CommandEncoderDescriptor::default());

        let view = frame
            .texture
            .create_view(&iced_wgpu::wgpu::TextureViewDescriptor::default());

        {
            renderer.with_primitives(|backend, primitives| {
                backend.present::<String>(
                    device,
                    queue,
                    &mut encoder,
                    Some(iced::Color::TRANSPARENT),
                    iced_wgpu::wgpu::TextureFormat::Rgba8UnormSrgb,
                    &view,
                    primitives,
                    &self.viewport,
                    &[],
                );
            });
        }

        queue.submit(Some(encoder.finish()));

        self.layer.wl_surface().damage_buffer(
            0,
            0,
            self.width as i32 * self.scale,
            self.height as i32 * self.scale,
        );

        // self.layer
        //     .wl_surface()
        //     .frame(qh, self.layer.wl_surface().clone());

        // Does a commit
        frame.present();
    }

    pub fn update_and_draw(
        &mut self,
        device: &iced_wgpu::wgpu::Device,
        queue: &iced_wgpu::wgpu::Queue,
        renderer: &mut iced_wgpu::Renderer,
        qh: &QueueHandle<State>,
    ) {
        let cursor = match self.pointer_location {
            Some((x, y)) => iced::mouse::Cursor::Available(iced::Point {
                x: x as f32,
                y: y as f32,
            }),
            None => iced::mouse::Cursor::Unavailable,
        };
        // TODO: the command bit
        let (events, _command) = self.widgets.update(
            self.viewport.logical_size(),
            cursor,
            renderer,
            &Theme::CatppuccinFrappe,
            &iced_wgpu::core::renderer::Style {
                text_color: Color::WHITE,
            },
            &mut self.clipboard,
            &mut Debug::new(),
        );

        for event in events {
            self.runtime.broadcast(event, iced::event::Status::Ignored);
        }

        self.draw(device, queue, renderer, qh);
    }

    pub fn set_scale(&mut self, scale: i32, device: &iced_wgpu::wgpu::Device) {
        self.scale = scale;
        self.layer.wl_surface().set_buffer_scale(scale);

        let surface_config = iced_wgpu::wgpu::SurfaceConfiguration {
            usage: iced_wgpu::wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: iced_wgpu::wgpu::TextureFormat::Rgba8UnormSrgb,
            width: self.width * scale as u32,
            height: self.height * scale as u32,
            present_mode: iced_wgpu::wgpu::PresentMode::Mailbox,
            desired_maximum_frame_latency: 2,
            alpha_mode: iced_wgpu::wgpu::CompositeAlphaMode::PreMultiplied,
            view_formats: vec![iced_wgpu::wgpu::TextureFormat::Rgba8UnormSrgb],
        };

        self.surface.configure(device, &surface_config);
    }
}
