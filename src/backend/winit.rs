// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::Duration;

use anyhow::{anyhow, ensure};
use smithay::{
    backend::{
        egl::EGLDevice,
        renderer::{
            self, buffer_type,
            damage::{self, OutputDamageTracker, RenderOutputResult},
            element::{self, surface::render_elements_from_surface_tree},
            gles::{GlesRenderbuffer, GlesRenderer, GlesTexture},
            Bind, Blit, BufferType, ExportMem, ImportDma, ImportEgl, ImportMemWl, Offscreen,
            TextureFilter,
        },
        winit::{self, WinitEvent, WinitGraphicsBackend},
    },
    input::pointer::CursorImageStatus,
    output::{Output, Scale, Subpixel},
    reexports::{
        calloop::{
            self,
            generic::Generic,
            timer::{TimeoutAction, Timer},
            Interest, LoopHandle, PostAction,
        },
        wayland_protocols::wp::presentation_time::server::wp_presentation_feedback,
        wayland_server::{
            protocol::{wl_shm, wl_surface::WlSurface},
            DisplayHandle,
        },
        winit::{
            platform::{pump_events::PumpStatus, wayland::WindowBuilderExtWayland},
            window::{Icon, WindowBuilder},
        },
    },
    utils::{IsAlive, Point, Rectangle, Transform},
    wayland::dmabuf::{self, DmabufFeedback, DmabufFeedbackBuilder, DmabufGlobal, DmabufState},
};
use tracing::{debug, error, trace, warn};

use crate::{
    output::BlankingState,
    render::{pointer::PointerElement, pointer_render_elements, take_presentation_feedback},
    state::{Pinnacle, State, WithState},
};

use super::{Backend, BackendData, UninitBackend};

const LOGO_BYTES: &[u8] = include_bytes!("../../resources/pinnacle_logo_icon.rgba");

pub struct Winit {
    pub backend: WinitGraphicsBackend<GlesRenderer>,
    pub damage_tracker: OutputDamageTracker,
    pub dmabuf_state: (DmabufState, DmabufGlobal, Option<DmabufFeedback>),
    pub full_redraw: u8,
}

impl BackendData for Winit {
    fn seat_name(&self) -> String {
        "winit".to_string()
    }

    fn reset_buffers(&mut self, _output: &Output) {
        self.full_redraw = 4;
    }

    fn early_import(&mut self, _surface: &WlSurface) {}
}

impl Backend {
    fn winit_mut(&mut self) -> &mut Winit {
        let Backend::Winit(winit) = self else { unreachable!() };
        winit
    }
}

impl Winit {
    pub(crate) fn try_new(display_handle: DisplayHandle) -> anyhow::Result<UninitBackend<Winit>> {
        let window_builder = WindowBuilder::new()
            .with_title("Pinnacle")
            .with_name("pinnacle", "pinnacle")
            .with_window_icon(Icon::from_rgba(LOGO_BYTES.to_vec(), 64, 64).ok());

        let (mut winit_backend, mut winit_evt_loop) =
            match winit::init_from_builder::<GlesRenderer>(window_builder) {
                Ok(ret) => ret,
                Err(err) => anyhow::bail!("Failed to init winit backend: {err}"),
            };

        let mode = smithay::output::Mode {
            size: winit_backend.window_size(),
            refresh: 144_000,
        };

        let physical_properties = smithay::output::PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "Pinnacle".to_string(),
            model: "Winit Window".to_string(),
        };

        let output = Output::new("Pinnacle Window".to_string(), physical_properties);

        output.change_current_state(
            Some(mode),
            Some(Transform::Flipped180),
            None,
            Some((0, 0).into()),
        );

        output.set_preferred(mode);
        output.with_state_mut(|state| state.modes = vec![mode]);

        let render_node =
            EGLDevice::device_for_display(winit_backend.renderer().egl_context().display())
                .and_then(|device| device.try_get_render_node());

        let dmabuf_default_feedback = match render_node {
            Ok(Some(node)) => {
                let dmabuf_formats = winit_backend
                    .renderer()
                    .dmabuf_formats()
                    .collect::<Vec<_>>();
                let dmabuf_default_feedback =
                    DmabufFeedbackBuilder::new(node.dev_id(), dmabuf_formats)
                        .build()
                        .expect("DmabufFeedbackBuilder error");
                Some(dmabuf_default_feedback)
            }
            Ok(None) => {
                warn!("failed to query render node, dmabuf will use v3");
                None
            }
            Err(err) => {
                warn!("{}", err);
                None
            }
        };

