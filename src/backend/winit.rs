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
            gles::GlesRenderer,
            Bind, Blit, BufferType, ExportMem, ImportDma, ImportEgl, ImportMemWl, TextureFilter,
        },
        winit::{self, WinitEvent, WinitGraphicsBackend},
    },
    output::{Output, Scale, Subpixel},
    reexports::{
        calloop::{self, generic::Generic, Interest, LoopHandle, PostAction},
        wayland_protocols::wp::presentation_time::server::wp_presentation_feedback,
        wayland_server::{
            protocol::{wl_shm, wl_surface::WlSurface},
            DisplayHandle,
        },
        winit::{
            platform::wayland::WindowAttributesExtWayland,
            window::{Icon, WindowAttributes},
        },
    },
    utils::{Rectangle, Transform},
    wayland::{
        dmabuf::{self, DmabufFeedback, DmabufFeedbackBuilder, DmabufGlobal, DmabufState},
        presentation::Refresh,
    },
};
use tracing::{debug, error, trace, warn};

use crate::{
    output::{BlankingState, OutputMode},
    render::{
        pointer::pointer_render_elements, take_presentation_feedback, OutputRenderElement,
        CLEAR_COLOR, CLEAR_COLOR_LOCKED,
    },
    state::{Pinnacle, State, WithState},
};

use super::{Backend, BackendData, UninitBackend};

const LOGO_BYTES: &[u8] = include_bytes!("../../resources/pinnacle_logo_icon.rgba");

pub struct Winit {
    pub backend: WinitGraphicsBackend<GlesRenderer>,
    pub damage_tracker: OutputDamageTracker,
    pub dmabuf_state: (DmabufState, DmabufGlobal, Option<DmabufFeedback>),
    pub full_redraw: u8,
    output: Output,
}

impl BackendData for Winit {
    fn seat_name(&self) -> String {
        "winit".to_string()
    }

    fn reset_buffers(&mut self, _output: &Output) {
        self.full_redraw = 4;
    }

    fn early_import(&mut self, _surface: &WlSurface) {}

    fn set_output_mode(&mut self, output: &Output, mode: OutputMode) {
        output.change_current_state(Some(mode.into()), None, None, None);
    }
}

impl Backend {
    fn winit_mut(&mut self) -> &mut Winit {
        let Backend::Winit(winit) = self else { unreachable!() };
        winit
    }
}

impl Winit {
    pub(crate) fn try_new(display_handle: DisplayHandle) -> anyhow::Result<UninitBackend<Winit>> {
        let window_attrs = WindowAttributes::default()
            .with_title("Pinnacle")
            .with_name("pinnacle", "pinnacle")
            .with_window_icon(Icon::from_rgba(LOGO_BYTES.to_vec(), 64, 64).ok());

        let (mut winit_backend, winit_evt_loop) =
            match winit::init_from_attributes::<GlesRenderer>(window_attrs) {
                Ok(ret) => ret,
                Err(err) => anyhow::bail!("Failed to init winit backend: {err}"),
            };

        let mode = smithay::output::Mode {
            size: winit_backend.window_size(),
            refresh: 60_000,
        };

        let physical_properties = smithay::output::PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "Pinnacle".to_string(),
            model: "Winit Window".to_string(),
        };

        let output = Output::new("Pinnacle Window".to_string(), physical_properties);

        output.with_state_mut(|state| {
            state.debug_damage_tracker = OutputDamageTracker::from_output(&output);
        });

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
                let dmabuf_formats = winit_backend.renderer().dmabuf_formats();
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
                let dmabuf_formats = winit_backend.renderer().dmabuf_formats();
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

        winit_backend.window().set_cursor_visible(false);

        let mut winit = Winit {
            backend: winit_backend,
            damage_tracker: OutputDamageTracker::from_output(&output),
            dmabuf_state,
            full_redraw: 0,
            output,
        };

        let seat_name = winit.seat_name();

        let init = Box::new(move |pinnacle: &mut Pinnacle| {
            let output = winit.output.clone();
            let global = output.create_global::<State>(&display_handle);
            output.with_state_mut(|state| state.enabled_global_id = Some(global));

            pinnacle.output_focus_stack.set_focus(output.clone());

            pinnacle.outputs.push(output.clone());

            pinnacle
                .shm_state
                .update_formats(winit.backend.renderer().shm_formats());

            pinnacle.space.map_output(&output, (0, 0));

            let insert_ret =
                pinnacle
                    .loop_handle
                    .insert_source(winit_evt_loop, move |event, _, state| match event {
                        WinitEvent::Resized { size, scale_factor } => {
                            let mode = smithay::output::Mode {
                                size,
                                refresh: 144_000,
                            };
                            state.pinnacle.change_output_state(
                                &mut state.backend,
                                &output,
                                Some(OutputMode::Smithay(mode)),
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
                            state
                                .backend
                                .winit_mut()
                                .render_winit_window(&mut state.pinnacle);
                        }
                        WinitEvent::CloseRequested => {
                            state.pinnacle.shutdown();
                        }
                    });

            if let Err(err) = insert_ret {
                anyhow::bail!("Failed to insert winit events into event loop: {err}");
            }

            Ok(winit)
        });

        Ok(UninitBackend { seat_name, init })
    }

