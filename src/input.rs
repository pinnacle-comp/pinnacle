// SPDX-License-Identifier: GPL-3.0-or-later

pub mod libinput;

use std::{
    collections::{HashMap, HashSet},
    mem::Discriminant,
    time::Duration,
};

use crate::{
    focus::{keyboard::KeyboardFocusTarget, pointer::PointerFocusTarget},
    state::{Pinnacle, WithState},
    window::WindowElement,
};
use indexmap::IndexMap;
use pinnacle_api_defs::pinnacle::input::v0alpha1::{
    set_libinput_setting_request::Setting, set_mousebind_request, SetKeybindResponse,
    SetMousebindResponse,
};
use smithay::{
    backend::{
        input::{
            AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent,
            KeyState, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent, PointerMotionEvent,
        },
        renderer::utils::with_renderer_surface_state,
    },
    desktop::{layer_map_for_output, space::SpaceElement, WindowSurfaceType},
    input::{
        keyboard::{keysyms, FilterResult, ModifiersState},
        pointer::{AxisFrame, ButtonEvent, MotionEvent, RelativeMotionEvent},
    },
    reexports::input::{self, Led},
    utils::{IsAlive, Logical, Point, Rectangle, SERIAL_COUNTER},
    wayland::{
        compositor::{self, RegionAttributes, SurfaceAttributes},
        pointer_constraints::{with_pointer_constraint, PointerConstraint},
        seat::WaylandFocus,
        shell::wlr_layer::{self, KeyboardInteractivity, LayerSurfaceCachedState},
    },
};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, info};
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

#[derive(Debug)]
pub struct KeybindData {
    pub sender: UnboundedSender<Result<SetKeybindResponse, tonic::Status>>,
    pub group: Option<String>,
    pub description: Option<String>,
}

#[derive(Default)]
pub struct InputState {
    // TODO: move all of these to config
    pub reload_keybind: Option<(ModifierMask, Keysym)>,
    pub kill_keybind: Option<(ModifierMask, Keysym)>,

    pub keybinds: IndexMap<(ModifierMask, Keysym), KeybindData>,
    pub mousebinds: HashMap<
        (ModifierMask, u32, set_mousebind_request::MouseEdge),
        UnboundedSender<Result<SetMousebindResponse, tonic::Status>>,
    >,
    //--------------------------------------------------
    #[allow(clippy::type_complexity)]
    pub libinput_settings: HashMap<Discriminant<Setting>, Box<dyn Fn(&mut input::Device) + Send>>,
    /// All libinput devices that have been connected
    pub libinput_devices: Vec<input::Device>,

    /// A keyboard focus target stack that is used when there are exclusive keyboard layer
    /// surfaces. When used, the first item is the previous focus before there were any
    /// exclusive layer surfaces.
    // TODO: make that a type or something
    exclusive_layer_focus_stack: Vec<KeyboardFocusTarget>,

    locked_pointer_position_hint: Option<Point<f64, Logical>>,

    // Keys that were used in a keybind and should not be released
    no_release_keys: HashSet<u32>,
}

