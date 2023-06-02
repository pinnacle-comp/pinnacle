use std::{error::Error, os::fd::AsRawFd, sync::Arc, time::Duration};

use smithay::{
    backend::{
        allocator::dmabuf::Dmabuf,
        egl::EGLDevice,
        input::{
            AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputEvent, KeyState,
            KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
        },
        renderer::{
            damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
            gles::GlesRenderer, ImportDma,
        },
        winit::{WinitError, WinitEvent, WinitGraphicsBackend},
    },
    delegate_dmabuf,
    desktop::{space, Space, Window, WindowSurfaceType},
    input::{
        keyboard::{keysyms, FilterResult},
        pointer::{AxisFrame, ButtonEvent, CursorImageStatus, MotionEvent},
        SeatState,
    },
    output::{Output, Subpixel},
    reexports::{
        calloop::{
            generic::Generic,
            timer::{TimeoutAction, Timer},
            EventLoop, Interest, Mode, PostAction,
        },
        wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge,
        wayland_server::{protocol::wl_surface::WlSurface, Display},
    },
    utils::{Clock, Monotonic, Physical, Point, Scale, Transform, SERIAL_COUNTER},
    wayland::{
        compositor::CompositorState,
        data_device::DataDeviceState,
        dmabuf::{
            DmabufFeedback, DmabufFeedbackBuilder, DmabufGlobal, DmabufHandler, DmabufState,
            ImportError,
        },
        output::OutputManagerState,
        shell::xdg::XdgShellState,
        shm::ShmState,
        socket::ListeningSocketSource,
    },
};

