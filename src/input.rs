// SPDX-License-Identifier: GPL-3.0-or-later

pub mod libinput;

use std::{collections::HashMap, mem::Discriminant};

use crate::{focus::FocusTarget, state::WithState, window::WindowElement};
use pinnacle_api_defs::pinnacle::input::v0alpha1::{
    set_libinput_setting_request::Setting, set_mousebind_request, SetKeybindResponse,
    SetMousebindResponse,
};
use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent,
        KeyState, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent, PointerMotionEvent,
    },
    desktop::{layer_map_for_output, space::SpaceElement},
    input::{
        keyboard::{keysyms, FilterResult, ModifiersState},
        pointer::{AxisFrame, ButtonEvent, MotionEvent, RelativeMotionEvent},
    },
    reexports::input::{self, Led},
    utils::{Logical, Point, SERIAL_COUNTER},
    wayland::shell::wlr_layer,
};
use tokio::sync::mpsc::UnboundedSender;
use xkbcommon::xkb::Keysym;

use crate::state::State;

bitflags::bitflags! {
    #[derive(Debug, Hash, Copy, Clone, PartialEq, Eq)]
    pub struct ModifierMask: u8 {
        const SHIFT = 1;
        const CTRL  = 1 << 1;
        const ALT   = 1 << 2;
        const SUPER = 1 << 3;
    }
}

impl From<ModifiersState> for ModifierMask {
    fn from(modifiers: ModifiersState) -> Self {
        let mut mask = ModifierMask::empty();
        if modifiers.alt {
            mask |= ModifierMask::ALT;
        }
        if modifiers.shift {
            mask |= ModifierMask::SHIFT;
        }
        if modifiers.ctrl {
            mask |= ModifierMask::CTRL;
        }
        if modifiers.logo {
            mask |= ModifierMask::SUPER;
        }
        mask
    }
}

impl From<&ModifiersState> for ModifierMask {
    fn from(modifiers: &ModifiersState) -> Self {
        let mut mask = ModifierMask::empty();
        if modifiers.alt {
            mask |= ModifierMask::ALT;
        }
        if modifiers.shift {
            mask |= ModifierMask::SHIFT;
        }
        if modifiers.ctrl {
            mask |= ModifierMask::CTRL;
        }
        if modifiers.logo {
            mask |= ModifierMask::SUPER;
        }
        mask
    }
}

#[derive(Default)]
pub struct InputState {
    pub reload_keybind: Option<(ModifierMask, Keysym)>,
    pub kill_keybind: Option<(ModifierMask, Keysym)>,
    /// All libinput devices that have been connected
    pub libinput_devices: Vec<input::Device>,

    pub keybinds:
        HashMap<(ModifierMask, Keysym), UnboundedSender<Result<SetKeybindResponse, tonic::Status>>>,
    pub mousebinds: HashMap<
        (ModifierMask, u32, set_mousebind_request::MouseEdge),
        UnboundedSender<Result<SetMousebindResponse, tonic::Status>>,
    >,
    #[allow(clippy::type_complexity)]
    pub libinput_settings: HashMap<Discriminant<Setting>, Box<dyn Fn(&mut input::Device) + Send>>,
}

impl InputState {
    pub fn clear(&mut self) {
        self.reload_keybind = None;
        self.kill_keybind = None;
        self.libinput_devices.clear();
        self.keybinds.clear();
        self.mousebinds.clear();
        self.libinput_settings.clear();
    }
}

impl std::fmt::Debug for InputState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InputState")
            .field("reload_keybind", &self.reload_keybind)
            .field("kill_keybind", &self.kill_keybind)
            .field("libinput_devices", &self.libinput_devices)
            .field("keybinds", &self.keybinds)
            .field("mousebinds", &self.mousebinds)
            .field("libinput_settings", &"...")
            .finish()
    }
}

impl InputState {
    pub fn new() -> Self {
        Default::default()
    }
}

