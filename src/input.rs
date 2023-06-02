use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent,
        KeyState, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
    },
    desktop::WindowSurfaceType,
    input::{
        keyboard::{keysyms, FilterResult},
        pointer::{AxisFrame, ButtonEvent, MotionEvent},
        Seat,
    },
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge,
    utils::{Point, SERIAL_COUNTER},
};

use crate::{backend::winit::WinitData, State};

impl State<WinitData> {
    pub fn process_input_event<B: InputBackend>(
        &mut self,
        seat: &Seat<State<WinitData>>,
        event: InputEvent<B>,
    ) {
        match event {
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
                    self,
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

                self.move_mode = move_mode;

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
                let output = self.space.outputs().next().unwrap();
                let output_geo = self.space.output_geometry(output).unwrap();
                let pointer_loc =
                    event.position_transformed(output_geo.size) + output_geo.loc.to_f64();
                let serial = SERIAL_COUNTER.next_serial();
                let pointer = seat.get_pointer().unwrap();

                let surface_under_pointer =
                    self.space
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
                    self,
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
                    if let Some((window, window_loc)) = self
                        .space
                        .element_under(pointer_loc)
                        .map(|(w, l)| (w.clone(), l))
                    {
                        const BUTTON_LEFT: u32 = 0x110;
                        const BUTTON_RIGHT: u32 = 0x111;
                        if self.move_mode {
                            if event.button_code() == BUTTON_LEFT {
                                crate::xdg::request::move_request_force(
                                    self,
                                    window.toplevel(),
                                    seat,
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
                                    self,
                                    window.toplevel(),
                                    seat,
                                    serial,
                                    edges,
                                    BUTTON_RIGHT,
                                );
                            }
                        } else {
                            // Move window to top of stack.
                            self.space.raise_element(&window, true);

                            // Focus on window.
                            keyboard.set_focus(
                                self,
                                Some(window.toplevel().wl_surface().clone()),
                                serial,
                            );
                            self.space.elements().for_each(|window| {
                                window.toplevel().send_configure();
                            });
                        }
                    } else {
                        self.space.elements().for_each(|window| {
                            window.set_activated(false);
                            window.toplevel().send_configure();
                        });
                        keyboard.set_focus(self, None, serial);
                    }
                };

                // Send the button event to the client.
                pointer.button(
                    self,
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

                let horizontal_amount = event
                    .amount(Axis::Horizontal)
                    .unwrap_or_else(|| event.amount_discrete(Axis::Horizontal).unwrap() * 3.0);

                let vertical_amount = event
                    .amount(Axis::Vertical)
                    .unwrap_or_else(|| event.amount_discrete(Axis::Vertical).unwrap() * 3.0);

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
                pointer.axis(self, frame);
            }

            _ => (),
        }
    }
}