use crate::{CalloopData, ClientState, State};

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
        global: &DmabufGlobal,
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

    let state = State {
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

        move_mode: false,
    };

    let mut data = CalloopData { display, state };

    data.state.space.map_output(&output, (0, 0));

    std::env::set_var("WAYLAND_DISPLAY", socket_name);

    let start_time = std::time::Instant::now();
    let timer = Timer::immediate();

    // TODO: pointer
    evt_loop_handle.insert_source(timer, move |_instant, _metadata, data| {
        let display = &mut data.display;
        let state = &mut data.state;

        let result = winit_evt_loop.dispatch_new_events(|event| {
            match event {
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
                WinitEvent::Input(input_evt) => match input_evt {
                    // TODO: extract input events
                    // |     into separate function

                    // InputEvent::DeviceAdded { device } => todo!(),
                    // InputEvent::DeviceRemoved { device } => todo!(),
                    InputEvent::Keyboard { event } => {
                        let serial = SERIAL_COUNTER.next_serial();
                        let time = event.time_msec();
                        let press_state = event.state();
                        let mut move_mode = false;
                        let action = seat.get_keyboard().unwrap().input(
                            state,
                            event.key_code(),
                            press_state,
                            serial,
                            time,
                            |_state, _modifiers, keysym| {
                                if press_state == KeyState::Pressed {
                                    match keysym.modified_sym() {
                                        keysyms::KEY_L => return FilterResult::Intercept(1),
                                        keysyms::KEY_K => return FilterResult::Intercept(2),
                                        keysyms::KEY_J => return FilterResult::Intercept(3),
                                        keysyms::KEY_H => return FilterResult::Intercept(4),
                                        _ => {}
                                    }
                                }

                                if keysym.modified_sym() == keysyms::KEY_Control_L {
                                    match press_state {
                                        KeyState::Pressed => {
                                            move_mode = true;
                                        }
                                        KeyState::Released => {
                                            move_mode = false;
                                        }
                                    }
                                    FilterResult::Forward
                                } else {
                                    FilterResult::Forward
                                }
                            },
                        );

                        state.move_mode = move_mode;

                        match action {
                            Some(1) => {
                                std::process::Command::new("alacritty").spawn().unwrap();
                            }
                            Some(2) => {
                                std::process::Command::new("nautilus").spawn().unwrap();
                            }
                            Some(3) => {
                                std::process::Command::new("kitty").spawn().unwrap();
                            }
                            Some(4) => {
                                std::process::Command::new("foot").spawn().unwrap();
                            }
                            Some(_) => {}
                            None => {}
                        }
                    }
                    InputEvent::PointerMotion { event } => {}
                    InputEvent::PointerMotionAbsolute { event } => {
                        let output = state.space.outputs().next().unwrap();
                        let output_geo = state.space.output_geometry(output).unwrap();
                        let pointer_loc =
                            event.position_transformed(output_geo.size) + output_geo.loc.to_f64();
                        let serial = SERIAL_COUNTER.next_serial();
                        let pointer = seat.get_pointer().unwrap();

                        let surface_under_pointer = state
                            .space
                            .element_under(pointer_loc)
                            .and_then(|(window, location)| {
                                window
                                    .surface_under(
                                        pointer_loc - location.to_f64(),
                                        WindowSurfaceType::ALL,
                                    )
                                    .map(|(s, p)| (s, p + location))
                            });

                        pointer.motion(
                            state,
                            surface_under_pointer,
                            &MotionEvent {
                                location: pointer_loc,
                                serial,
                                time: event.time_msec(),
                            },
                        );
                    }
                    InputEvent::PointerButton { event } => {
                        let pointer = seat.get_pointer().unwrap();
                        let keyboard = seat.get_keyboard().unwrap();

                        // A serial is a number sent with a event that is sent back to the
                        // server by the clients in further requests. This allows the server to
                        // keep track of which event caused which requests. It is an AtomicU32
                        // that increments when next_serial is called.
                        let serial = SERIAL_COUNTER.next_serial();

                        // Returns which button on the pointer was used.
                        let button = event.button_code();

                        // The state, either released or pressed.
                        let button_state = event.state();

                        let pointer_loc = pointer.current_location();

                        // If the button was clicked, focus on the window below if exists, else
                        // unfocus on windows.
                        if ButtonState::Pressed == button_state {
                            if let Some((window, window_loc)) = state
                                .space
                                .element_under(pointer_loc)
                                .map(|(w, l)| (w.clone(), l))
                            {
                                const BUTTON_LEFT: u32 = 0x110;
                                const BUTTON_RIGHT: u32 = 0x111;
                                if state.move_mode {
                                    if event.button_code() == BUTTON_LEFT {
                                        crate::xdg::request::move_request_force(
                                            state,
                                            window.toplevel(),
                                            &seat,
                                            serial,
                                        );
                                        return; // TODO: kinda ugly return here
                                    } else if event.button_code() == BUTTON_RIGHT {
                                        let window_geometry = window.geometry();
                                        let window_x = window_loc.x as f64;
                                        let window_y = window_loc.y as f64;
                                        let window_width = window_geometry.size.w as f64;
                                        let window_height = window_geometry.size.h as f64;
                                        let half_width = window_x + window_width / 2.0;
                                        let half_height = window_y + window_height / 2.0;
                                        let full_width = window_x + window_width;
                                        let full_height = window_y + window_height;

                                        println!(
                                            "window loc: {}, {} | window size: {}, {}",
                                            window_x, window_y, window_width, window_height
                                        );

                                        let edges = match pointer_loc {
                                            Point { x, y, .. }
                                                if (window_x..=half_width).contains(&x)
                                                    && (window_y..=half_height).contains(&y) =>
                                            {
                                                ResizeEdge::TopLeft
                                            }
                                            Point { x, y, .. }
                                                if (half_width..=full_width).contains(&x)
                                                    && (window_y..=half_height).contains(&y) =>
                                            {
                                                ResizeEdge::TopRight
                                            }
                                            Point { x, y, .. }
                                                if (window_x..=half_width).contains(&x)
                                                    && (half_height..=full_height).contains(&y) =>
                                            {
                                                ResizeEdge::BottomLeft
                                            }
                                            Point { x, y, .. }
                                                if (half_width..=full_width).contains(&x)
                                                    && (half_height..=full_height).contains(&y) =>
                                            {
                                                ResizeEdge::BottomRight
                                            }
                                            _ => ResizeEdge::None,
                                        };

                                        crate::xdg::request::resize_request_force(
                                            state,
                                            window.toplevel(),
                                            &seat,
                                            serial,
                                            edges,
                                            BUTTON_RIGHT,
                                        );
                                    }
                                } else {
                                    // Move window to top of stack.
                                    state.space.raise_element(&window, true);

                                    // Focus on window.
                                    keyboard.set_focus(
                                        state,
                                        Some(window.toplevel().wl_surface().clone()),
                                        serial,
                                    );
                                    state.space.elements().for_each(|window| {
                                        window.toplevel().send_configure();
                                    });
                                }
                            } else {
                                state.space.elements().for_each(|window| {
                                    window.set_activated(false);
                                    window.toplevel().send_configure();
                                });
                                keyboard.set_focus(state, None, serial);
                            }
                        };

                        // Send the button event to the client.
                        pointer.button(
                            state,
                            &ButtonEvent {
                                button,
                                state: button_state,
                                serial,
                                time: event.time_msec(),
                            },
                        );
                    }
                    InputEvent::PointerAxis { event } => {
                        let pointer = seat.get_pointer().unwrap();

                        let source = event.source();

                        let horizontal_amount =
                            event.amount(Axis::Horizontal).unwrap_or_else(|| {
                                event.amount_discrete(Axis::Horizontal).unwrap() * 3.0
                            });

                        let vertical_amount = event.amount(Axis::Vertical).unwrap_or_else(|| {
                            event.amount_discrete(Axis::Vertical).unwrap() * 3.0
                        });

                        let horizontal_amount_discrete = event.amount_discrete(Axis::Horizontal);
                        let vertical_amount_discrete = event.amount_discrete(Axis::Vertical);

                        let mut frame = AxisFrame::new(event.time_msec()).source(source);

                        if horizontal_amount != 0.0 {
                            frame = frame.value(Axis::Horizontal, horizontal_amount);
                            if let Some(discrete) = horizontal_amount_discrete {
                                frame = frame.discrete(Axis::Horizontal, discrete as i32);
                            }
                        } else if source == AxisSource::Finger {
                            frame = frame.stop(Axis::Horizontal);
                        }

                        if vertical_amount != 0.0 {
                            frame = frame.value(Axis::Vertical, vertical_amount);
                            if let Some(discrete) = vertical_amount_discrete {
                                frame = frame.discrete(Axis::Vertical, discrete as i32);
                            }
                        } else if source == AxisSource::Finger {
                            frame = frame.stop(Axis::Vertical);
                        }

                        println!("axisframe: {:?}", frame);
                        pointer.axis(state, frame);
                    }
                    // TODO: rest of the InputEvents
                    _ => (),
                },
                WinitEvent::Refresh => {}
            }
        });

        match result {
            Ok(_) => {}
            Err(WinitError::WindowClosed) => {
                state.loop_signal.stop();
            }
        };

        state.backend_data.backend.bind().unwrap();

        let scale = Scale::from(output.current_scale().fractional_scale());
        let cursor_pos = state.pointer_location;
        let _cursor_pos_scaled: Point<i32, Physical> = cursor_pos.to_physical(scale).to_i32_round();

        space::render_output::<_, WaylandSurfaceRenderElement<GlesRenderer>, _, _>(
            &output,
            state.backend_data.backend.renderer(),
            1.0,
            0,
            [&state.space],
            &[],
            &mut state.backend_data.damage_tracker,
            [0.5, 0.5, 0.5, 1.0],
        )
        .unwrap();

        state.backend_data.backend.submit(None).unwrap();

        state.space.elements().for_each(|window| {
            window.send_frame(
                &output,
                start_time.elapsed(),
                Some(Duration::ZERO),
                |_, _| Some(output.clone()),
            )
        });

        state.space.refresh();

        display.flush_clients().unwrap();

        TimeoutAction::ToDuration(Duration::from_millis(6))
    })?;

    event_loop.run(None, &mut data, |_data| {})?;

    Ok(())
}