impl InputState {
    pub fn clear(&mut self) {
        self.reload_keybind = None;
        self.kill_keybind = None;
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
    /// Call a config callback.
    CallCallback(UnboundedSender<Result<SetKeybindResponse, tonic::Status>>),
    /// Quit the compositor.
    Quit,
    /// Switch ttys.
    SwitchVt(i32),
    /// Reload the config.
    ReloadConfig,
    /// Prevent the key from being sent to clients.
    Suppress,
}

impl Pinnacle {
    /// Get the [`PointerFocusTarget`] under `point` along with its origin in the global space.
    pub fn pointer_focus_target_under<P>(
        &self,
        point: P,
    ) -> Option<(PointerFocusTarget, Point<f64, Logical>)>
    where
        P: Into<Point<f64, Logical>>,
    {
        let point: Point<f64, Logical> = point.into();

        let output = self.space.outputs().find(|op| {
            // FIXME: loc is i32
            self.space
                .output_geometry(op)
                .expect("called output_geometry on unmapped output (this shouldn't happen here)")
                .contains(point.to_i32_round())
        })?;

        let output_geo = self
            .space
            .output_geometry(output)
            .expect("called output_geometry on unmapped output");

        if !self.lock_state.is_unlocked() {
            return output
                .with_state(|state| state.lock_surface.clone())
                .map(|lock_surface| {
                    (
                        PointerFocusTarget::WlSurface(lock_surface.wl_surface().clone()),
                        output_geo.loc.to_f64(),
                    )
                });
        }

        let mut fullscreen_and_up_split_at = 0;

        for (i, win) in self
            .space
            .elements()
            .rev()
            .filter(|win| win.is_on_active_tag())
            .enumerate()
        {
            if win.with_state(|state| state.fullscreen_or_maximized.is_fullscreen()) {
                fullscreen_and_up_split_at = i + 1;
            }
        }

        let layer_under =
            |layers: &[wlr_layer::Layer]| -> Option<(PointerFocusTarget, Point<f64, Logical>)> {
                let layer_map = layer_map_for_output(output);
                let layer = layers.iter().find_map(|layer| {
                    layer_map.layer_under(*layer, point - output_geo.loc.to_f64())
                })?;

                let layer_loc = layer_map.layer_geometry(layer)?.loc.to_f64();

                layer
                    .surface_under(
                        point - layer_loc.to_f64() - output_geo.loc.to_f64(),
                        WindowSurfaceType::ALL,
                    )
                    .map(|(surf, surf_loc)| {
                        (
                            PointerFocusTarget::WlSurface(surf),
                            surf_loc.to_f64() + layer_loc + output_geo.loc.to_f64(),
                        )
                    })
            };

        let window_under =
            |windows: &[&WindowElement]| -> Option<(PointerFocusTarget, Point<f64, Logical>)> {
                windows.iter().find_map(|win| {
                    let loc = self
                        .space
                        .element_location(win)
                        .expect("called elem loc on unmapped win")
                        - win.geometry().loc;
                    // FIXME: i32 -> f64
                    let loc = loc.to_f64();

                    win.surface_under(point - loc, WindowSurfaceType::ALL).map(
                        |(surf, surf_loc)| {
                            (PointerFocusTarget::WlSurface(surf), surf_loc.to_f64() + loc)
                        },
                    )
                })
            };

        // Input and rendering go, from top to bottom,
        // - Overlay layer surfaces
        // - All windows down to the bottom-most fullscreen window (this mimics Awesome)
        // - Top layer surfaces
        // - The rest of the windows
        // - Bottom and background layer surfaces

        layer_under(&[wlr_layer::Layer::Overlay])
            .or_else(|| {
                window_under(
                    &self
                        .space
                        .elements()
                        .rev()
                        .filter(|win| win.is_on_active_tag())
                        .take(fullscreen_and_up_split_at)
                        .collect::<Vec<_>>(),
                )
            })
            .or_else(|| layer_under(&[wlr_layer::Layer::Top]))
            .or_else(|| {
                window_under(
                    &self
                        .space
                        .elements()
                        .rev()
                        .filter(|win| win.is_on_active_tag())
                        .skip(fullscreen_and_up_split_at)
                        .collect::<Vec<_>>(),
                )
            })
            .or_else(|| layer_under(&[wlr_layer::Layer::Bottom, wlr_layer::Layer::Background]))
    }
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

