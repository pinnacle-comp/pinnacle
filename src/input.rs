// SPDX-License-Identifier: GPL-3.0-or-later

pub mod bind;
pub mod libinput;

use std::time::Duration;

use crate::{
    focus::{keyboard::KeyboardFocusTarget, pointer::PointerFocusTarget},
    state::{Pinnacle, WithState},
    window::WindowElement,
};
use bind::BindState;
use libinput::LibinputState;
use smithay::{
    backend::{
        input::{
            AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, GestureBeginEvent,
            GestureEndEvent, InputBackend, InputEvent, KeyState, KeyboardKeyEvent,
            PointerAxisEvent, PointerButtonEvent, PointerMotionEvent,
        },
        renderer::utils::with_renderer_surface_state,
    },
    desktop::{layer_map_for_output, space::SpaceElement, WindowSurfaceType},
    input::{
        keyboard::{keysyms, FilterResult, ModifiersState},
        pointer::{
            AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent,
            GesturePinchBeginEvent, GesturePinchEndEvent, GesturePinchUpdateEvent,
            GestureSwipeBeginEvent, GestureSwipeEndEvent, GestureSwipeUpdateEvent, MotionEvent,
            RelativeMotionEvent,
        },
    },
    utils::{Logical, Point, Rectangle, SERIAL_COUNTER},
    wayland::{
        compositor::{self, RegionAttributes, SurfaceAttributes},
        keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitorSeat,
        pointer_constraints::{with_pointer_constraint, PointerConstraint},
        seat::WaylandFocus,
        shell::wlr_layer::{self, KeyboardInteractivity, LayerSurfaceCachedState},
    },
};
use tracing::{error, info};

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

#[derive(Default, Debug)]
pub struct InputState {
    pub bind_state: BindState,
    pub libinput_state: LibinputState,
}

impl InputState {
    pub fn clear(&mut self) {
        self.bind_state.clear();
    }
}

impl InputState {
    pub fn new() -> Self {
        Default::default()
    }
}

