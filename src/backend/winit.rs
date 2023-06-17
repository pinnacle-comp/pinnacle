use std::{error::Error, sync::Mutex, time::Duration};

use smithay::{
    backend::{
        allocator::dmabuf::Dmabuf,
        egl::EGLDevice,
        renderer::{
            damage::{self, OutputDamageTracker},
            element::{
                default_primary_scanout_output_compare, surface::WaylandSurfaceRenderElement,
                AsRenderElements,
            },
            gles::{GlesRenderer, GlesTexture},
            ImportDma, ImportMemWl,
        },
        winit::{WinitError, WinitEvent, WinitGraphicsBackend},
    },
    delegate_dmabuf,
    desktop::{
        space,
        utils::{surface_primary_scanout_output, update_surface_primary_scanout_output},
    },
    input::pointer::{CursorImageAttributes, CursorImageStatus},
    output::{Output, Subpixel},
    reexports::{
        calloop::{
            timer::{TimeoutAction, Timer},
            EventLoop,
        },
        wayland_server::{protocol::wl_surface::WlSurface, Display},
    },
    utils::{IsAlive, Scale, Transform},
    wayland::{
        compositor::{self},
        dmabuf::{
            DmabufFeedback, DmabufFeedbackBuilder, DmabufGlobal, DmabufHandler, DmabufState,
            ImportError,
        },
        fractional_scale::with_fractional_scale,
    },
};

use crate::{
    layout::{Direction, Layout},
    render::{pointer::PointerElement, CustomRenderElements, OutputRenderElements},
    state::{CalloopData, State},
};

use super::Backend;

pub struct WinitData {
    pub backend: WinitGraphicsBackend<GlesRenderer>,
    pub damage_tracker: OutputDamageTracker,
    pub dmabuf_state: (DmabufState, DmabufGlobal, Option<DmabufFeedback>),
    pub full_redraw: u8,
}

impl Backend for WinitData {
    fn seat_name(&self) -> String {
        "winit".to_string()
    }

    fn reset_buffers(&mut self, _output: &Output) {
        self.full_redraw = 4;
    }

    fn early_import(&mut self, _surface: &WlSurface) {}
}

impl DmabufHandler for State<WinitData> {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.backend_data.dmabuf_state.0
    }

    fn dmabuf_imported(
        &mut self,
        _global: &DmabufGlobal,
        dmabuf: Dmabuf,
    ) -> Result<(), ImportError> {
        self.backend_data
            .backend
            .renderer()
            .import_dmabuf(&dmabuf, None)
            .map(|_| ())
            .map_err(|_| ImportError::Failed)
    }
}
delegate_dmabuf!(State<WinitData>);