        let dmabuf_state = match dmabuf_default_feedback {
            Some(default_feedback) => {
                let mut dmabuf_state = DmabufState::new();
                let dmabuf_global = dmabuf_state.create_global_with_default_feedback::<State>(
                    &display_handle,
                    &default_feedback,
                );
                (dmabuf_state, dmabuf_global, Some(default_feedback))
            }
            None => {
                let dmabuf_formats = winit_backend
                    .renderer()
                    .dmabuf_formats()
                    .collect::<Vec<_>>();
                let mut dmabuf_state = DmabufState::new();
                let dmabuf_global =
                    dmabuf_state.create_global::<State>(&display_handle, dmabuf_formats);
                (dmabuf_state, dmabuf_global, None)
            }
        };

        if winit_backend
            .renderer()
            .bind_wl_display(&display_handle)
            .is_ok()
        {
            tracing::info!("EGL hardware-acceleration enabled");
        }

        let mut winit = Winit {
            backend: winit_backend,
            damage_tracker: OutputDamageTracker::from_output(&output),
            dmabuf_state,
            full_redraw: 0,
        };

        Ok(UninitBackend {
            seat_name: winit.seat_name(),
            init: Box::new(move |pinnacle: &mut Pinnacle| {
                output.create_global::<State>(&display_handle);

                pinnacle.output_focus_stack.set_focus(output.clone());

                pinnacle
                    .shm_state
                    .update_formats(winit.backend.renderer().shm_formats());

                pinnacle.space.map_output(&output, (0, 0));

                let insert_ret = pinnacle.loop_handle.insert_source(
                    Timer::immediate(),
                    move |_instant, _metadata, state| {
                        let status = winit_evt_loop.dispatch_new_events(|event| match event {
                            WinitEvent::Resized { size, scale_factor } => {
                                let mode = smithay::output::Mode {
                                    size,
                                    refresh: 144_000,
                                };
                                state.pinnacle.change_output_state(
                                    &output,
                                    Some(mode),
                                    None,
                                    Some(Scale::Fractional(scale_factor)),
                                    // None,
                                    None,
                                );
                                state.pinnacle.request_layout(&output);
                            }
                            WinitEvent::Focus(focused) => {
                                if focused {
                                    state.backend.winit_mut().reset_buffers(&output);
                                }
                            }
                            WinitEvent::Input(input_evt) => {
                                state.process_input_event(input_evt);
                            }
                            WinitEvent::Redraw => {
                                state.render_winit_window(&output);
                            }
                            WinitEvent::CloseRequested => {
                                state.pinnacle.shutdown();
                            }
                        });

                        if let PumpStatus::Exit(_) = status {
                            state.pinnacle.shutdown();
                        }

                        state.render_winit_window(&output);

                        TimeoutAction::ToDuration(Duration::from_micros(
                            ((1.0 / 144.0) * 1000000.0) as u64,
                        ))
                    },
                );
                if let Err(err) = insert_ret {
                    anyhow::bail!("Failed to insert winit events into event loop: {err}");
                }

                Ok(winit)
            }),
        })
    }
}

