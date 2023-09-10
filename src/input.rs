// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;

use crate::{
    api::msg::{CallbackId, Modifier, ModifierMask, OutgoingMsg},
    focus::FocusTarget,
    state::{Backend, WithState},
    window::WindowElement,
};
use smithay::{
    backend::{
        input::{
            AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent,
            KeyState, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent, PointerMotionEvent,
        },
        session::Session,
    },
    desktop::{layer_map_for_output, space::SpaceElement},
    input::{
        keyboard::{keysyms, FilterResult},
        pointer::{AxisFrame, ButtonEvent, MotionEvent},
    },
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge,
    utils::{Logical, Point, SERIAL_COUNTER},
    wayland::{compositor, seat::WaylandFocus, shell::wlr_layer},
};

use crate::state::State;

pub struct InputState {
    /// A hashmap of modifier keys and keycodes to callback IDs
    pub keybinds: HashMap<(ModifierMask, u32), CallbackId>,
    /// A hashmap of modifier keys and mouse button codes to callback IDs
    pub mousebinds: HashMap<(ModifierMask, u32), CallbackId>,
    pub reload_keybind: (ModifierMask, u32),
    pub kill_keybind: (ModifierMask, u32),
}

impl InputState {
    pub fn new(reload_keybind: (ModifierMask, u32), kill_keybind: (ModifierMask, u32)) -> Self {
        Self {
            keybinds: HashMap::new(),
            mousebinds: HashMap::new(),
            reload_keybind,
            kill_keybind,
        }
    }
}

#[derive(Debug)]
enum KeyAction {
    CallCallback(CallbackId),
    Quit,
    SwitchVt(i32),
    ReloadConfig,
}

impl State {
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

    /// Get the [`FocusTarget`] under `point`.
    pub fn surface_under<P>(&self, point: P) -> Option<(FocusTarget, Point<i32, Logical>)>
    where
        P: Into<Point<f64, Logical>>,
    {
        let point: Point<f64, Logical> = point.into();

        let output = self.space.outputs().find(|op| {
            self.space
                .output_geometry(op)
                .expect("called output_geometry on unmapped output (this shouldn't happen here)")
                .contains(point.to_i32_round())
        })?;

        let output_geo = self
            .space
            .output_geometry(output)
            .expect("called output_geometry on unmapped output");

        let layers = layer_map_for_output(output);

        let top_fullscreen_window = self.focus_state.focus_stack.iter().rev().find(|win| {
            win.with_state(|state| {
                state.fullscreen_or_maximized.is_fullscreen()
                    && state.tags.iter().any(|tag| tag.active())
            })
        });

        // I think I'm going a bit too far with the functional stuff

        // The topmost fullscreen window
        top_fullscreen_window
            .map(|window| (FocusTarget::from(window.clone()), output_geo.loc))
            .or_else(|| {
                // The topmost layer surface in Overlay or Top
                layers
                    .layer_under(wlr_layer::Layer::Overlay, point)
                    .or_else(|| layers.layer_under(wlr_layer::Layer::Top, point))
                    .map(|layer| {
                        let layer_loc = layers.layer_geometry(layer).expect("no layer geo").loc;
                        (FocusTarget::from(layer.clone()), output_geo.loc + layer_loc)
                    })
            })
            .or_else(|| {
                // The topmost window
                self.space
                    .elements()
                    .rev()
                    .filter(|win| win.is_on_active_tag(self.space.outputs()))
                    .find_map(|win| {
                        let loc = self
                            .space
                            .element_location(win)
                            .expect("called elem loc on unmapped win")
                            - win.geometry().loc;

                        if win.is_in_input_region(&(point - loc.to_f64())) {
                            Some((win.clone().into(), loc))
                        } else {
                            None
                        }
                    })
            })
            .or_else(|| {
                // The topmost layer surface in Bottom or Background
                layers
                    .layer_under(wlr_layer::Layer::Bottom, point)
                    .or_else(|| layers.layer_under(wlr_layer::Layer::Background, point))
                    .map(|layer| {
                        let layer_loc = layers.layer_geometry(layer).expect("no layer geo").loc;
                        (FocusTarget::from(layer.clone()), output_geo.loc + layer_loc)
                    })
            })
    }

