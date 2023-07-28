// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;

use crate::{
    api::msg::{CallbackId, Modifier, ModifierMask, OutgoingMsg},
    focus::FocusTarget,
    window::WindowElement,
};
use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent,
        KeyState, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent, PointerMotionEvent,
    },
    desktop::{space::SpaceElement, WindowSurfaceType},
    input::{
        keyboard::{keysyms, FilterResult},
        pointer::{AxisFrame, ButtonEvent, MotionEvent},
    },
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge,
    utils::{Logical, Point, SERIAL_COUNTER},
    wayland::compositor,
};

use crate::{
    backend::{udev::UdevData, winit::WinitData, Backend},
    state::State,
};

#[derive(Default)]
pub struct InputState {
    /// A hashmap of modifier keys and keycodes to callback IDs
    pub keybinds: HashMap<(ModifierMask, u32), CallbackId>,
    /// A hashmap of modifier keys and mouse button codes to callback IDs
    pub mousebinds: HashMap<(ModifierMask, u32), CallbackId>,
}

impl InputState {
    pub fn new() -> Self {
        Default::default()
    }
}

impl<B: Backend> State<B> {
    pub fn surface_under<P>(&self, point: P) -> Option<(WindowElement, Point<i32, Logical>)>
    where
        P: Into<Point<f64, Logical>>,
    {
        // TODO: layer map, popups, fullscreen
        self.space
            .element_under(point)
            .map(|(window, loc)| (window.clone(), loc))
    }