pub fn run_winit() -> Result<(), Box<dyn Error>> {
    let mut event_loop: EventLoop<CalloopData<WinitData>> = EventLoop::try_new()?;

    let mut display: Display<State<WinitData>> = Display::new()?;
    let display_handle = display.handle();

    let evt_loop_handle = event_loop.handle();

    let (mut winit_backend, mut winit_evt_loop) = smithay::backend::winit::init::<GlesRenderer>()?;

    let mode = smithay::output::Mode {
        size: winit_backend.window_size().physical_size,
        refresh: 144_000,
    };

    let physical_properties = smithay::output::PhysicalProperties {
        size: (0, 0).into(),
        subpixel: Subpixel::Unknown,
        make: "Comp make".to_string(),
        model: "Comp model".to_string(),
    };

    let output = Output::new("27GL83A".to_string(), physical_properties);

    output.create_global::<State<WinitData>>(&display_handle);

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
                .unwrap();
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
                .create_global_with_default_feedback::<State<WinitData>>(
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
                dmabuf_state.create_global::<State<WinitData>>(&display_handle, dmabuf_formats);
            (dmabuf_state, dmabuf_global, None)
        }
    };

    let mut state = State::init(
        WinitData {
            backend: winit_backend,
            damage_tracker: OutputDamageTracker::from_output(&output),
            dmabuf_state,
            full_redraw: 0,
        },
        &mut display,
        event_loop.get_signal(),
        evt_loop_handle,
    )?;

    // std::process::Command::new("lua")
    //     .arg("../pinnacle_api_lua/init.lua")
    //     .spawn()
    //     .unwrap();

    state
        .shm_state
        .update_formats(state.backend_data.backend.renderer().shm_formats());

    state.space.map_output(&output, (0, 0));

    let mut pointer_element = PointerElement::<GlesTexture>::new();

    // TODO: pointer
    state
        .loop_handle
        .insert_source(Timer::immediate(), move |_instant, _metadata, data| {
            let display = &mut data.display;
            let state = &mut data.state;

            let result = winit_evt_loop.dispatch_new_events(|event| match event {
                WinitEvent::Resized {
                    size,
                    scale_factor: _,
                } => {
                    output.change_current_state(
                        Some(smithay::output::Mode {
                            size,
                            refresh: 144_000,
                        }),
                        None,
                        None,
                        None,
                    );
                    Layout::master_stack(
                        state,
                        state.space.elements().cloned().collect(),
                        Direction::Left,
                    );
                }
                WinitEvent::Focus(_) => {}
                WinitEvent::Input(input_evt) => {
                    state.process_input_event(input_evt);
                }
                WinitEvent::Refresh => {}
            });

            match result {
                Ok(_) => {}
                Err(WinitError::WindowClosed) => {
                    state.loop_signal.stop();
                }
            };

            if let CursorImageStatus::Surface(ref surface) = state.cursor_status {
                if !surface.alive() {
                    state.cursor_status = CursorImageStatus::Default;
                }
            }

            let cursor_visible = !matches!(state.cursor_status, CursorImageStatus::Surface(_));

            pointer_element.set_status(state.cursor_status.clone());

            let full_redraw = &mut state.backend_data.full_redraw;
            *full_redraw = full_redraw.saturating_sub(1);

            let scale = Scale::from(output.current_scale().fractional_scale());
            let cursor_hotspot =
                if let CursorImageStatus::Surface(ref surface) = state.cursor_status {
                    compositor::with_states(surface, |states| {
                        states
                            .data_map
                            .get::<Mutex<CursorImageAttributes>>()
                            .unwrap()
                            .lock()
                            .unwrap()
                            .hotspot
                    })
                } else {
                    (0, 0).into()
                };
            let cursor_pos = state.pointer_location - cursor_hotspot.to_f64();
            let cursor_pos_scaled = cursor_pos.to_physical(scale).to_i32_round::<i32>();

            let mut custom_render_elements = Vec::<CustomRenderElements<GlesRenderer>>::new();

            custom_render_elements.extend(pointer_element.render_elements(
                state.backend_data.backend.renderer(),
                cursor_pos_scaled,
                scale,
                1.0,
            ));

            let render_res = state.backend_data.backend.bind().and_then(|_| {
                let age = if *full_redraw > 0 {
                    0
                } else {
                    state.backend_data.backend.buffer_age().unwrap_or(0)
                };

                let renderer = state.backend_data.backend.renderer();

                // render_output()
                let space_render_elements =
                    space::space_render_elements(renderer, [&state.space], &output, 1.0).unwrap();

                let mut output_render_elements = Vec::<
                    OutputRenderElements<GlesRenderer, WaylandSurfaceRenderElement<GlesRenderer>>,
                >::new();

                output_render_elements.extend(
                    custom_render_elements
                        .into_iter()
                        .map(OutputRenderElements::from),
                );
                output_render_elements.extend(
                    space_render_elements
                        .into_iter()
                        .map(OutputRenderElements::from),
                );

                state
                    .backend_data
                    .damage_tracker
                    .render_output(renderer, age, &output_render_elements, [0.5, 0.5, 0.5, 1.0])
                    .map_err(|err| match err {
                        damage::Error::Rendering(err) => err.into(),
                        damage::Error::OutputNoMode(_) => todo!(),
                    })
            });

            match render_res {
                Ok((damage, states)) => {
                    let has_rendered = damage.is_some();
                    if let Some(damage) = damage {
                        if let Err(err) = state.backend_data.backend.submit(Some(&damage)) {
                            tracing::warn!("{}", err);
                        }
                    }

                    state
                        .backend_data
                        .backend
                        .window()
                        .set_cursor_visible(cursor_visible);

                    let throttle = Some(Duration::from_secs(1));

                    state.space.elements().for_each(|window| {
                        window.with_surfaces(|surface, states_inner| {
                            let primary_scanout_output = update_surface_primary_scanout_output(
                                surface,
                                &output,
                                states_inner,
                                &states,
                                default_primary_scanout_output_compare,
                            );

                            if let Some(output) = primary_scanout_output {
                                with_fractional_scale(states_inner, |fraction_scale| {
                                    fraction_scale.set_preferred_scale(
                                        output.current_scale().fractional_scale(),
                                    );
                                });
                            }
                        });

                        if state.space.outputs_for_element(window).contains(&output) {
                            window.send_frame(
                                &output,
                                state.clock.now(),
                                throttle,
                                surface_primary_scanout_output,
                            );
                            // TODO: dmabuf_feedback
                        }
                    });

                    if has_rendered {
                        // TODO:
                    }
                }
                Err(err) => {
                    tracing::warn!("{}", err);
                }
            }

            state.space.refresh();
            state.popup_manager.cleanup();
            display
                .flush_clients()
                .expect("failed to flush client buffers");

            TimeoutAction::ToDuration(Duration::from_millis(6))
        })?;

    event_loop.run(None, &mut CalloopData { display, state }, |_data| {})?;

    Ok(())
}