    /// Schedule a render on the winit window.
    pub fn schedule_render(&mut self) {
        let _span = tracy_client::span!("Winit::schedule_render");
        self.backend.window().request_redraw();
    }

    fn render_winit_window(&mut self, pinnacle: &mut Pinnacle) {
        let _span = tracy_client::span!("Winit::render_winit_window");

        let full_redraw = &mut self.full_redraw;
        *full_redraw = full_redraw.saturating_sub(1);

        let mut output_render_elements = Vec::new();

        let should_draw_cursor = !pinnacle.lock_state.is_unlocked()
            || self.output.with_state(|state| {
                // Don't draw cursor when screencopy without cursor is pending
                //  FIXME: This causes the cursor to disappear (duh)
                state.screencopies.iter().all(|sc| sc.overlay_cursor())
            });

        if should_draw_cursor {
            let pointer_location = pinnacle
                .seat
                .get_pointer()
                .map(|ptr| ptr.current_location())
                .unwrap_or((0.0, 0.0).into());

            let (pointer_render_elements, _cursor_ids) = pointer_render_elements(
                &self.output,
                self.backend.renderer(),
                &mut pinnacle.cursor_state,
                &pinnacle.space,
                pointer_location,
                pinnacle.dnd_icon.as_ref(),
                &pinnacle.clock,
            );
            output_render_elements.extend(
                pointer_render_elements
                    .into_iter()
                    .map(OutputRenderElement::from),
            );
        }

        let should_blank = pinnacle.lock_state.is_locking()
            || (pinnacle.lock_state.is_locked()
                && self.output.with_state(|state| state.lock_surface.is_none()));

        if should_blank {
            self.output.with_state_mut(|state| {
                if let BlankingState::NotBlanked = state.blanking_state {
                    debug!("Blanking output {} for session lock", self.output.name());
                    state.blanking_state = BlankingState::Blanking;
                }
            });
        } else if pinnacle.lock_state.is_locked() {
            if let Some(lock_surface) = self.output.with_state(|state| state.lock_surface.clone()) {
                let elems = render_elements_from_surface_tree(
                    self.backend.renderer(),
                    lock_surface.wl_surface(),
                    (0, 0),
                    self.output.current_scale().fractional_scale(),
                    1.0,
                    element::Kind::Unspecified,
                );

                output_render_elements.extend(elems);
            }
        } else {
            output_render_elements.extend(crate::render::output_render_elements(
                &self.output,
                self.backend.renderer(),
                &pinnacle.space,
                &pinnacle.z_index_stack,
            ));
        }

        if pinnacle.config.debug.visualize_opaque_regions {
            crate::render::util::render_opaque_regions(
                &mut output_render_elements,
                smithay::utils::Scale::from(self.output.current_scale().fractional_scale()),
            );
        }

        if pinnacle.config.debug.visualize_damage {
            let damage_elements = self.output.with_state_mut(|state| {
                crate::render::util::render_damage_from_elements(
                    &mut state.debug_damage_tracker,
                    &output_render_elements,
                    [0.3, 0.0, 0.0, 0.3].into(),
                )
            });
            output_render_elements = damage_elements
                .into_iter()
                .map(From::from)
                .chain(output_render_elements)
                .collect();
        }

        // FIXME: always errors and returns none, https://github.com/Smithay/smithay/issues/1672
        // let age = if *full_redraw > 0 {
        //     0
        // } else {
        //     self.backend.buffer_age().unwrap_or(0)
        // };
        let age = 0;

        let render_res = self.backend.bind().and_then(|(renderer, mut framebuffer)| {
            let clear_color = if pinnacle.lock_state.is_unlocked() {
                CLEAR_COLOR
            } else {
                CLEAR_COLOR_LOCKED
            };

            self.damage_tracker
                .render_output(
                    renderer,
                    &mut framebuffer,
                    age,
                    &output_render_elements,
                    clear_color,
                )
                .map_err(|err| match err {
                    damage::Error::Rendering(err) => err.into(),
                    damage::Error::OutputNoMode(_) => panic!("winit output has no mode set"),
                })
        });

        match render_res {
            Ok(render_output_result) => {
                let has_rendered = render_output_result.damage.is_some();

                match self
                    .backend
                    .submit(render_output_result.damage.map(|damage| damage.as_slice()))
                {
                    Ok(()) => {
                        if has_rendered {
                            self.output.with_state_mut(|state| {
                                if matches!(state.blanking_state, BlankingState::Blanking) {
                                    // TODO: this is probably wrong
                                    debug!("Output {} blanked", self.output.name());
                                    state.blanking_state = BlankingState::Blanked;
                                }
                            });
                        }
                    }
                    Err(err) => {
                        error!("Failed to submit buffer: {:?}", err);
                    }
                }

                if pinnacle.lock_state.is_unlocked() {
                    Winit::handle_pending_screencopy(
                        &mut self.backend,
                        &self.output,
                        &render_output_result,
                        &pinnacle.loop_handle,
                    );
                }

                let now = pinnacle.clock.now();

                pinnacle.update_primary_scanout_output(&self.output, &render_output_result.states);

                if has_rendered {
                    let mut output_presentation_feedback = take_presentation_feedback(
                        &self.output,
                        &pinnacle.space,
                        &render_output_result.states,
                    );
                    output_presentation_feedback.presented(
                        now,
                        self.output
                            .current_mode()
                            .map(|mode| {
                                Refresh::Fixed(Duration::from_secs_f64(
                                    1000f64 / mode.refresh as f64,
                                ))
                            })
                            .unwrap_or(Refresh::Unknown),
                        0,
                        wp_presentation_feedback::Kind::Vsync,
                    );
                }
            }
            Err(err) => {
                warn!("{}", err);
            }
        };

        pinnacle.send_frame_callbacks(&self.output, None);

        // At the end cuz borrow checker
        if pinnacle.cursor_state.is_current_cursor_animated() {
            self.schedule_render();
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
        let _span = tracy_client::span!("Winit::handle_pending_screencopy");

        let screencopies =
            output.with_state_mut(|state| state.screencopies.drain(..).collect::<Vec<_>>());
        for mut screencopy in screencopies {
            assert_eq!(screencopy.output(), output);

            if screencopy.with_damage() {
                match render_output_result.damage.as_ref() {
                    Some(damage) if !damage.is_empty() => screencopy.damage(damage),
                    _ => {
                        output.with_state_mut(|state| state.screencopies.push(screencopy));
                        continue;
                    }
                }
            }

            let sync_point = if let Ok(mut dmabuf) =
                dmabuf::get_dmabuf(screencopy.buffer()).cloned()
            {
                trace!("Dmabuf screencopy");

                let current = backend.bind();

                current
                    .and_then(|(renderer, current_fb)| {
                        let mut dmabuf_fb = renderer.bind(&mut dmabuf)?;

                        Ok(renderer.blit(
                            &current_fb,
                            &mut dmabuf_fb,
                            screencopy.physical_region(),
                            Rectangle::from_size(screencopy.physical_region().size),
                            TextureFilter::Nearest,
                        )?)
                    })
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
                    let screencopy = &screencopy;
                    if !matches!(buffer_type(screencopy.buffer()), Some(BufferType::Shm)) {
                        warn!("screencopy does not have a shm buffer");
                        continue;
                    }

                    let res = smithay::wayland::shm::with_buffer_contents_mut(
                        &screencopy.buffer().clone(),
                        |shm_ptr, shm_len, buffer_data| {
                            // yoinked from Niri (thanks yall)
                            ensure!(
                                // The buffer prefers pixels in little endian ...
                                buffer_data.format == wl_shm::Format::Argb8888
                                    && buffer_data.stride
                                        == screencopy.physical_region().size.w * 4
                                    && buffer_data.height == screencopy.physical_region().size.h
                                    && shm_len as i32 == buffer_data.stride * buffer_data.height,
                                "invalid buffer format or size"
                            );

                            let buffer_rect = screencopy.physical_region().to_logical(1).to_buffer(
                                1,
                                Transform::Normal,
                                &screencopy.physical_region().size.to_logical(1),
                            );

                            let (renderer, current_fb) = backend.bind()?;

                            let mapping = renderer.copy_framebuffer(
                                &current_fb,
                                Rectangle::from_size(buffer_rect.size),
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

                sync_point
            };

            match sync_point {
                Ok(sync_point) if !sync_point.is_reached() => {
                    let Some(sync_fd) = sync_point.export() else {
                        screencopy.submit(false);
                        continue;
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
}