    fn pointer_button<I: InputBackend>(&mut self, event: I::PointerButtonEvent) {
        let pointer = self.seat.get_pointer().expect("Seat has no pointer"); // FIXME: handle err
        let keyboard = self.seat.get_keyboard().expect("Seat has no keyboard"); // FIXME: handle err

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
            if let Some((window, window_loc)) = self.surface_under(pointer_loc) {
                const BUTTON_LEFT: u32 = 0x110;
                const BUTTON_RIGHT: u32 = 0x111;
                if self.move_mode {
                    if event.button_code() == BUTTON_LEFT {
                        if let Some(wl_surf) = window.wl_surface() {
                            crate::xdg::request::move_request_force(
                                self,
                                &wl_surf,
                                &self.seat.clone(),
                                serial,
                            );
                        }
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

                        if let Some(wl_surf) = window.wl_surface() {
                            crate::xdg::request::resize_request_force(
                                self,
                                &wl_surf,
                                &self.seat.clone(),
                                serial,
                                edges,
                                BUTTON_RIGHT,
                            );
                        }
                    }
                } else {
                    // Move window to top of stack.
                    self.space.raise_element(&window, true);
                    if let WindowElement::X11(surface) = &window {
                        self.xwm
                            .as_mut()
                            .expect("no xwm")
                            .raise_window(surface)
                            .expect("failed to raise x11 win");
                        surface.set_activated(true).unwrap();
                    }

                    tracing::debug!(
                        "wl_surface focus is some? {}",
                        window.wl_surface().is_some()
                    );
                    keyboard.set_focus(self, Some(FocusTarget::Window(window.clone())), serial);

                    self.space.elements().for_each(|window| {
                        if let WindowElement::Wayland(window) = window {
                            window.toplevel().send_configure();
                        }
                    });

                    let focused_name = match &window {
                        WindowElement::Wayland(win) => {
                            compositor::with_states(win.toplevel().wl_surface(), |states| {
                                let lock = states
                                    .data_map
                                    .get::<smithay::wayland::shell::xdg::XdgToplevelSurfaceData>()
                                    .expect("XdgToplevelSurfaceData wasn't in surface's data map")
                                    .lock()
                                    .expect("failed to acquire lock");
                                lock.app_id.clone().unwrap_or_default()
                            })
                        }
                        WindowElement::X11(surf) => surf.class(),
                    };
                    tracing::debug!("setting keyboard focus to {focused_name}");
                }
            } else {
                self.space.elements().for_each(|window| match window {
                    WindowElement::Wayland(window) => {
                        window.set_activated(false);
                        window.toplevel().send_configure();
                    }
                    WindowElement::X11(surface) => {
                        surface
                            .set_activated(false)
                            .expect("failed to deactivate x11 win");
                        // INFO: do i need to configure this?
                    }
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

    fn pointer_axis<I: InputBackend>(&mut self, event: I::PointerAxisEvent) {
        let source = event.source();

        let horizontal_amount = event
            .amount(Axis::Horizontal)
            .unwrap_or_else(|| event.amount_discrete(Axis::Horizontal).unwrap_or(0.0) * 3.0);

        let vertical_amount = event
            .amount(Axis::Vertical)
            .unwrap_or_else(|| event.amount_discrete(Axis::Vertical).unwrap_or(0.0) * 3.0);

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

        self.seat
            .get_pointer()
            .expect("Seat has no pointer")
            .axis(self, frame); // FIXME: handle err
    }

    fn keyboard<I: InputBackend>(&mut self, event: I::KeyboardKeyEvent) {
        let serial = SERIAL_COUNTER.next_serial();
        let time = event.time_msec();
        let press_state = event.state();
        let mut move_mode = false;
        let action = self
            .seat
            .get_keyboard()
            .expect("Seat has no keyboard") // FIXME: handle err
            .input(
                self,
                event.key_code(),
                press_state,
                serial,
                time,
                |state, modifiers, keysym| {
                    if press_state == KeyState::Pressed {
                        let mut modifier_mask = Vec::<Modifier>::new();
                        if modifiers.alt {
                            modifier_mask.push(Modifier::Alt);
                        }
                        if modifiers.shift {
                            modifier_mask.push(Modifier::Shift);
                        }
                        if modifiers.ctrl {
                            modifier_mask.push(Modifier::Ctrl);
                        }
                        if modifiers.logo {
                            modifier_mask.push(Modifier::Super);
                        }
                        let raw_sym = if keysym.raw_syms().len() == 1 {
                            keysym.raw_syms()[0]
                        } else {
                            keysyms::KEY_NoSymbol
                        };
                        if let Some(callback_id) = state
                            .input_state
                            .keybinds
                            .get(&(modifier_mask.into(), raw_sym))
                        {
                            return FilterResult::Intercept(*callback_id);
                        } else if modifiers.ctrl
                            && modifiers.shift
                            && modifiers.alt
                            && keysym.modified_sym() == keysyms::KEY_Escape
                        {
                            return FilterResult::Intercept(CallbackId(999999));
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
                    }
                    FilterResult::Forward
                },
            );

        self.move_mode = move_mode;

        if let Some(callback_id) = action {
            if callback_id.0 == 999999 {
                self.loop_signal.stop();
            }
            if let Some(stream) = self.api_state.stream.as_ref() {
                if let Err(err) = crate::api::send_to_client(
                    &mut stream.lock().expect("Could not lock stream mutex"),
                    &OutgoingMsg::CallCallback {
                        callback_id,
                        args: None,
                    },
                ) {
                    tracing::warn!("error sending msg to client: {err}");
                }
            }
        }
    }
}

impl State<WinitData> {
    pub fn process_input_event<B: InputBackend>(&mut self, event: InputEvent<B>) {
        match event {
            // TODO: rest of input events

            // InputEvent::DeviceAdded { device } => todo!(),
            // InputEvent::DeviceRemoved { device } => todo!(),
            InputEvent::Keyboard { event } => self.keyboard::<B>(event),
            // InputEvent::PointerMotion { event } => {}
            InputEvent::PointerMotionAbsolute { event } => self.pointer_motion_absolute::<B>(event),
            InputEvent::PointerButton { event } => self.pointer_button::<B>(event),
            InputEvent::PointerAxis { event } => self.pointer_axis::<B>(event),

            _ => (),
        }
    }

    fn pointer_motion_absolute<I: InputBackend>(&mut self, event: I::PointerMotionAbsoluteEvent) {
        let Some(output) = self.space.outputs().next() else { return; };
        let output_geo = self
            .space
            .output_geometry(output)
            .expect("Output geometry doesn't exist");
        let pointer_loc = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();
        let serial = SERIAL_COUNTER.next_serial();
        let pointer = self.seat.get_pointer().expect("Seat has no pointer"); // FIXME: handle err

        // tracing::info!("pointer_loc: {:?}", pointer_loc);

        self.pointer_location = pointer_loc;

        match self.focus_state.focused_output {
            Some(_) => {
                if let Some(output) = self
                    .space
                    .output_under(self.pointer_location)
                    .next()
                    .cloned()
                {
                    self.focus_state.focused_output = Some(output);
                }
            }
            None => {
                self.focus_state.focused_output = self.space.outputs().next().cloned();
            }
        }

        let surface_under_pointer =
            self.space
                .element_under(pointer_loc)
                .and_then(|(window, location)| {
                    window
                        .surface_under(pointer_loc - location.to_f64(), WindowSurfaceType::ALL)
                        .map(|(_s, p)| (FocusTarget::Window(window.clone()), p + location))
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
}

impl State<UdevData> {
    pub fn process_input_event<B: InputBackend>(&mut self, event: InputEvent<B>) {
        match event {
            // TODO: rest of input events

            // InputEvent::DeviceAdded { device } => todo!(),
            // InputEvent::DeviceRemoved { device } => todo!(),
            InputEvent::Keyboard { event } => self.keyboard::<B>(event),
            InputEvent::PointerMotion { event } => self.pointer_motion::<B>(event),
            InputEvent::PointerMotionAbsolute { event } => self.pointer_motion_absolute::<B>(event),
            InputEvent::PointerButton { event } => self.pointer_button::<B>(event),
            InputEvent::PointerAxis { event } => self.pointer_axis::<B>(event),

            _ => (),
        }
    }

    fn pointer_motion<I: InputBackend>(&mut self, event: I::PointerMotionEvent) {
        let serial = SERIAL_COUNTER.next_serial();
        self.pointer_location += event.delta();

        // clamp to screen limits
        // this event is never generated by winit
        self.pointer_location = self.clamp_coords(self.pointer_location);
        match self.focus_state.focused_output {
            Some(_) => {
                if let Some(output) = self
                    .space
                    .output_under(self.pointer_location)
                    .next()
                    .cloned()
                {
                    self.focus_state.focused_output = Some(output);
                }
            }
            None => {
                self.focus_state.focused_output = self.space.outputs().next().cloned();
            }
        }

        let surface_under = self
            .surface_under(self.pointer_location)
            .map(|(window, loc)| (FocusTarget::Window(window), loc));

        // tracing::info!("{:?}", self.pointer_location);
        if let Some(ptr) = self.seat.get_pointer() {
            ptr.motion(
                self,
                surface_under,
                &MotionEvent {
                    location: self.pointer_location,
                    serial,
                    time: event.time_msec(),
                },
            );

            // ptr.relative_motion(
            //     self,
            //     under,
            //     &RelativeMotionEvent {
            //         delta: event.delta(),
            //         delta_unaccel: event.delta_unaccel(),
            //         utime: event.time(),
            //     },
            // )
        }
    }

    fn pointer_motion_absolute<I: InputBackend>(&mut self, event: I::PointerMotionAbsoluteEvent) {
        let serial = SERIAL_COUNTER.next_serial();

        let max_x = self.space.outputs().fold(0, |acc, o| {
            acc + self
                .space
                .output_geometry(o)
                .expect("Output geometry doesn't exist")
                .size
                .w
        });

        let Some(max_h_output) = self
            .space
            .outputs()
            .max_by_key(|o| {
                self.space
                    .output_geometry(o)
                    .expect("Output geometry doesn't exist")
                    .size
                    .h
            })
        else {
            tracing::warn!("Pointer moved, but there was no output");
            return;
        };

        let max_y = self
            .space
            .output_geometry(max_h_output)
            .expect("Output geometry doesn't exist")
            .size
            .h;

        self.pointer_location.x = event.x_transformed(max_x);
        self.pointer_location.y = event.y_transformed(max_y);

        self.pointer_location = self.clamp_coords(self.pointer_location);

        let surface_under = self
            .surface_under(self.pointer_location)
            .map(|(window, loc)| (FocusTarget::Window(window), loc));

        if let Some(ptr) = self.seat.get_pointer() {
            ptr.motion(
                self,
                surface_under,
                &MotionEvent {
                    location: self.pointer_location,
                    serial,
                    time: event.time_msec(),
                },
            );
        }
    }

    fn clamp_coords(&self, pos: Point<f64, Logical>) -> Point<f64, Logical> {
        if self.space.outputs().next().is_none() {
            return pos;
        }

        let (pos_x, pos_y) = pos.into();
        let max_x = self.space.outputs().fold(0, |acc, o| {
            acc + self
                .space
                .output_geometry(o)
                .expect("Output geometry doesn't exist")
                .size
                .w
        });
        let clamped_x = pos_x.clamp(0.0, max_x as f64);
        let max_y = self
            .space
            .outputs()
            .find(|o| {
                let geo = self
                    .space
                    .output_geometry(o)
                    .expect("Output geometry doesn't exist");
                geo.contains((clamped_x as i32, 0))
            })
            .map(|o| {
                self.space
                    .output_geometry(o)
                    .expect("Output geometry doesn't exist")
                    .size
                    .h
            });

        if let Some(max_y) = max_y {
            let clamped_y = pos_y.clamp(0.0, max_y as f64);
            (clamped_x, clamped_y).into()
        } else {
            (clamped_x, pos_y).into()
        }
    }
}