    fn keyboard<I: InputBackend>(&mut self, event: I::KeyboardKeyEvent) {
        let serial = SERIAL_COUNTER.next_serial();
        let time = event.time_msec();
        let press_state = event.state();
        let mut move_mode = false;

        let reload_keybind = self.input_state.reload_keybind;
        let kill_keybind = self.input_state.kill_keybind;

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
                        let modifier_mask = ModifierMask::from(modifier_mask);
                        let raw_sym = keysym.raw_syms().iter().next();
                        let mod_sym = keysym.modified_sym();

                        let cb_id_mod = state
                            .input_state
                            .keybinds
                            .get(&(modifier_mask, mod_sym));

                        let cb_id_raw = if let Some(raw_sym) = raw_sym {
                            state.input_state.keybinds.get(&(modifier_mask, *raw_sym))
                        } else {
                            None
                        };

                        match (cb_id_mod, cb_id_raw) {
                            (Some(cb_id), _) | (None, Some(cb_id)) => {
                                return FilterResult::Intercept(KeyAction::CallCallback(*cb_id));
                            }
                            (None, None) => ()
                        }

                        if (modifier_mask, mod_sym) == kill_keybind {
                            return FilterResult::Intercept(KeyAction::Quit);
                        } else if (modifier_mask, mod_sym) == reload_keybind {
                            return FilterResult::Intercept(KeyAction::ReloadConfig);
                        } else if let mut vt @ keysyms::KEY_XF86Switch_VT_1..=keysyms::KEY_XF86Switch_VT_12 =
                            keysym.modified_sym() {
                            vt = vt - keysyms::KEY_XF86Switch_VT_1 + 1;
                            tracing::info!("Switching to vt {vt}");
                            return FilterResult::Intercept(KeyAction::SwitchVt(vt as i32));
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

        match action {
            Some(KeyAction::CallCallback(callback_id)) => {
                if let Some(stream) = self.api_state.stream.as_ref() {
                    if let Err(err) = crate::api::send_to_client(
                        &mut stream.lock().expect("Could not lock stream mutex"),
                        &OutgoingMsg::CallCallback {
                            callback_id,
                            args: None,
                        },
                    ) {
                        tracing::error!("error sending msg to client: {err}");
                    }
                }
            }
            Some(KeyAction::SwitchVt(vt)) => {
                if let Backend::Udev(udev) = &mut self.backend {
                    if let Err(err) = udev.session.change_vt(vt) {
                        tracing::error!("Failed to switch to vt {vt}: {err}");
                    }
                }
            }
            Some(KeyAction::Quit) => {
                self.loop_signal.stop();
            }
            Some(KeyAction::ReloadConfig) => {
                self.restart_config().expect("failed to restart config");
            }
            None => {}
        }
    }

    fn pointer_button<I: InputBackend>(&mut self, event: I::PointerButtonEvent) {
        let pointer = self.seat.get_pointer().expect("Seat has no pointer"); // FIXME: handle err
        let keyboard = self.seat.get_keyboard().expect("Seat has no keyboard"); // FIXME: handle err

        let serial = SERIAL_COUNTER.next_serial();

        let button = event.button_code();

        let button_state = event.state();

        let pointer_loc = pointer.current_location();

        // If the button was clicked, focus on the window below if exists, else
        // unfocus on windows.
        if ButtonState::Pressed == button_state {
            if let Some((focus, window_loc)) = self.surface_under(pointer_loc) {
                // tracing::debug!("button click on {window:?}");
                const BUTTON_LEFT: u32 = 0x110;
                const BUTTON_RIGHT: u32 = 0x111;
                if self.move_mode {
                    if event.button_code() == BUTTON_LEFT {
                        if let Some(wl_surf) = focus.wl_surface() {
                            crate::grab::move_grab::move_request_server(
                                self,
                                &wl_surf,
                                &self.seat.clone(),
                                serial,
                                BUTTON_LEFT,
                            );
                        }
                        return; // TODO: kinda ugly return here
                    } else if event.button_code() == BUTTON_RIGHT {
                        let FocusTarget::Window(window) = focus else { return };
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
                            crate::grab::resize_grab::resize_request_server(
                                self,
                                &wl_surf,
                                &self.seat.clone(),
                                serial,
                                edges.into(),
                                BUTTON_RIGHT,
                            );
                        }
                    }
                } else {
                    // Move window to top of stack.
                    if let FocusTarget::Window(window) = &focus {
                        self.space.raise_element(window, true);
                        if let WindowElement::X11(surface) = &window {
                            if !surface.is_override_redirect() {
                                self.xwm
                                    .as_mut()
                                    .expect("no xwm")
                                    .raise_window(surface)
                                    .expect("failed to raise x11 win");
                                surface
                                    .set_activated(true)
                                    .expect("failed to set x11 win to activated");
                            }
                        }
                    }

                    tracing::debug!("wl_surface focus is some? {}", focus.wl_surface().is_some());

                    // NOTE: *Do not* set keyboard focus to an override redirect window. This leads
                    // |     to wonky things like right-click menus not correctly getting pointer
                    // |     clicks or showing up at all.

                    // TODO: use update_keyboard_focus from anvil

                    if !matches!(&focus, FocusTarget::Window(WindowElement::X11(surf)) if surf.is_override_redirect())
                    {
                        keyboard.set_focus(self, Some(focus.clone()), serial);
                    }

                    self.space.elements().for_each(|window| {
                        if let WindowElement::Wayland(window) = window {
                            window.toplevel().send_configure();
                        }
                    });

                    if let FocusTarget::Window(window) = &focus {
                        tracing::debug!("setting keyboard focus to {:?}", window.class());
                    }
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

        // tracing::debug!(
        //     "axis on current focus: {:?}",
        //     self.seat.get_pointer().unwrap().current_focus()
        // );

        self.seat
            .get_pointer()
            .expect("Seat has no pointer")
            .axis(self, frame);
    }

    /// Clamp pointer coordinates inside outputs
    fn clamp_coords(&self, pos: Point<f64, Logical>) -> Point<f64, Logical> {
        if self.space.outputs().next().is_none() {
            return pos;
        }

        let (pos_x, pos_y) = pos.into();

        let nearest_points = self.space.outputs().map(|op| {
            let size = self
                .space
                .output_geometry(op)
                .expect("called output_geometry on unmapped output")
                .size;
            let loc = op.current_location();
            let pos_x = pos_x.clamp(loc.x as f64, (loc.x + size.w) as f64);
            let pos_y = pos_y.clamp(loc.y as f64, (loc.y + size.h) as f64);
            (pos_x, pos_y)
        });

        let nearest_point = nearest_points.min_by(|(x1, y1), (x2, y2)| {
            f64::total_cmp(
                &((pos_x - x1).powi(2) + (pos_y - y1).powi(2)).sqrt(),
                &((pos_x - x2).powi(2) + (pos_y - y2).powi(2)).sqrt(),
            )
        });

        nearest_point.map(|point| point.into()).unwrap_or(pos)
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

        let surface_under_pointer = self.surface_under(pointer_loc);

        // tracing::debug!("surface_under_pointer: {surface_under_pointer:?}");
        // tracing::debug!("pointer focus: {:?}", pointer.current_focus());
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

        let surface_under = self.surface_under(self.pointer_location);

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
}