    /// Update the pointer focus if it's different from the previous one.
    pub fn update_pointer_focus(&mut self) {
        let Some(pointer) = self.pinnacle.seat.get_pointer() else {
            return;
        };

        let location = pointer.current_location();
        let surface_under = self.pinnacle.pointer_focus_target_under(location);

        // PERF: I'm not really a fan of polling all the time looking for locked pointer
        // updates, but there doesn't seem to be a great way to get the final cursor
        // position hint before destruction. I experimented with a
        // `PointerConstraintsHandler::constraint_destroyed` method but doing so
        // required threading the state through a bunch of different functions.
        // Additionally, `PointerConstraintRef::deactivate` gets called in `WlSurface::leave`,
        // so that would require all compositors implement `PointerConstraintsHandler`
        // which seems very scuffed.
        if pointer.current_focus().as_ref() == surface_under.as_ref().map(|s| &s.0) {
            if let Some((surf, surf_loc)) =
                surface_under.and_then(|(foc, loc)| Some((foc.wl_surface()?.into_owned(), loc)))
            {
                let unlocked = with_pointer_constraint(&surf, &pointer, |constraint| {
                    let Some(constraint) = constraint else {
                        return true;
                    };
                    if !constraint.is_active() {
                        return true;
                    }
                    match &*constraint {
                        PointerConstraint::Confined(_) => true,
                        PointerConstraint::Locked(locked) => {
                            self.pinnacle.input_state.locked_pointer_position_hint =
                                locked.cursor_position_hint();
                            false
                        }
                    }
                });

                if unlocked {
                    if let Some(hint) = self
                        .pinnacle
                        .input_state
                        .locked_pointer_position_hint
                        .take()
                    {
                        self.warp_cursor_to_global_loc(hint + surf_loc.to_f64());
                    }
                }
            }
            return;
        }

        self.pinnacle.maybe_activate_pointer_constraint(location);

        pointer.motion(
            self,
            surface_under,
            &MotionEvent {
                location,
                serial: SERIAL_COUNTER.next_serial(),
                time: Duration::from(self.pinnacle.clock.now()).as_millis() as u32,
            },
        );
        pointer.frame(self);
    }

    /// Warp the cursor to the given `loc` in the global space.
    ///
    /// This is not handled by [`State::pointer_motion`] because I haven't
    /// figured out how thread that through yet.
    pub fn warp_cursor_to_global_loc(&mut self, loc: impl Into<Point<f64, Logical>>) {
        let Some(pointer) = self.pinnacle.seat.get_pointer() else {
            return;
        };
        let loc: Point<f64, Logical> = loc.into();
        self.pinnacle.maybe_activate_pointer_constraint(loc);
        let new_under = self.pinnacle.pointer_focus_target_under(loc);

        pointer.motion(
            self,
            new_under,
            &MotionEvent {
                location: loc,
                serial: SERIAL_COUNTER.next_serial(),
                time: Duration::from(self.pinnacle.clock.now()).as_millis() as u32,
            },
        );

        // TODO: only on outputs that the ptr left and entered
        for output in self.pinnacle.space.outputs().cloned().collect::<Vec<_>>() {
            self.schedule_render(&output);
        }
    }

