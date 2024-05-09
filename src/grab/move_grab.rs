// SPDX-License-Identifier: GPL-3.0-or-later

use smithay::{
    desktop::space::SpaceElement,
    // NOTE: maybe alias this to PointerGrabStartData because there's another GrabStartData in
    // |     input::keyboard
    input::{
        pointer::{
            AxisFrame, ButtonEvent, Focus, GestureHoldBeginEvent, GestureHoldEndEvent,
            GesturePinchBeginEvent, GesturePinchEndEvent, GesturePinchUpdateEvent,
            GestureSwipeBeginEvent, GestureSwipeEndEvent, GestureSwipeUpdateEvent, GrabStartData,
            MotionEvent, PointerGrab, PointerInnerHandle, RelativeMotionEvent,
        },
        Seat, SeatHandler,
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{IsAlive, Logical, Point, Rectangle, Serial},
};
use tracing::{debug, warn};

use crate::{
    state::{State, WithState},
    window::{window_state::FloatingOrTiled, WindowElement},
};

/// Data for moving a window.
pub struct MoveSurfaceGrab {
    pub start_data: GrabStartData<State>,
    /// The window being moved
    pub window: WindowElement,
    pub initial_window_loc: Point<i32, Logical>,
}

impl PointerGrab<State> for MoveSurfaceGrab {
    fn frame(&mut self, data: &mut State, handle: &mut PointerInnerHandle<'_, State>) {
        handle.frame(data);
    }

    fn motion(
        &mut self,
        state: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        _focus: Option<(<State as SeatHandler>::PointerFocus, Point<i32, Logical>)>,
        event: &MotionEvent,
    ) {
        handle.motion(state, None, event);

        if !self.window.alive() {
            handle.unset_grab(self, state, event.serial, event.time, true);
            return;
        }

        state.pinnacle.raise_window(self.window.clone(), false);

        if let Some(surface) = self.window.x11_surface() {
            // INFO: can you raise OR windows or no idk
            if !surface.is_override_redirect() {
                state
                    .pinnacle
                    .xwm
                    .as_mut()
                    .expect("no xwm")
                    .raise_window(surface)
                    .expect("failed to raise x11 win");
            }
        }

        let is_tiled = self
            .window
            .with_state(|state| state.floating_or_tiled.is_tiled());

        if is_tiled {
            // INFO: this is being used instead of space.element_under(event.location) because that
            // |     uses the bounding box, which is different from the actual geometry
            let window_under = state
                .pinnacle
                .space
                .elements()
                .filter(|win| win.is_on_active_tag())
                .rev()
                .find(|&win| {
                    if let Some(loc) = state.pinnacle.space.element_location(win) {
                        let size = win.geometry().size;
                        let rect = Rectangle { size, loc };
                        rect.contains(event.location.to_i32_round())
                    } else {
                        false
                    }
                })
                .cloned();

            if let Some(window_under) = window_under {
                if state.pinnacle.layout_state.pending_swap {
                    return;
                }

                if window_under == self.window {
                    return;
                }

                if window_under.with_state(|state| {
                    state.floating_or_tiled.is_floating() || state.target_loc.is_some()
                }) {
                    return;
                }

                debug!("Swapping window positions");
                state
                    .pinnacle
                    .swap_window_positions(&self.window, &window_under);
            }
        } else {
            let delta = event.location - self.start_data.location;
            let new_loc = (self.initial_window_loc.to_f64() + delta).to_i32_round();
            state
                .pinnacle
                .space
                .map_element(self.window.clone(), new_loc, true);

            let size = state
                .pinnacle
                .space
                .element_geometry(&self.window)
                .expect("window wasn't mapped")
                .size;

            self.window.with_state_mut(|state| {
                if state.floating_or_tiled.is_floating() {
                    state.floating_or_tiled =
                        FloatingOrTiled::Floating(Rectangle::from_loc_and_size(new_loc, size));
                }
            });

            if let Some(surface) = self.window.x11_surface() {
                if !surface.is_override_redirect() {
                    let geo = surface.geometry();
                    let new_geo = Rectangle::from_loc_and_size(new_loc, geo.size);
                    surface
                        .configure(new_geo)
                        .expect("failed to configure x11 win");
                }
            }

            let outputs = state.pinnacle.space.outputs_for_element(&self.window);
            for output in outputs {
                state.schedule_render(&output);
            }
        }
    }

    fn relative_motion(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        focus: Option<(<State as SeatHandler>::PointerFocus, Point<i32, Logical>)>,
        event: &RelativeMotionEvent,
    ) {
        handle.relative_motion(data, focus, event);
    }

    fn button(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &ButtonEvent,
    ) {
        handle.button(data, event);

        if !handle.current_pressed().contains(&self.start_data.button) {
            handle.unset_grab(self, data, event.serial, event.time, true);
        }
    }

    fn axis(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        details: AxisFrame,
    ) {
        handle.axis(data, details);
    }

    fn start_data(&self) -> &GrabStartData<State> {
        &self.start_data
    }

    fn unset(&mut self, _data: &mut State) {}

    fn gesture_swipe_begin(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GestureSwipeBeginEvent,
    ) {
        handle.gesture_swipe_begin(data, event);
    }

    fn gesture_swipe_update(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GestureSwipeUpdateEvent,
    ) {
        handle.gesture_swipe_update(data, event);
    }

    fn gesture_swipe_end(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GestureSwipeEndEvent,
    ) {
        handle.gesture_swipe_end(data, event);
    }

    fn gesture_pinch_begin(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GesturePinchBeginEvent,
    ) {
        handle.gesture_pinch_begin(data, event);
    }

    fn gesture_pinch_update(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GesturePinchUpdateEvent,
    ) {
        handle.gesture_pinch_update(data, event);
    }

    fn gesture_pinch_end(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GesturePinchEndEvent,
    ) {
        handle.gesture_pinch_end(data, event);
    }

    fn gesture_hold_begin(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GestureHoldBeginEvent,
    ) {
        handle.gesture_hold_begin(data, event);
    }

    fn gesture_hold_end(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GestureHoldEndEvent,
    ) {
        handle.gesture_hold_end(data, event);
    }
}

impl State {
    /// The application initiated a move grab e.g. when you drag a titlebar.
    pub fn move_request_client(&mut self, surface: &WlSurface, seat: &Seat<State>, serial: Serial) {
        let pointer = seat.get_pointer().expect("seat had no pointer");
        if let Some(start_data) = crate::grab::pointer_grab_start_data(&pointer, surface, serial) {
            let Some(window) = self.pinnacle.window_for_surface(surface) else {
                warn!("Surface had no window, cancelling move request");
                return;
            };

            let initial_window_loc = self
                .pinnacle
                .space
                .element_location(&window)
                .expect("move request was called on an unmapped window");

            let grab = MoveSurfaceGrab {
                start_data,
                window,
                initial_window_loc,
            };

            pointer.set_grab(self, grab, serial, Focus::Clear);
        } else {
            debug!("No grab start data for grab, cancelling");
        }
    }

    /// The compositor initiated a move grab e.g. you hold the mod key and drag.
    pub fn move_request_server(
        &mut self,
        surface: &WlSurface,
        seat: &Seat<State>,
        serial: Serial,
        button_used: u32,
    ) {
        let pointer = seat.get_pointer().expect("seat had no pointer");
        let Some(window) = self.pinnacle.window_for_surface(surface) else {
            warn!("Surface had no window, cancelling move request");
            return;
        };

        let initial_window_loc = self
            .pinnacle
            .space
            .element_location(&window)
            .expect("move request was called on an unmapped window");

        let start_data = smithay::input::pointer::GrabStartData {
            focus: pointer
                .current_focus()
                .map(|focus| (focus, initial_window_loc)),
            button: button_used,
            location: pointer.current_location(),
        };

        let grab = MoveSurfaceGrab {
            start_data,
            window,
            initial_window_loc,
        };

        pointer.set_grab(self, grab, serial, Focus::Clear);
    }
}
