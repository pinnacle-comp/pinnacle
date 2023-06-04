use std::{error::Error, os::fd::AsRawFd, sync::Arc, time::Duration};

use smithay::{
    backend::{
        allocator::dmabuf::Dmabuf,
        egl::EGLDevice,
        renderer::{
            damage::{self, OutputDamageTracker},
            element::default_primary_scanout_output_compare,
            gles::GlesRenderer,
            ImportDma, ImportMemWl,
        },
        winit::{WinitError, WinitEvent, WinitGraphicsBackend},
    },
    delegate_dmabuf,
    desktop::{
        space,
        utils::{surface_primary_scanout_output, update_surface_primary_scanout_output},
        Space, Window,
    },
    input::{pointer::CursorImageStatus, SeatState},
    output::{Output, Subpixel},
    reexports::{
        calloop::{
            generic::Generic,
            timer::{TimeoutAction, Timer},
            EventLoop, Interest, Mode, PostAction,
        },
        wayland_server::{protocol::wl_surface::WlSurface, Display},
    },
    utils::{Clock, Monotonic, Physical, Point, Scale, Transform},
    wayland::{
        compositor::CompositorState,
        data_device::DataDeviceState,
        dmabuf::{
            DmabufFeedback, DmabufFeedbackBuilder, DmabufGlobal, DmabufHandler, DmabufState,
            ImportError,
        },
        fractional_scale::{with_fractional_scale, FractionalScaleManagerState},
        output::OutputManagerState,
        shell::xdg::XdgShellState,
        shm::ShmState,
        socket::ListeningSocketSource,
        viewporter::ViewporterState,
    },
};

use crate::state::{CalloopData, ClientState, State};

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
    let mut event_loop: EventLoop<CalloopData> = EventLoop::try_new()?;

    let mut display: Display<State<WinitData>> = Display::new()?;

    let socket = ListeningSocketSource::new_auto()?;
    let socket_name = socket.socket_name().to_os_string();

    let evt_loop_handle = event_loop.handle();

    evt_loop_handle.insert_source(socket, |stream, _metadata, data| {
        data.display
            .handle()
            .insert_client(stream, Arc::new(ClientState::default()))
            .unwrap();
    })?;

    evt_loop_handle.insert_source(
        Generic::new(
            display.backend().poll_fd().as_raw_fd(),
            Interest::READ,
            Mode::Level,
        ),
        |_readiness, _metadata, data| {
            data.display.dispatch_clients(&mut data.state)?;
            Ok(PostAction::Continue)
        },
    )?;

    let display_handle = display.handle();

    let mut seat_state = SeatState::<State<WinitData>>::new();
    let mut seat = seat_state.new_wl_seat(&display_handle, "seat1");

    seat.add_keyboard(Default::default(), 500, 50)?;
    seat.add_pointer();

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

    let mut damage_tracker = OutputDamageTracker::from_output(&output);

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
            eprintln!("failed to query render node, dmabuf will use v3"); // TODO: tracing
            None
        }
        Err(err) => {
            // TODO: tracing
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

    let mut state = State {
        // TODO: move winit_backend and damage_tracker into their own scope so I can't access them
        // |     from after this
        backend_data: WinitData {
            backend: winit_backend,
            damage_tracker,
            dmabuf_state,
            full_redraw: 0,
        },
        loop_signal: event_loop.get_signal(),
        loop_handle: event_loop.handle(),
        clock: Clock::<Monotonic>::new()?,
        compositor_state: CompositorState::new::<State<WinitData>>(&display_handle),
        data_device_state: DataDeviceState::new::<State<WinitData>>(&display_handle),
        seat_state,
        shm_state: ShmState::new::<State<WinitData>>(&display_handle, Vec::new()),
        space: Space::<Window>::default(),
        cursor_status: CursorImageStatus::Default,
        pointer_location: (0.0, 0.0).into(),
        output_manager_state: OutputManagerState::new_with_xdg_output::<State<WinitData>>(
            &display_handle,
        ),
        xdg_shell_state: XdgShellState::new::<State<WinitData>>(&display_handle),
        viewporter_state: ViewporterState::new::<State<WinitData>>(&display_handle),
        fractional_scale_manager_state: FractionalScaleManagerState::new::<State<WinitData>>(
            &display_handle,
        ),

        move_mode: false,
        socket_name: socket_name.clone(),
    };

    state
        .shm_state
        .update_formats(state.backend_data.backend.renderer().shm_formats());

    let mut data = CalloopData { display, state };

    data.state.space.map_output(&output, (0, 0));

    std::env::set_var("WAYLAND_DISPLAY", socket_name);

    let start_time = std::time::Instant::now();
    let timer = Timer::immediate();

    // TODO: pointer
    evt_loop_handle.insert_source(timer, move |_instant, _metadata, data| {
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
            }
            WinitEvent::Focus(_) => {}
            WinitEvent::Input(input_evt) => {
                state.process_input_event(&seat, input_evt);
            }
            WinitEvent::Refresh => {}
        });

        match result {
            Ok(_) => {}
            Err(WinitError::WindowClosed) => {
                state.loop_signal.stop();
            }
        };

        let full_redraw = &mut state.backend_data.full_redraw;
        *full_redraw = full_redraw.saturating_sub(1);

        let render_res = state.backend_data.backend.bind().and_then(|_| {
            let age = if *full_redraw > 0 {
                0
            } else {
                state.backend_data.backend.buffer_age().unwrap_or(0)
            };

            let renderer = state.backend_data.backend.renderer();

            // render_output()
            let output_render_elements =
                space::space_render_elements(renderer, [&state.space], &output, 1.0).unwrap();

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
                        // TODO: WARN
                    }
                }
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
                                fraction_scale
                                    .set_preferred_scale(output.current_scale().fractional_scale());
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
                // TODO: WARN
            }
        }

        let scale = Scale::from(output.current_scale().fractional_scale());
        let cursor_pos = state.pointer_location;
        let _cursor_pos_scaled: Point<i32, Physical> = cursor_pos.to_physical(scale).to_i32_round();

        state.space.refresh();

        display.flush_clients().unwrap();

        TimeoutAction::ToDuration(Duration::from_millis(6))
    })?;

    event_loop.run(None, &mut data, |_data| {})?;

    Ok(())
}