    fn keyboard<I: InputBackend>(&mut self, event: I::KeyboardKeyEvent) {
        self.pinnacle
            .idle_notifier_state
            .notify_activity(&self.pinnacle.seat);

        let serial = SERIAL_COUNTER.next_serial();
        let time = event.time_msec();
        let press_state = event.state();

        let reload_keybind = self.pinnacle.input_state.reload_keybind;
        let kill_keybind = self.pinnacle.input_state.kill_keybind;

        let keyboard = self
            .pinnacle
            .seat
            .get_keyboard()
            .expect("Seat has no keyboard");

        let modifiers = keyboard.modifier_state();

        let mut leds = Led::empty();
        if modifiers.num_lock {
            leds |= Led::NUMLOCK;
        }
        if modifiers.caps_lock {
            leds |= Led::CAPSLOCK;
        }

        // FIXME: Leds only update once another key is pressed.
        for device in self.pinnacle.input_state.libinput_devices.iter_mut() {
            device.led_update(leds);
        }

        if self.pinnacle.lock_state.is_unlocked() {
            // Handle exclusive layers
            for layer in self.pinnacle.layer_shell_state.layer_surfaces().rev() {
                let data = compositor::with_states(layer.wl_surface(), |states| {
                    *states.cached_state.current::<LayerSurfaceCachedState>()
                });
                if data.keyboard_interactivity == KeyboardInteractivity::Exclusive
                    && matches!(
                        data.layer,
                        wlr_layer::Layer::Top | wlr_layer::Layer::Overlay
                    )
                {
                    let layer_surface = self.pinnacle.space.outputs().find_map(|op| {
                        let map = layer_map_for_output(op);
                        let cloned = map.layers().find(|l| l.layer_surface() == &layer).cloned();
                        cloned
                    });

                    if let Some(layer_surface) = layer_surface {
                        match self.pinnacle.input_state.exclusive_layer_focus_stack.last() {
                            Some(focus) => {
                                let layer_focus = KeyboardFocusTarget::LayerSurface(layer_surface);
                                if &layer_focus != focus {
                                    self.pinnacle
                                        .input_state
                                        .exclusive_layer_focus_stack
                                        .push(layer_focus);
                                }
                            }
                            // Push the previous focus on as this is the first exclusive layer surface
                            // on screen. This lets us restore it when that layer surface goes away.
                            None => {
                                self.pinnacle
                                    .input_state
                                    .exclusive_layer_focus_stack
                                    .extend(keyboard.current_focus());
                                self.pinnacle
                                    .input_state
                                    .exclusive_layer_focus_stack
                                    .push(KeyboardFocusTarget::LayerSurface(layer_surface));
                            }
                        }
                    }
                }
            }

            while let Some(last) = self.pinnacle.input_state.exclusive_layer_focus_stack.pop() {
                if last.alive() {
                    // If it's not empty then there's another exclusive layer surface
                    // underneath. Otherwise `last` is the previous keyboard focus
                    // and we don't need the stack anymore.
                    if !self
                        .pinnacle
                        .input_state
                        .exclusive_layer_focus_stack
                        .is_empty()
                    {
                        self.pinnacle
                            .input_state
                            .exclusive_layer_focus_stack
                            .push(last.clone());
                    }
                    keyboard.set_focus(self, Some(last), serial);
                    break;
                }
            }
        } else {
            // We don't want anything but lock surfaces getting keyboard input when locked
            let lock_surface = self
                .pinnacle
                .space
                .outputs()
                .find_map(|op| op.with_state(|state| state.lock_surface.clone()));

            if !matches!(
                keyboard.current_focus(),
                Some(KeyboardFocusTarget::LockSurface(_))
            ) {
                keyboard.set_focus(
                    self,
                    lock_surface.map(KeyboardFocusTarget::LockSurface),
                    serial,
                );
            }
        }

        let action = keyboard.input(
            self,
            event.key_code(),
            press_state,
            serial,
            time,
            |state, modifiers, keysym| {
                if press_state == KeyState::Released
                    && state
                        .pinnacle
                        .input_state
                        .no_release_keys
                        .contains(&event.key_code())
                {
                    return FilterResult::Intercept(KeyAction::Suppress);
                }

                if press_state == KeyState::Pressed {
                    let mod_mask = ModifierMask::from(modifiers);

                    let raw_sym = keysym.raw_syms().iter().next();
                    let mod_sym = keysym.modified_sym();

                    if let Some(keybind_data) = state
                        .pinnacle
                        .input_state
                        .keybinds
                        .get(&(mod_mask, mod_sym))
                        .or_else(|| {
                            raw_sym.and_then(|raw_sym| {
                                state
                                    .pinnacle
                                    .input_state
                                    .keybinds
                                    .get(&(mod_mask, *raw_sym))
                            })
                        })
                    {
                        if state.pinnacle.lock_state.is_unlocked() {
                            return FilterResult::Intercept(KeyAction::CallCallback(
                                keybind_data.sender.clone(),
                            ));
                        }
                    }

                    if kill_keybind == Some((mod_mask, mod_sym)) {
                        return FilterResult::Intercept(KeyAction::Quit);
                    }

                    if reload_keybind == Some((mod_mask, mod_sym)) {
                        return FilterResult::Intercept(KeyAction::ReloadConfig);
                    }

                    if let mut vt @ keysyms::KEY_XF86Switch_VT_1..=keysyms::KEY_XF86Switch_VT_12 =
                        keysym.modified_sym().raw()
                    {
                        vt = vt - keysyms::KEY_XF86Switch_VT_1 + 1;
                        tracing::info!("Switching to vt {vt}");
                        return FilterResult::Intercept(KeyAction::SwitchVt(vt as i32));
                    }
                }

                FilterResult::Forward
            },
        );

        if let Some(KeyAction::Suppress) = action.as_ref() {
            self.pinnacle
                .input_state
                .no_release_keys
                .remove(&event.key_code());
            return;
        }

        if let Some(action) = action {
            self.pinnacle
                .input_state
                .no_release_keys
                .insert(event.key_code());
            match action {
                KeyAction::CallCallback(sender) => {
                    let _ = sender.send(Ok(SetKeybindResponse {}));
                }
                KeyAction::Quit => {
                    self.pinnacle.shutdown();
                }
                KeyAction::SwitchVt(vt) => {
                    self.switch_vt(vt);
                    self.pinnacle.input_state.no_release_keys.clear();
                }
                KeyAction::ReloadConfig => {
                    info!("Reloading config");
                    self.pinnacle
                        .start_config(false)
                        .expect("failed to restart config");
                }
                KeyAction::Suppress => unreachable!("handled above"),
            }
        }
    }

