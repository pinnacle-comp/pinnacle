// SPDX-License-Identifier: GPL-3.0-or-later

use smithay::{
    desktop::space::SpaceElement,
    // NOTE: maybe alias this to PointerGrabStartData because there's another GrabStartData in
    // |     input::keyboard
    input::{
        pointer::{
            AxisFrame, ButtonEvent, CursorIcon, CursorImageStatus, Focus, GestureHoldBeginEvent,
            GestureHoldEndEvent, GesturePinchBeginEvent, GesturePinchEndEvent,
            GesturePinchUpdateEvent, GestureSwipeBeginEvent, GestureSwipeEndEvent,
            GestureSwipeUpdateEvent, GrabStartData, MotionEvent, PointerGrab, PointerInnerHandle,
            RelativeMotionEvent,
        },
        Seat, SeatHandler,
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{IsAlive, Logical, Point, Rectangle, Serial},
};
use tracing::{debug, warn};

use crate::{
    state::{State, WithState},
    window::WindowElement,
};

/// Data for moving a window.
pub struct MoveSurfaceGrab {
    pub start_data: GrabStartData<State>,
    /// The window being moved
    pub window: WindowElement,
    pub initial_window_loc: Point<f64, Logical>,
}

impl PointerGrab<State> for MoveSurfaceGrab {
    fn frame(&mut self, data: &mut State, handle: &mut PointerInnerHandle<'_, State>) {
        handle.frame(data);
    }

    fn motion(
        &mut self,
        state: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        _focus: Option<(<State as SeatHandler>::PointerFocus, Point<f64, Logical>)>,
        event: &MotionEvent,
    ) {
        handle.motion(state, None, event);

        if !self.window.alive() {
            state
                .pinnacle
                .cursor_state
                .set_cursor_image(CursorImageStatus::default_named());
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

        let is_floating = self
            .window
            .with_state(|state| state.window_state.is_floating());

        if is_floating {
            let tag_output = self.window.output(&state.pinnacle);
            if let Some(focused_output) = state.pinnacle.focused_output() {
                if Some(focused_output) != tag_output.as_ref() {
                    state
                        .pinnacle
                        .place_window_on_output(&self.window, focused_output);
                }
            }

            let delta = event.location - self.start_data.location;
            let new_loc = self.initial_window_loc.to_f64() + delta;
            // FIXME: space maps locs as i32 not f64
            state
                .pinnacle
                .space
                .map_element(self.window.clone(), new_loc.to_i32_round(), true);

            self.window.with_state_mut(|state| {
                state.floating_loc = Some(new_loc);
            });

            if let Some(surface) = self.window.x11_surface() {
                if !surface.is_override_redirect() {
                    let geo = surface.geometry();
                    // FIXME: prolly not fixable but xwayland configures with loc i32 not f64
                    let new_geo = Rectangle::new(new_loc.to_i32_round(), geo.size);
                    surface
                        .configure(new_geo)
                        .expect("failed to configure x11 win");
                }
            }

            let outputs = state.pinnacle.space.outputs_for_element(&self.window);
            for output in outputs {
                state.schedule_render(&output);
            }
        } else {
            let tag_output = self.window.output(&state.pinnacle);
            if let Some(focused_output) = state.pinnacle.focused_output().cloned() {
                if Some(&focused_output) != tag_output.as_ref() {
                    state.capture_snapshots_on_output(&focused_output, []);
                    if let Some(tag_output) = tag_output.as_ref() {
                        state.capture_snapshots_on_output(tag_output, []);
                    }

                    state
                        .pinnacle
                        .place_window_on_output(&self.window, &focused_output);

                    state.pinnacle.begin_layout_transaction(&focused_output);
                    state.pinnacle.request_layout(&focused_output);
                    if let Some(tag_output) = tag_output {
                        state.pinnacle.begin_layout_transaction(&tag_output);
                        state.pinnacle.request_layout(&tag_output);
                    }
                }
            }

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

                if window_under.with_state(|state| state.window_state.is_floating()) {
                    return;
                }

                let output = self.window.output(&state.pinnacle);

                // HACK: Snapshots may not be cleared and updated when swapping two windows of the same size;
                // this causes new snapshots attempts to fizzle and the currently stored snapshot
                // will have the wrong location. We're just gonna invalidate all window snapshots here
                // because I'm too lazy to rearchitect stuff to make it more sensible.
                for window in state.pinnacle.windows.iter() {
                    window.with_state_mut(|state| state.snapshot.take());
                }

                if let Some(output) = output.as_ref() {
                    state.capture_snapshots_on_output(output, [self.window.clone()]);
                }

                debug!("Swapping window positions");
                state
                    .pinnacle
                    .swap_window_positions(&self.window, &window_under);

                state.pinnacle.layout_state.pending_swap = true;

                if let Some(output) = output.as_ref() {
                    state.pinnacle.begin_layout_transaction(output);
                    state.pinnacle.request_layout(output);
                }
            }
        }
    }

    fn relative_motion(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        focus: Option<(<State as SeatHandler>::PointerFocus, Point<f64, Logical>)>,
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
            data.pinnacle
                .cursor_state
                .set_cursor_image(CursorImageStatus::default_named());
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
                .expect("move request was called on an unmapped window")
                .to_f64();

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
            .expect("move request was called on an unmapped window")
            .to_f64(); // TODO: add space f64 support or move away from space

        let start_data = smithay::input::pointer::GrabStartData {
            // If Some and same as the dragged window then the window is allowed to
            // change the cursor, which we don't want, therefore this is None
            focus: None,
            button: button_used,
            location: pointer.current_location(),
        };

        let grab = MoveSurfaceGrab {
            start_data,
            window,
            initial_window_loc,
        };

        pointer.set_grab(self, grab, serial, Focus::Clear);

        self.pinnacle
            .cursor_state
            .set_cursor_image(CursorImageStatus::Named(CursorIcon::Grabbing));
    }
}