impl State {
    fn render_winit_window(&mut self, output: &Output) {
        let winit = self.backend.winit_mut();

        let full_redraw = &mut winit.full_redraw;
        *full_redraw = full_redraw.saturating_sub(1);

        if let CursorImageStatus::Surface(surface) = &self.pinnacle.cursor_status {
            if !surface.alive() {
                self.pinnacle.cursor_status = CursorImageStatus::default_named();
            }
        }

        let cursor_visible = !matches!(self.pinnacle.cursor_status, CursorImageStatus::Surface(_));

        let mut pointer_element = PointerElement::<GlesTexture>::new();

        pointer_element.set_status(self.pinnacle.cursor_status.clone());

        // The z-index of these is determined by `state.fixup_z_layering()`, which is called at the end
        // of every event loop cycle
        let windows = self.pinnacle.space.elements().cloned().collect::<Vec<_>>();

        let mut output_render_elements = Vec::new();

        let should_blank = self.pinnacle.lock_state.is_locking()
            || (self.pinnacle.lock_state.is_locked()
                && output.with_state(|state| state.lock_surface.is_none()));

        let should_draw_cursor = !self.pinnacle.lock_state.is_unlocked()
            || output.with_state(|state| {
                // Don't draw cursor when screencopy without cursor is pending
                !state
                    .screencopy
                    .as_ref()
                    .is_some_and(|sc| !sc.overlay_cursor())
            });

        if should_draw_cursor {
            let pointer_location = self
                .pinnacle
                .seat
                .get_pointer()
                .map(|ptr| ptr.current_location())
                .unwrap_or((0.0, 0.0).into());

            let pointer_render_elements = pointer_render_elements(
                output,
                winit.backend.renderer(),
                &self.pinnacle.space,
                pointer_location,
                &mut self.pinnacle.cursor_status,
                self.pinnacle.dnd_icon.as_ref(),
                &pointer_element,
            );
            output_render_elements.extend(pointer_render_elements);
        }

        if should_blank {
            // Don't push any render elements and we get a blank frame
            output.with_state_mut(|state| {
                if let BlankingState::NotBlanked = state.blanking_state {
                    debug!("Blanking output {} for session lock", output.name());
                    state.blanking_state = BlankingState::Blanking;
                }
            });
        } else if let Some(lock_surface) = output.with_state(|state| state.lock_surface.clone()) {
            let elems = render_elements_from_surface_tree(
                winit.backend.renderer(),
                lock_surface.wl_surface(),
                (0, 0),
                output.current_scale().fractional_scale(),
                1.0,
                element::Kind::Unspecified,
            );

            output_render_elements.extend(elems);
        } else {
            output_render_elements.extend(crate::render::output_render_elements(
                output,
                winit.backend.renderer(),
                &self.pinnacle.space,
                &windows,
            ));
        }

        let render_res = winit.backend.bind().and_then(|_| {
            let age = if *full_redraw > 0 {
                0
            } else {
                winit.backend.buffer_age().unwrap_or(0)
            };

            let renderer = winit.backend.renderer();

            winit
                .damage_tracker
                .render_output(renderer, age, &output_render_elements, [0.6, 0.6, 0.6, 1.0])
                .map_err(|err| match err {
                    damage::Error::Rendering(err) => err.into(),
                    damage::Error::OutputNoMode(_) => panic!("winit output has no mode set"),
                })
        });

        match render_res {
            Ok(render_output_result) => {
                if self.pinnacle.lock_state.is_unlocked() {
                    Winit::handle_pending_screencopy(
                        &mut winit.backend,
                        output,
                        &render_output_result,
                        &self.pinnacle.loop_handle,
                    );
                }

                let has_rendered = render_output_result.damage.is_some();
                if let Some(damage) = render_output_result.damage {
                    match winit.backend.submit(Some(damage)) {
                        Ok(()) => {
                            output.with_state_mut(|state| {
                                if matches!(state.blanking_state, BlankingState::Blanking) {
                                    // TODO: this is probably wrong
                                    debug!("Output {} blanked", output.name());
                                    state.blanking_state = BlankingState::Blanked;
                                }
                            });
                        }
                        Err(err) => {
                            error!("Failed to submit buffer: {}", err);
                        }
                    }
                }

                winit.backend.window().set_cursor_visible(cursor_visible);

                let time = self.pinnacle.clock.now();

                super::post_repaint(
                    output,
                    &render_output_result.states,
                    &self.pinnacle.space,
                    None,
                    time.into(),
                    &self.pinnacle.cursor_status,
                );

                if has_rendered {
                    let mut output_presentation_feedback = take_presentation_feedback(
                        output,
                        &self.pinnacle.space,
                        &render_output_result.states,
                    );
                    output_presentation_feedback.presented(
                        time,
                        output
                            .current_mode()
                            .map(|mode| Duration::from_secs_f64(1000f64 / mode.refresh as f64))
                            .unwrap_or_default(),
                        0,
                        wp_presentation_feedback::Kind::Vsync,
                    );
                }
            }
            Err(err) => {
                warn!("{}", err);
            }
        }
    }
}

