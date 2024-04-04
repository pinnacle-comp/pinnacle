// SPDX-License-Identifier: GPL-3.0-or-later

use std::{ffi::OsString, path::PathBuf, time::Duration};

use anyhow::anyhow;
use smithay::{
    backend::{
        egl::EGLDevice,
        renderer::{
            self,
            damage::{self, OutputDamageTracker},
            gles::{GlesRenderer, GlesTexture},
            Blit, BufferType, ImportDma, ImportEgl, ImportMemWl, TextureFilter,
        },
        winit::{self, WinitEvent, WinitGraphicsBackend},
    },
    desktop::layer_map_for_output,
    input::pointer::CursorImageStatus,
    output::{Output, Scale, Subpixel},
    reexports::{
        calloop::{
            self,
            generic::Generic,
            timer::{TimeoutAction, Timer},
            EventLoop, Interest, PostAction,
        },
        wayland_protocols::wp::presentation_time::server::wp_presentation_feedback,
        wayland_server::{protocol::wl_surface::WlSurface, Display},
        winit::{
            platform::pump_events::PumpStatus,
            window::{Icon, WindowBuilder},
        },
    },
    utils::{IsAlive, Point, Rectangle, Transform},
    wayland::dmabuf::{self, DmabufFeedback, DmabufFeedbackBuilder, DmabufGlobal, DmabufState},
};
use tracing::{error, info};

use crate::{
    render::{
        copy_to_shm_screencopy, generate_render_elements, pointer::PointerElement,
        pointer_render_elements, take_presentation_feedback,
    },
    state::{State, WithState},
};

use super::{Backend, BackendData};

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