#[derive(Debug)]
enum KeyAction {
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
            if win.with_state(|state| state.window_state.is_fullscreen()) {
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
        self.pinnacle
            .idle_notifier_state
            .notify_activity(&self.pinnacle.seat);

        match event {
            // InputEvent::DeviceAdded { device } => todo!(),
            // InputEvent::DeviceRemoved { device } => todo!(),
            InputEvent::Keyboard { event } => self.on_keyboard::<B>(event),

            InputEvent::PointerMotion { event } => self.on_pointer_motion::<B>(event),
            InputEvent::PointerMotionAbsolute { event } => {
                self.on_pointer_motion_absolute::<B>(event)
            }
            InputEvent::PointerButton { event } => self.on_pointer_button::<B>(event),
            InputEvent::PointerAxis { event } => self.on_pointer_axis::<B>(event),

            InputEvent::GestureSwipeBegin { event } => self.on_gesture_swipe_begin::<B>(event),
            InputEvent::GestureSwipeUpdate { event } => self.on_gesture_swipe_update::<B>(event),
            InputEvent::GestureSwipeEnd { event } => self.on_gesture_swipe_end::<B>(event),
            InputEvent::GesturePinchBegin { event } => self.on_gesture_pinch_begin::<B>(event),
            InputEvent::GesturePinchUpdate { event } => self.on_gesture_pinch_update::<B>(event),
            InputEvent::GesturePinchEnd { event } => self.on_gesture_pinch_end::<B>(event),
            InputEvent::GestureHoldBegin { event } => self.on_gesture_hold_begin::<B>(event),
            InputEvent::GestureHoldEnd { event } => self.on_gesture_hold_end::<B>(event),

            // TODO: rest of input events
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

        if pointer.current_focus().as_ref() == surface_under.as_ref().map(|s| &s.0) {
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

    fn on_keyboard<I: InputBackend>(&mut self, event: I::KeyboardKeyEvent) {
        let serial = SERIAL_COUNTER.next_serial();
        let time = event.time_msec();
        let press_state = event.state();

        let keyboard = self
            .pinnacle
            .seat
            .get_keyboard()
            .expect("Seat has no keyboard");

        if self.pinnacle.lock_state.is_unlocked() {
            // Focus the topmost exclusive layer, if any
            for layer in self.pinnacle.layer_shell_state.layer_surfaces().rev() {
                let data = compositor::with_states(layer.wl_surface(), |states| {
                    *states
                        .cached_state
                        .get::<LayerSurfaceCachedState>()
                        .current()
                });
                if data.keyboard_interactivity == KeyboardInteractivity::Exclusive
                    && matches!(
                        data.layer,
                        wlr_layer::Layer::Top | wlr_layer::Layer::Overlay
                    )
                {
                    let layer_surface = self.pinnacle.space.outputs().find_map(|op| {
                        let map = layer_map_for_output(op);
                        let layer = map.layers().find(|l| l.layer_surface() == &layer).cloned();
                        layer
                    });

                    if let Some(layer_surface) = layer_surface {
                        keyboard.set_focus(
                            self,
                            Some(KeyboardFocusTarget::LayerSurface(layer_surface)),
                            serial,
                        );
                        break;
                    }
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

        let shortcuts_inhibited = keyboard
            .current_focus()
            .and_then(|focus| {
                focus.wl_surface().and_then(|surf| {
                    self.pinnacle
                        .seat
                        .keyboard_shortcuts_inhibitor_for_surface(&surf)
                })
            })
            .is_some_and(|inhibitor| inhibitor.is_active());

        let action = keyboard.input(
            self,
            event.key_code(),
            press_state,
            serial,
            time,
            |state, modifiers, keysym| {
                if press_state == KeyState::Pressed {
                    if let mut vt @ keysyms::KEY_XF86Switch_VT_1..=keysyms::KEY_XF86Switch_VT_12 =
                        keysym.modified_sym().raw()
                    {
                        vt = vt - keysyms::KEY_XF86Switch_VT_1 + 1;
                        return FilterResult::Intercept(KeyAction::SwitchVt(vt as i32));
                    }
                }

                if shortcuts_inhibited {
                    return FilterResult::Forward;
                }

                let Some(raw_sym) = keysym.raw_latin_sym_or_raw_current_sym() else {
                    return FilterResult::Forward;
                };

                let edge = match press_state {
                    KeyState::Released => bind::Edge::Release,
                    KeyState::Pressed => bind::Edge::Press,
                };

                let bind_action = state.pinnacle.input_state.bind_state.keybinds.key(
                    raw_sym,
                    *modifiers,
                    edge,
                    state.pinnacle.input_state.bind_state.current_layer(),
                );

                match bind_action {
                    bind::BindAction::Forward => FilterResult::Forward,
                    bind::BindAction::Suppress => FilterResult::Intercept(KeyAction::Suppress),
                    bind::BindAction::Quit => FilterResult::Intercept(KeyAction::Quit),
                    bind::BindAction::ReloadConfig => {
                        FilterResult::Intercept(KeyAction::ReloadConfig)
                    }
                }
            },
        );

        if let Some(action) = action {
            match action {
                KeyAction::Quit => {
                    self.pinnacle.shutdown();
                }
                KeyAction::SwitchVt(vt) => {
                    self.switch_vt(vt);
                    self.pinnacle
                        .input_state
                        .bind_state
                        .keybinds
                        .last_pressed_triggered_binds
                        .clear();
                }
                KeyAction::ReloadConfig => {
                    info!("Reloading config");
                    self.pinnacle
                        .start_config(false)
                        .expect("failed to restart config");
                }
                KeyAction::Suppress => (),
            }
        }
    }

    fn on_pointer_button<I: InputBackend>(&mut self, event: I::PointerButtonEvent) {
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

        let mods = keyboard.modifier_state();

        let edge = match button_state {
            ButtonState::Released => bind::Edge::Release,
            ButtonState::Pressed => bind::Edge::Press,
        };

        let current_layer = self.pinnacle.input_state.bind_state.current_layer();
        let bind_action =
            self.pinnacle
                .input_state
                .bind_state
                .mousebinds
                .btn(button, mods, edge, current_layer);

        match bind_action {
            bind::BindAction::Forward => (),
            bind::BindAction::Suppress => {
                if !pointer.is_grabbed() {
                    return;
                }
            }
            bind::BindAction::Quit => {
                self.pinnacle.shutdown();
                return;
            }
            bind::BindAction::ReloadConfig => {
                info!("Reloading config");
                self.pinnacle
                    .start_config(false)
                    .expect("failed to restart config");
                return;
            }
        }

        if button_state == ButtonState::Pressed {
            if let Some((focus, _)) = self.pinnacle.pointer_focus_target_under(pointer_loc) {
                if let Some(window) = focus.window_for(self) {
                    self.pinnacle.raise_window(window.clone(), true);
                    if !window.is_x11_override_redirect() {
                        if let Some(output) = window.output(&self.pinnacle) {
                            output.with_state_mut(|state| {
                                state.focus_stack.set_focus(window.clone())
                            });
                            self.update_keyboard_focus(&output);
                        }
                    }
                    for output in self.pinnacle.space.outputs_for_element(&window) {
                        self.schedule_render(&output);
                    }
                } else if let Some(layer) = focus.layer_for(self) {
                    if layer.can_receive_keyboard_focus() {
                        keyboard.set_focus(
                            self,
                            Some(KeyboardFocusTarget::LayerSurface(layer)),
                            serial,
                        );
                    }
                }
            } else if let Some(focused_op) = self.pinnacle.focused_output().cloned() {
                focused_op.with_state_mut(|state| {
                    state.focus_stack.unset_focus();
                });
                self.update_keyboard_focus(&focused_op);
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

    fn on_pointer_axis<I: InputBackend>(&mut self, event: I::PointerAxisEvent) {
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
    fn on_pointer_motion_absolute<I: InputBackend>(
        &mut self,
        event: I::PointerMotionAbsoluteEvent,
    ) {
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

    fn on_pointer_motion<I: InputBackend>(&mut self, event: I::PointerMotionEvent) {
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
                            .get::<SurfaceAttributes>()
                            .current()
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

    fn on_gesture_swipe_begin<I: InputBackend>(&mut self, event: I::GestureSwipeBeginEvent) {
        let Some(pointer) = self.pinnacle.seat.get_pointer() else {
            return;
        };

        pointer.gesture_swipe_begin(
            self,
            &GestureSwipeBeginEvent {
                serial: SERIAL_COUNTER.next_serial(),
                time: event.time_msec(),
                fingers: event.fingers(),
            },
        );
    }

    fn on_gesture_swipe_update<I: InputBackend>(&mut self, event: I::GestureSwipeUpdateEvent) {
        let Some(pointer) = self.pinnacle.seat.get_pointer() else {
            return;
        };

        use smithay::backend::input::GestureSwipeUpdateEvent as _;

        pointer.gesture_swipe_update(
            self,
            &GestureSwipeUpdateEvent {
                time: event.time_msec(),
                delta: event.delta(),
            },
        );
    }

    fn on_gesture_swipe_end<I: InputBackend>(&mut self, event: I::GestureSwipeEndEvent) {
        let Some(pointer) = self.pinnacle.seat.get_pointer() else {
            return;
        };

        pointer.gesture_swipe_end(
            self,
            &GestureSwipeEndEvent {
                serial: SERIAL_COUNTER.next_serial(),
                time: event.time_msec(),
                cancelled: event.cancelled(),
            },
        );
    }

    fn on_gesture_pinch_begin<I: InputBackend>(&mut self, event: I::GesturePinchBeginEvent) {
        let Some(pointer) = self.pinnacle.seat.get_pointer() else {
            return;
        };

        pointer.gesture_pinch_begin(
            self,
            &GesturePinchBeginEvent {
                serial: SERIAL_COUNTER.next_serial(),
                time: event.time_msec(),
                fingers: event.fingers(),
            },
        );
    }

    fn on_gesture_pinch_update<I: InputBackend>(&mut self, event: I::GesturePinchUpdateEvent) {
        let Some(pointer) = self.pinnacle.seat.get_pointer() else {
            return;
        };

        use smithay::backend::input::GesturePinchUpdateEvent as _;

        pointer.gesture_pinch_update(
            self,
            &GesturePinchUpdateEvent {
                time: event.time_msec(),
                delta: event.delta(),
                scale: event.scale(),
                rotation: event.rotation(),
            },
        );
    }

    fn on_gesture_pinch_end<I: InputBackend>(&mut self, event: I::GesturePinchEndEvent) {
        let Some(pointer) = self.pinnacle.seat.get_pointer() else {
            return;
        };

        pointer.gesture_pinch_end(
            self,
            &GesturePinchEndEvent {
                serial: SERIAL_COUNTER.next_serial(),
                time: event.time_msec(),
                cancelled: event.cancelled(),
            },
        );
    }

    fn on_gesture_hold_begin<I: InputBackend>(&mut self, event: I::GestureHoldBeginEvent) {
        let Some(pointer) = self.pinnacle.seat.get_pointer() else {
            return;
        };

        pointer.gesture_hold_begin(
            self,
            &GestureHoldBeginEvent {
                serial: SERIAL_COUNTER.next_serial(),
                time: event.time_msec(),
                fingers: event.fingers(),
            },
        );
    }

    fn on_gesture_hold_end<I: InputBackend>(&mut self, event: I::GestureHoldEndEvent) {
        let Some(pointer) = self.pinnacle.seat.get_pointer() else {
            return;
        };

        pointer.gesture_hold_end(
            self,
            &GestureHoldEndEvent {
                serial: SERIAL_COUNTER.next_serial(),
                time: event.time_msec(),
                cancelled: event.cancelled(),
            },
        );
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