    fn pointer_button<I: InputBackend>(&mut self, event: I::PointerButtonEvent) {
        self.pinnacle
            .idle_notifier_state
            .notify_activity(&self.pinnacle.seat);

        let Some(pointer) = self.pinnacle.seat.get_pointer() else {
            return;
        };
        let Some(keyboard) = self.pinnacle.seat.get_keyboard() else {
            return;
        };

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
            .pinnacle
            .input_state
            .mousebinds
            .get(&(mod_mask, button, mouse_edge))
        {
            let _ = stream.send(Ok(SetMousebindResponse {}));
            return;
        }

        if button_state == ButtonState::Pressed {
            if let Some((focus, _)) = self.pinnacle.pointer_focus_target_under(pointer_loc) {
                if let Some(window) = focus.window_for(self) {
                    self.pinnacle.raise_window(window.clone(), true);
                    if let Some(output) = window.output(&self.pinnacle) {
                        output.with_state_mut(|state| state.focus_stack.set_focus(window.clone()));
                    }
                }

                if !matches!(
                    focus.window_for(self),
                    Some(window) if window.is_x11_override_redirect()
                ) && focus.popup_for(self).is_none()
                {
                    keyboard.set_focus(self, focus.to_keyboard_focus_target(self), serial);
                }

                for window in self.pinnacle.space.elements() {
                    if let Some(toplevel) = window.toplevel() {
                        toplevel.send_configure();
                    }
                }
            } else {
                if let Some(focused_op) = self.pinnacle.focused_output() {
                    focused_op.with_state_mut(|state| {
                        state.focus_stack.unset_focus();
                        for window in state.focus_stack.stack.iter() {
                            window.set_activate(false);
                            if let Some(toplevel) = window.toplevel() {
                                toplevel.send_configure();
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
        self.pinnacle
            .idle_notifier_state
            .notify_activity(&self.pinnacle.seat);

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

        let pointer = self
            .pinnacle
            .seat
            .get_pointer()
            .expect("Seat has no pointer");

        pointer.axis(self, frame);
        pointer.frame(self);
    }

    /// Handle an absolute pointer motion event.
    ///
    /// This *should* only be generated on the winit backend.
    /// Unless there's a case where it's generated on udev that I'm unaware of.
    fn pointer_motion_absolute<I: InputBackend>(&mut self, event: I::PointerMotionAbsoluteEvent) {
        self.pinnacle
            .idle_notifier_state
            .notify_activity(&self.pinnacle.seat);

        let Some(pointer) = self.pinnacle.seat.get_pointer() else {
            error!("Pointer motion absolute received with no pointer on seat");
            return;
        };

        let Some(output) = self.pinnacle.space.outputs().next() else {
            return;
        };

        let Some(output_geo) = self.pinnacle.space.output_geometry(output) else {
            unreachable!("output should have a geometry as it was mapped");
        };

        let pointer_loc = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();
        let serial = SERIAL_COUNTER.next_serial();

        if let Some(output) = self
            .pinnacle
            .space
            .output_under(pointer_loc)
            .next()
            .cloned()
        {
            self.pinnacle.output_focus_stack.set_focus(output);
        }

        self.pinnacle.maybe_activate_pointer_constraint(pointer_loc);

        let pointer_focus = self.pinnacle.pointer_focus_target_under(pointer_loc);

        pointer.motion(
            self,
            pointer_focus,
            &MotionEvent {
                location: pointer_loc,
                serial,
                time: event.time_msec(),
            },
        );

        pointer.frame(self);

        if let Some(output) = self.pinnacle.focused_output().cloned() {
            self.schedule_render(&output);
        }
    }

    fn pointer_motion<I: InputBackend>(&mut self, event: I::PointerMotionEvent) {
        self.pinnacle
            .idle_notifier_state
            .notify_activity(&self.pinnacle.seat);

        let Some(pointer) = self.pinnacle.seat.get_pointer() else {
            error!("Pointer motion received with no pointer on seat");
            return;
        };

        let pointer_loc = pointer.current_location();

        let mut pointer_confined_to: Option<(
            PointerFocusTarget,
            Point<f64, Logical>,
            Option<RegionAttributes>,
        )> = None;

        let current_under = self.pinnacle.pointer_focus_target_under(pointer_loc);

        // TODO: possibly cache the current pointer focus and location?
        if let Some((surface, surface_loc)) = &current_under {
            let surface_loc = *surface_loc;
            if let Some(wl_surface) = surface.wl_surface() {
                let mut pointer_locked = false;

                with_pointer_constraint(&wl_surface, &pointer, |constraint| {
                    let Some(constraint) = constraint else {
                        return;
                    };
                    tracing::debug!(constraint = ?*constraint);
                    if !constraint.is_active() {
                        return;
                    }

                    let pointer_loc_relative_to_surf = pointer_loc - surface_loc;

                    // Constraint does not apply if not within region.
                    if let Some(region) = constraint.region() {
                        if !region.contains(pointer_loc_relative_to_surf.to_i32_round()) {
                            return;
                        }
                    }

                    match &*constraint {
                        PointerConstraint::Confined(confined) => {
                            pointer_confined_to =
                                Some((surface.clone(), surface_loc, confined.region().cloned()));
                        }
                        PointerConstraint::Locked(_) => {
                            pointer_locked = true;
                        }
                    }
                });

                if pointer_locked {
                    pointer.relative_motion(
                        self,
                        Some((surface.clone(), surface_loc)),
                        &RelativeMotionEvent {
                            delta: event.delta(),
                            delta_unaccel: event.delta_unaccel(),
                            utime: event.time(),
                        },
                    );

                    pointer.frame(self);

                    return;
                }
            }
        }

        let mut new_pointer_loc = pointer_loc + event.delta();

        // Place the pointer inside the nearest output if it would be outside one
        if self
            .pinnacle
            .space
            .output_under(new_pointer_loc)
            .next()
            .is_none()
        {
            let output_locs = self
                .pinnacle
                .space
                .outputs()
                .flat_map(|op| self.pinnacle.space.output_geometry(op));
            new_pointer_loc = constrain_point_inside_rects(new_pointer_loc, output_locs);
        }

        let new_under = self.pinnacle.pointer_focus_target_under(new_pointer_loc);

        // FIXME: rework this for float locs
        if let Some((focus, surf_loc, region)) = &pointer_confined_to {
            let region = region
                .clone()
                .or_else(|| {
                    compositor::with_states(&*focus.wl_surface()?, |states| {
                        states
                            .cached_state
                            .current::<SurfaceAttributes>()
                            .input_region
                            .clone()
                    })
                })
                .or_else(|| {
                    // No region or input region means constrain within the whole surface
                    let surface_size =
                        with_renderer_surface_state(&*focus.wl_surface()?, |state| {
                            state.surface_size()
                        })??;

                    let mut attrs = RegionAttributes::default();
                    attrs.rects.push((
                        compositor::RectangleKind::Add,
                        Rectangle::from_loc_and_size((0, 0), surface_size),
                    ));
                    Some(attrs)
                })
                .unwrap_or_default();

            let mut region_rects = Vec::<Rectangle<i32, Logical>>::new();

            for (kind, mut rect) in region.rects {
                // make loc global
                // FIXME: f64 -> i32
                rect.loc += surf_loc.to_i32_round();
                // PERF: Who knows how out of hand this can get lol
                match kind {
                    compositor::RectangleKind::Add => {
                        region_rects.push(rect);
                    }
                    compositor::RectangleKind::Subtract => {
                        region_rects =
                            Rectangle::subtract_rects_many_in_place(region_rects, [rect]);
                    }
                }
            }

            new_pointer_loc = constrain_point_inside_rects(new_pointer_loc, region_rects);
        }

        self.pinnacle
            .maybe_activate_pointer_constraint(new_pointer_loc);

        if let Some(output) = self
            .pinnacle
            .space
            .output_under(new_pointer_loc)
            .next()
            .cloned()
        {
            self.pinnacle.output_focus_stack.set_focus(output);
        }

        let focus_target = pointer_confined_to
            .map(|(focus, loc, _)| (focus, loc))
            .or(new_under);

        pointer.motion(
            self,
            focus_target.clone(),
            &MotionEvent {
                location: new_pointer_loc,
                serial: SERIAL_COUNTER.next_serial(),
                time: event.time_msec(),
            },
        );

        pointer.relative_motion(
            self,
            focus_target,
            &RelativeMotionEvent {
                delta: event.delta(),
                delta_unaccel: event.delta_unaccel(),
                utime: event.time(),
            },
        );

        pointer.frame(self);

        if let Some(output) = self.pinnacle.focused_output().cloned() {
            self.schedule_render(&output);
        }
    }
}

/// Clamp the given point within the given rects.
///
/// This returns the nearest point inside the rects.
fn constrain_point_inside_rects(
    pos: Point<f64, Logical>,
    rects: impl IntoIterator<Item = Rectangle<i32, Logical>>,
) -> Point<f64, Logical> {
    let (pos_x, pos_y) = pos.into();

    let nearest_points = rects.into_iter().map(|rect| {
        let pos = pos.constrain(rect.to_f64());
        (rect, pos.x, pos.y)
    });

    let nearest_point = nearest_points.min_by(|(_, x1, y1), (_, x2, y2)| {
        f64::total_cmp(
            &((pos_x - x1).powi(2) + (pos_y - y1).powi(2)).sqrt(),
            &((pos_x - x2).powi(2) + (pos_y - y2).powi(2)).sqrt(),
        )
    });

    nearest_point
        .map(|(rect, mut x, mut y)| {
            let rect = rect.to_f64();

            // Clamp the point to actually be in the rect and not
            // touching its edge.
            x = f64::min(x, rect.loc.x + rect.size.w - 1.0);
            y = f64::min(y, rect.loc.y + rect.size.h - 1.0);

            (x, y).into()
        })
        .unwrap_or(pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(loc: (i32, i32), size: (i32, i32)) -> Rectangle<i32, Logical> {
        Rectangle::from_loc_and_size(loc, size)
    }

    #[test]
    fn constrain_point_inside_rects_single_rect() {
        let rects = [rect((300, 300), (300, 300))];
        assert_eq!(
            constrain_point_inside_rects((0.0, 0.0).into(), rects),
            (300.0, 300.0).into(),
            "top left failed"
        );
        assert_eq!(
            constrain_point_inside_rects((450.0, 0.0).into(), rects),
            (450.0, 300.0).into(),
            "top failed"
        );
        assert_eq!(
            constrain_point_inside_rects((750.0, 0.0).into(), rects),
            (599.0, 300.0).into(),
            "top right failed"
        );
        assert_eq!(
            constrain_point_inside_rects((0.0, 450.0).into(), rects),
            (300.0, 450.0).into(),
            "left failed"
        );
        assert_eq!(
            constrain_point_inside_rects((450.0, 450.0).into(), rects),
            (450.0, 450.0).into(),
            "center failed"
        );
        assert_eq!(
            constrain_point_inside_rects((750.0, 450.0).into(), rects),
            (599.0, 450.0).into(),
            "right failed"
        );
        assert_eq!(
            constrain_point_inside_rects((0.0, 750.0).into(), rects),
            (300.0, 599.0).into(),
            "bottom left failed"
        );
        assert_eq!(
            constrain_point_inside_rects((450.0, 750.0).into(), rects),
            (450.0, 599.0).into(),
            "bottom failed"
        );
        assert_eq!(
            constrain_point_inside_rects((750.0, 750.0).into(), rects),
            (599.0, 599.0).into(),
            "bottom right failed"
        );
    }

    #[test]
    fn constrain_point_inside_rects_multiple_rects() {
        let rects = [rect((300, 300), (300, 300)), rect((900, 900), (300, 300))];
        assert_eq!(
            constrain_point_inside_rects((750.0, 750.0).into(), rects),
            (599.0, 599.0).into(),
            "equal distance favoring first rect failed"
        );
        assert_eq!(
            constrain_point_inside_rects((700.0, 700.0).into(), rects),
            (599.0, 599.0).into(),
            "closer to first rect failed"
        );
        assert_eq!(
            constrain_point_inside_rects((800.0, 800.0).into(), rects),
            (900.0, 900.0).into(),
            "closer to second rect failed"
        );
    }
}