#[derive(Debug)]
enum KeyAction {
    CallCallback(UnboundedSender<Result<SetKeybindResponse, tonic::Status>>),
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
    pub fn focus_target_under<P>(&self, point: P) -> Option<(FocusTarget, Point<i32, Logical>)>
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

        let top_fullscreen_window = output
            .with_state(|state| state.focus_stack.stack.clone())
            .into_iter()
            .rev()
            .find(|win| {
                win.with_state(|state| {
                    state.fullscreen_or_maximized.is_fullscreen()
                        && output.with_state(|op_state| {
                            op_state
                                .focused_tags()
                                .any(|op_tag| state.tags.contains(op_tag))
                        })
                })
            });

        if let Some(window) = top_fullscreen_window {
            Some((FocusTarget::from(window.clone()), output_geo.loc))
        } else if let (Some(layer), _) | (None, Some(layer)) = (
            layers.layer_under(wlr_layer::Layer::Overlay, point),
            layers.layer_under(wlr_layer::Layer::Top, point),
        ) {
            let layer_loc = layers.layer_geometry(layer).expect("no layer geo").loc;
            Some((FocusTarget::from(layer.clone()), output_geo.loc + layer_loc))
        } else if let Some(ret) = self
            .space
            .elements()
            .rev()
            .filter(|win| win.is_on_active_tag(self.space.outputs()))
            .find_map(|win| {
                let loc = self
                    .space
                    .element_location(win)
                    .expect("called elem loc on unmapped win")
                    - win.geometry().loc;

                win.is_in_input_region(&(point - loc.to_f64()))
                    .then(|| (win.clone().into(), loc))
            })
        {
            Some(ret)
        } else if let (Some(layer), _) | (None, Some(layer)) = (
            layers.layer_under(wlr_layer::Layer::Overlay, point),
            layers.layer_under(wlr_layer::Layer::Top, point),
        ) {
            let layer_loc = layers.layer_geometry(layer).expect("no layer geo").loc;
            Some((FocusTarget::from(layer.clone()), output_geo.loc + layer_loc))
        } else {
            None
        }
    }

    fn keyboard<I: InputBackend>(&mut self, event: I::KeyboardKeyEvent) {
        let serial = SERIAL_COUNTER.next_serial();
        let time = event.time_msec();
        let press_state = event.state();

        let reload_keybind = self.input_state.reload_keybind;
        let kill_keybind = self.input_state.kill_keybind;

        let keyboard = self.seat.get_keyboard().expect("Seat has no keyboard");

        let modifiers = keyboard.modifier_state();

        let mut leds = Led::empty();
        if modifiers.num_lock {
            leds |= Led::NUMLOCK;
        }
        if modifiers.caps_lock {
            leds |= Led::CAPSLOCK;
        }

        // FIXME: Leds only update once another key is pressed.
        for device in self.input_state.libinput_devices.iter_mut() {
            device.led_update(leds);
        }

        let action = keyboard.input(
            self,
            event.key_code(),
            press_state,
            serial,
            time,
            |state, modifiers, keysym| {
                // tracing::debug!(keysym = ?keysym, raw_keysyms = ?keysym.raw_syms(), modified_syms = ?keysym.modified_syms());
                if press_state == KeyState::Pressed {
                    let mod_mask = ModifierMask::from(modifiers);

                    let raw_sym = keysym.raw_syms().iter().next();
                    let mod_sym = keysym.modified_sym();

                    if let (Some(sender), _) | (None, Some(sender)) = (
                        state.input_state.keybinds.get(&(mod_mask, mod_sym)),
                        raw_sym.and_then(|raw_sym| {
                            state.input_state.keybinds.get(&(mod_mask, *raw_sym))
                        }),
                    ) {
                        return FilterResult::Intercept(KeyAction::CallCallback(sender.clone()));
                    }

                    if kill_keybind == Some((mod_mask, mod_sym)) {
                        return FilterResult::Intercept(KeyAction::Quit);
                    } else if reload_keybind == Some((mod_mask, mod_sym)) {
                        return FilterResult::Intercept(KeyAction::ReloadConfig);
                    } else if let mut vt @ keysyms::KEY_XF86Switch_VT_1
                        ..=keysyms::KEY_XF86Switch_VT_12 = keysym.modified_sym().raw()
                    {
                        vt = vt - keysyms::KEY_XF86Switch_VT_1 + 1;
                        tracing::info!("Switching to vt {vt}");
                        return FilterResult::Intercept(KeyAction::SwitchVt(vt as i32));
                    }
                }

                FilterResult::Forward
            },
        );

        match action {
            Some(KeyAction::CallCallback(sender)) => {
                let _ = sender.send(Ok(SetKeybindResponse {}));
            }
            Some(KeyAction::SwitchVt(vt)) => {
                self.switch_vt(vt);
            }
            Some(KeyAction::Quit) => {
                self.shutdown();
            }
            Some(KeyAction::ReloadConfig) => {
                self.start_config(crate::config::get_config_dir(&self.xdg_base_dirs))
                    .expect("failed to restart config");
            }
            None => (),
        }
    }

    fn pointer_button<I: InputBackend>(&mut self, event: I::PointerButtonEvent) {
        let pointer = self.seat.get_pointer().expect("Seat has no pointer"); // FIXME: handle err
        let keyboard = self.seat.get_keyboard().expect("Seat has no keyboard"); // FIXME: handle err

        let serial = SERIAL_COUNTER.next_serial();

        let button = event.button_code();

        let button_state = event.state();

        let pointer_loc = pointer.current_location();

        let mod_mask = ModifierMask::from(keyboard.modifier_state());

        let mouse_edge = match button_state {
            ButtonState::Released => set_mousebind_request::MouseEdge::Release,
            ButtonState::Pressed => set_mousebind_request::MouseEdge::Press,
        };

        if let Some(stream) = self
            .input_state
            .mousebinds
            .get(&(mod_mask, button, mouse_edge))
        {
            let _ = stream.send(Ok(SetMousebindResponse {}));
            return;
        }

        // If the button was clicked, focus on the window below if exists, else
        // unfocus on windows.
        if button_state == ButtonState::Pressed {
            if let Some((focus, _)) = self.focus_target_under(pointer_loc) {
                // NOTE: *Do not* set keyboard focus to an override redirect window. This leads
                // |     to wonky things like right-click menus not correctly getting pointer
                // |     clicks or showing up at all.

                // TODO: use update_keyboard_focus from anvil

                if let FocusTarget::Window(window) = &focus {
                    self.space.raise_element(window, true);
                    self.z_index_stack.set_focus(window.clone());
                    if let Some(output) = window.output(self) {
                        output.with_state(|state| state.focus_stack.set_focus(window.clone()));
                    }
                }

                if !matches!(
                    &focus,
                    FocusTarget::Window(WindowElement::X11OverrideRedirect(_))
                ) {
                    keyboard.set_focus(self, Some(focus.clone()), serial);
                }

                for window in self.space.elements() {
                    if let WindowElement::Wayland(window) = window {
                        window.toplevel().expect("in wayland enum").send_configure();
                    }
                }
            } else {
                if let Some(focused_op) = self.output_focus_stack.current_focus() {
                    focused_op.with_state(|state| {
                        state.focus_stack.unset_focus();
                        for window in state.focus_stack.stack.iter() {
                            window.set_activate(false);
                            if let WindowElement::Wayland(window) = window {
                                window.toplevel().expect("in wayland enum").send_configure();
                            }
                        }
                    });
                }
                keyboard.set_focus(self, None, serial);
            }
        };

        pointer.button(
            self,
            &ButtonEvent {
                button,
                state: button_state,
                serial,
                time: event.time_msec(),
            },
        );
        pointer.frame(self);
    }

    fn pointer_axis<I: InputBackend>(&mut self, event: I::PointerAxisEvent) {
        let source = event.source();

        let horizontal_amount = event
            .amount(Axis::Horizontal)
            .unwrap_or_else(|| event.amount_v120(Axis::Horizontal).unwrap_or(0.0) * 3.0 / 120.);

        let vertical_amount = event
            .amount(Axis::Vertical)
            .unwrap_or_else(|| event.amount_v120(Axis::Vertical).unwrap_or(0.0) * 3.0 / 120.);

        let horizontal_amount_discrete = event.amount_v120(Axis::Horizontal);
        let vertical_amount_discrete = event.amount_v120(Axis::Vertical);

        let mut frame = AxisFrame::new(event.time_msec()).source(source);

        if horizontal_amount != 0.0 {
            frame = frame.value(Axis::Horizontal, horizontal_amount);
            if let Some(discrete) = horizontal_amount_discrete {
                frame = frame.v120(Axis::Horizontal, discrete as i32);
            }
        } else if source == AxisSource::Finger {
            frame = frame.stop(Axis::Horizontal);
        }

        if vertical_amount != 0.0 {
            frame = frame.value(Axis::Vertical, vertical_amount);
            if let Some(discrete) = vertical_amount_discrete {
                frame = frame.v120(Axis::Vertical, discrete as i32);
            }
        } else if source == AxisSource::Finger {
            frame = frame.stop(Axis::Vertical);
        }

        let pointer = self.seat.get_pointer().expect("Seat has no pointer");

        pointer.axis(self, frame);
        pointer.frame(self);
    }

    /// Clamp pointer coordinates inside outputs.
    ///
    /// This returns the nearest point inside an output.
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
        let Some(output) = self.space.outputs().next() else {
            return;
        };

        let output_geo = self
            .space
            .output_geometry(output)
            .expect("Output geometry doesn't exist");
        let pointer_loc = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();
        let serial = SERIAL_COUNTER.next_serial();
        let pointer = self.seat.get_pointer().expect("Seat has no pointer"); // FIXME: handle err

        self.pointer_location = pointer_loc;

        match self.output_focus_stack.current_focus() {
            Some(_) => {
                if let Some(output) = self
                    .space
                    .output_under(self.pointer_location)
                    .next()
                    .cloned()
                {
                    self.output_focus_stack.set_focus(output);
                }
            }
            None => {
                if let Some(output) = self.space.outputs().next().cloned() {
                    self.output_focus_stack.set_focus(output);
                }
            }
        }

        pointer.motion(
            self,
            self.focus_target_under(pointer_loc),
            &MotionEvent {
                location: pointer_loc,
                serial,
                time: event.time_msec(),
            },
        );

        pointer.frame(self);
    }

    fn pointer_motion<I: InputBackend>(&mut self, event: I::PointerMotionEvent) {
        let serial = SERIAL_COUNTER.next_serial();
        self.pointer_location += event.delta();

        // clamp to screen limits
        // this event is never generated by winit
        self.pointer_location = self.clamp_coords(self.pointer_location);

        match self.output_focus_stack.current_focus() {
            Some(_) => {
                if let Some(output) = self
                    .space
                    .output_under(self.pointer_location)
                    .next()
                    .cloned()
                {
                    self.output_focus_stack.set_focus(output);
                }
            }
            None => {
                if let Some(output) = self.space.outputs().next().cloned() {
                    self.output_focus_stack.set_focus(output);
                }
            }
        }

        let surface_under = self.focus_target_under(self.pointer_location);

        if let Some(pointer) = self.seat.get_pointer() {
            pointer.motion(
                self,
                surface_under.clone(),
                &MotionEvent {
                    location: self.pointer_location,
                    serial,
                    time: event.time_msec(),
                },
            );

            pointer.relative_motion(
                self,
                surface_under,
                &RelativeMotionEvent {
                    delta: event.delta(),
                    delta_unaccel: event.delta_unaccel(),
                    utime: event.time(),
                },
            );

            pointer.frame(self);

            if let Some(output) = self.output_focus_stack.current_focus().cloned() {
                self.schedule_render(&output);
            }
        }
    }
}