/// Start Pinnacle as a window in a graphical environment.
pub fn setup_winit(
    no_config: bool,
    config_dir: Option<PathBuf>,
) -> anyhow::Result<(State, EventLoop<'static, State>)> {
    let event_loop: EventLoop<State> = EventLoop::try_new()?;

    let display: Display<State> = Display::new()?;
    let display_handle = display.handle();

    let loop_handle = event_loop.handle();

    let window_builder = WindowBuilder::new()
        .with_title("Pinnacle")
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

    let output = Output::new("Pinnacle window".to_string(), physical_properties);

    output.create_global::<State>(&display_handle);

    output.change_current_state(
        Some(mode),
        Some(Transform::Flipped180),
        None,
        Some((0, 0).into()),
    );

    output.set_preferred(mode);

    let render_node =
        EGLDevice::device_for_display(winit_backend.renderer().egl_context().display())
            .and_then(|device| device.try_get_render_node());

    let dmabuf_default_feedback = match render_node {
        Ok(Some(node)) => {
            let dmabuf_formats = winit_backend
                .renderer()
                .dmabuf_formats()
                .collect::<Vec<_>>();
            let dmabuf_default_feedback = DmabufFeedbackBuilder::new(node.dev_id(), dmabuf_formats)
                .build()
                .expect("DmabufFeedbackBuilder error");
            Some(dmabuf_default_feedback)
        }
        Ok(None) => {
            tracing::warn!("failed to query render node, dmabuf will use v3");
            None
        }
        Err(err) => {
            tracing::warn!("{}", err);
            None
        }
    };

    let dmabuf_state = match dmabuf_default_feedback {
        Some(default_feedback) => {
            let mut dmabuf_state = DmabufState::new();
            let dmabuf_global = dmabuf_state
                .create_global_with_default_feedback::<State>(&display_handle, &default_feedback);
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

    let backend = Backend::Winit(Winit {
        backend: winit_backend,
        damage_tracker: OutputDamageTracker::from_output(&output),
        dmabuf_state,
        full_redraw: 0,
    });

    let mut state = State::init(
        backend,
        display,
        event_loop.get_signal(),
        loop_handle,
        no_config,
        config_dir,
    )?;

    state.output_focus_stack.set_focus(output.clone());

    let winit = state.backend.winit_mut();

    state
        .shm_state
        .update_formats(winit.backend.renderer().shm_formats());

    state.space.map_output(&output, (0, 0));

    if let Err(err) = state.xwayland.start(
        state.loop_handle.clone(),
        None,
        std::iter::empty::<(OsString, OsString)>(),
        true,
        |_| {},
    ) {
        error!("Failed to start XWayland: {err}");
    }

    let insert_ret =
        state
            .loop_handle
            .insert_source(Timer::immediate(), move |_instant, _metadata, state| {
                let status = winit_evt_loop.dispatch_new_events(|event| match event {
                    WinitEvent::Resized { size, scale_factor } => {
                        let mode = smithay::output::Mode {
                            size,
                            refresh: 144_000,
                        };
                        output.change_current_state(
                            Some(mode),
                            None,
                            Some(Scale::Fractional(scale_factor)),
                            None,
                        );
                        layer_map_for_output(&output).arrange();
                        state.request_layout(&output);
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
                        state.shutdown();
                    }
                });

                if let PumpStatus::Exit(_) = status {
                    state.shutdown();
                }

                state.render_winit_window(&output);

                TimeoutAction::ToDuration(Duration::from_micros(((1.0 / 144.0) * 1000000.0) as u64))
            });
    if let Err(err) = insert_ret {
        anyhow::bail!("Failed to insert winit events into event loop: {err}");
    }

    Ok((state, event_loop))
}

impl State {
    fn render_winit_window(&mut self, output: &Output) {
        let winit = self.backend.winit_mut();

        let full_redraw = &mut winit.full_redraw;
        *full_redraw = full_redraw.saturating_sub(1);

        if let CursorImageStatus::Surface(surface) = &self.cursor_status {
            if !surface.alive() {
                self.cursor_status = CursorImageStatus::default_named();
            }
        }

        let cursor_visible = !matches!(self.cursor_status, CursorImageStatus::Surface(_));

        let mut pointer_element = PointerElement::<GlesTexture>::new();

        pointer_element.set_status(self.cursor_status.clone());

        // The z-index of these is determined by `state.fixup_z_layering()`, which is called at the end
        // of every event loop cycle
        let windows = self.space.elements().cloned().collect::<Vec<_>>();

        let mut output_render_elements = Vec::new();

        let pending_screencopy_without_cursor = output.with_state(|state| {
            state
                .screencopy
                .as_ref()
                .is_some_and(|sc| !sc.overlay_cursor())
        });

        // If there isn't a pending screencopy that doesn't want to overlay the cursor,
        // render it.
        //
        // This will cause the cursor to disappear for a frame if there is one though,
        // but it shouldn't meaningfully affect anything.
        if !pending_screencopy_without_cursor {
            let pointer_location = self
                .seat
                .get_pointer()
                .map(|ptr| ptr.current_location())
                .unwrap_or((0.0, 0.0).into());

            let pointer_render_elements = pointer_render_elements(
                output,
                winit.backend.renderer(),
                &self.space,
                pointer_location,
                &mut self.cursor_status,
                self.dnd_icon.as_ref(),
                &pointer_element,
            );
            output_render_elements.extend(pointer_render_elements);
        }

        output_render_elements.extend(generate_render_elements(
            output,
            winit.backend.renderer(),
            &self.space,
            &windows,
        ));

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
                'screencopy: {
                    // If there is a pending screencopy, copy to it
                    if let Some(mut screencopy) =
                        output.with_state_mut(|state| state.screencopy.take())
                    {
                        assert!(screencopy.output() == output);

                        if screencopy.with_damage() {
                            if let Some(damage) = render_output_result.damage.as_ref() {
                                let damage = damage
                                    .iter()
                                    .flat_map(|rect| {
                                        rect.intersection(screencopy.physical_region())
                                    })
                                    .collect::<Vec<_>>();
                                if damage.is_empty() {
                                    output.with_state_mut(|state| {
                                        state.screencopy.replace(screencopy)
                                    });
                                    break 'screencopy;
                                }
                                screencopy.damage(&damage);
                            } else {
                                output.with_state_mut(|state| state.screencopy.replace(screencopy));
                                break 'screencopy;
                            }
                        }

                        let sync_fd = if let Ok(dmabuf) = dmabuf::get_dmabuf(screencopy.buffer()) {
                            info!("Dmabuf screencopy");

                            winit
                                .backend
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
                                .map(|_| render_output_result.sync.export())
                                .map_err(|err| anyhow!("{err}"))
                        } else if !matches!(
                            renderer::buffer_type(screencopy.buffer()),
                            Some(BufferType::Shm)
                        ) {
                            Err(anyhow!("not a shm buffer"))
                        } else {
                            info!("Shm screencopy");

                            let res = copy_to_shm_screencopy(winit.backend.renderer(), &screencopy)
                                .map(|_| render_output_result.sync.export());

                            // We must rebind to the underlying EGL surface for buffer swapping
                            // as it is bound in `copy_to_shm_screencopy` above.
                            let _ = winit.backend.bind();

                            res
                        };

                        match sync_fd {
                            Ok(Some(sync_fd)) => {
                                let mut screencopy = Some(screencopy);
                                let source =
                                    Generic::new(sync_fd, Interest::READ, calloop::Mode::OneShot);
                                let res = self.loop_handle.insert_source(source, move |_, _, _| {
                                    let Some(screencopy) = screencopy.take() else {
                                        unreachable!("This source is removed after one run");
                                    };
                                    screencopy.submit(false);
                                    Ok(PostAction::Remove)
                                });
                                if res.is_err() {
                                    error!("Failed to schedule screencopy submission");
                                }
                            }
                            Ok(None) => {
                                screencopy.submit(false);
                            }
                            Err(err) => error!("Failed to submit screencopy: {err}"),
                        }
                    }
                }

                let has_rendered = render_output_result.damage.is_some();
                if let Some(damage) = render_output_result.damage {
                    if let Err(err) = winit.backend.submit(Some(&damage)) {
                        error!("Failed to submit buffer: {}", err);
                    }
                }

                winit.backend.window().set_cursor_visible(cursor_visible);

                let time = self.clock.now();

                super::post_repaint(
                    output,
                    &render_output_result.states,
                    &self.space,
                    None,
                    time.into(),
                    &self.cursor_status,
                );

                if has_rendered {
                    let mut output_presentation_feedback = take_presentation_feedback(
                        output,
                        &self.space,
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
                tracing::warn!("{}", err);
            }
        }
    }
}