impl Winit {
    fn handle_pending_screencopy(
        backend: &mut WinitGraphicsBackend<GlesRenderer>,
        output: &Output,
        render_output_result: &RenderOutputResult,
        loop_handle: &LoopHandle<'static, State>,
    ) {
        let Some(mut screencopy) = output.with_state_mut(|state| state.screencopy.take()) else {
            return;
        };

        assert!(screencopy.output() == output);

        if screencopy.with_damage() {
            match render_output_result.damage.as_ref() {
                Some(damage) if !damage.is_empty() => screencopy.damage(damage),
                _ => {
                    output.with_state_mut(|state| state.screencopy.replace(screencopy));
                    return;
                }
            }
        }

        let sync_point = if let Ok(dmabuf) = dmabuf::get_dmabuf(screencopy.buffer()) {
            trace!("Dmabuf screencopy");

            backend
                .renderer()
                .blit_to(
                    dmabuf,
                    screencopy.physical_region(),
                    Rectangle::from_loc_and_size(
                        Point::from((0, 0)),
                        screencopy.physical_region().size,
                    ),
                    TextureFilter::Nearest,
                )
                .map(|_| render_output_result.sync.clone())
                .map_err(|err| anyhow!("{err}"))
        } else if !matches!(
            renderer::buffer_type(screencopy.buffer()),
            Some(BufferType::Shm)
        ) {
            Err(anyhow!("not a shm buffer"))
        } else {
            trace!("Shm screencopy");

            let sync_point = {
                let renderer = backend.renderer();
                let screencopy = &screencopy;
                if !matches!(buffer_type(screencopy.buffer()), Some(BufferType::Shm)) {
                    warn!("screencopy does not have a shm buffer");
                    return;
                }

                let res = smithay::wayland::shm::with_buffer_contents_mut(
                    &screencopy.buffer().clone(),
                    |shm_ptr, shm_len, buffer_data| {
                        // yoinked from Niri (thanks yall)
                        ensure!(
                            // The buffer prefers pixels in little endian ...
                            buffer_data.format == wl_shm::Format::Argb8888
                                && buffer_data.stride == screencopy.physical_region().size.w * 4
                                && buffer_data.height == screencopy.physical_region().size.h
                                && shm_len as i32 == buffer_data.stride * buffer_data.height,
                            "invalid buffer format or size"
                        );

                        let buffer_rect = screencopy.physical_region().to_logical(1).to_buffer(
                            1,
                            Transform::Normal,
                            &screencopy.physical_region().size.to_logical(1),
                        );

                        // On winit, we cannot just copy the EGL framebuffer because I get an
                        // `UnsupportedPixelFormat` error. Therefore we'll blit
                        // to this buffer and then copy it.
                        let offscreen: GlesRenderbuffer = renderer.create_buffer(
                            smithay::backend::allocator::Fourcc::Argb8888,
                            buffer_rect.size,
                        )?;

                        renderer.blit_to(
                            offscreen.clone(),
                            screencopy.physical_region(),
                            Rectangle::from_loc_and_size(
                                Point::from((0, 0)),
                                screencopy.physical_region().size,
                            ),
                            TextureFilter::Nearest,
                        )?;

                        renderer.bind(offscreen)?;

                        let mapping = renderer.copy_framebuffer(
                            Rectangle::from_loc_and_size(Point::from((0, 0)), buffer_rect.size),
                            smithay::backend::allocator::Fourcc::Argb8888,
                        )?;

                        let bytes = renderer.map_texture(&mapping)?;

                        ensure!(bytes.len() == shm_len, "mapped buffer has wrong length");

                        // SAFETY:
                        //      - `bytes.as_ptr()` is valid for reads of size `shm_len` because that was
                        //        checked above and is properly aligned because it
                        //        originated from safe Rust
                        //      - We are assuming `shm_ptr` is valid for writes of `shm_len` and is
                        //        properly aligned
                        //      - Overlapping-ness: TODO:
                        unsafe {
                            std::ptr::copy_nonoverlapping(bytes.as_ptr(), shm_ptr, shm_len);
                        }

                        Ok(())
                    },
                );

                let Ok(res) = res else {
                    unreachable!(
                        "buffer is guaranteed to be shm from above and managed by smithay"
                    );
                };

                res
            }
            .map(|_| render_output_result.sync.clone());

            // We must rebind to the underlying EGL surface for buffer swapping
            // as it is bound to a `GlesRenderbuffer` above.
            if let Err(err) = backend.bind() {
                error!("Failed to rebind EGL surface after screencopy: {err}");
            }

            sync_point
        };

        match sync_point {
            Ok(sync_point) if !sync_point.is_reached() => {
                let Some(sync_fd) = sync_point.export() else {
                    screencopy.submit(false);
                    return;
                };
                let mut screencopy = Some(screencopy);
                let source = Generic::new(sync_fd, Interest::READ, calloop::Mode::OneShot);
                let res = loop_handle.insert_source(source, move |_, _, _| {
                    let Some(screencopy) = screencopy.take() else {
                        unreachable!("This source is removed after one run");
                    };
                    screencopy.submit(false);
                    trace!("Submitted screencopy");
                    Ok(PostAction::Remove)
                });
                if res.is_err() {
                    error!("Failed to schedule screencopy submission");
                }
            }
            Ok(_) => screencopy.submit(false),
            Err(err) => error!("Failed to submit screencopy: {err}"),
        }
    }
}
