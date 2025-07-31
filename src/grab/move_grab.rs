// SPDX-License-Identifier: GPL-3.0-or-later

use smithay::{
    // NOTE: maybe alias this to PointerGrabStartData because there's another GrabStartData in
    // |     input::keyboard
    input::{
        Seat, SeatHandler,
        pointer::{
            AxisFrame, ButtonEvent, CursorIcon, CursorImageStatus, Focus, GestureHoldBeginEvent,
            GestureHoldEndEvent, GesturePinchBeginEvent, GesturePinchEndEvent,
            GesturePinchUpdateEvent, GestureSwipeBeginEvent, GestureSwipeEndEvent,
            GestureSwipeUpdateEvent, GrabStartData, MotionEvent, PointerGrab, PointerInnerHandle,
            RelativeMotionEvent,
        },
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{IsAlive, Logical, Point, Rectangle, Serial},
};
use tracing::{debug, warn};

use crate::{
    state::{State, WithState},
    window::{WindowElement, window_state::LayoutModeKind},
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

        state.pinnacle.raise_window(self.window.clone());

        let mut layout_mode = self.window.with_state(|state| state.layout_mode.current());

        let output_under_pointer = state
            .pinnacle
            .pointer_contents
            .output_under
            .as_ref()
            .and_then(|op| op.upgrade());

        let win_output = self.window.output(&state.pinnacle);

        if matches!(layout_mode, LayoutModeKind::Spilled) && win_output != output_under_pointer {
            layout_mode = LayoutModeKind::Tiled;
        }

        match layout_mode {
            LayoutModeKind::Tiled => {
                let tag_output = self.window.output(&state.pinnacle);
                if let Some(output_under_pointer) = output_under_pointer
                    && Some(&output_under_pointer) != tag_output.as_ref()
                {
                    self.window.set_tags_to_output(&output_under_pointer);

                    if self.window.with_state(|state| state.layout_mode.is_tiled()) {
                        self.window
                            .with_state_mut(|state| state.set_floating_loc(None));
                    }

                    state.pinnacle.request_layout(&output_under_pointer);

                    if let Some(tag_output) = tag_output {
                        state.pinnacle.request_layout(&tag_output);
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

                    if window_under.with_state(|state| !state.layout_mode.is_tiled()) {
                        return;
                    }

                    let output = self.window.output(&state.pinnacle);

                    debug!("Swapping window positions");
                    state
                        .pinnacle
                        .swap_window_positions(&self.window, &window_under);

                    state.pinnacle.layout_state.pending_swap = true;

                    if let Some(output) = output.as_ref() {
                        state.pinnacle.request_layout(output);
                    }
                }
            }
            LayoutModeKind::Floating | LayoutModeKind::Spilled => {
                let delta = event.location - self.start_data.location;
                let new_loc = self.initial_window_loc.to_f64() + delta;

                state
                    .pinnacle
                    .map_window_to(&self.window, new_loc.to_i32_round());

                self.window.with_state_mut(|state| {
                    state.set_floating_loc(new_loc.to_i32_round());
                });

                if let Some(surface) = self.window.x11_surface()
                    && !surface.is_override_redirect()
                {
                    let geo = surface.geometry();
                    let new_geo = Rectangle::new(new_loc.to_i32_round(), geo.size);
                    surface
                        .configure(new_geo)
                        .expect("failed to configure x11 win");
                }

                let outputs = state.pinnacle.space.outputs_for_element(&self.window);
                for output in outputs {
                    state.schedule_render(&output);
                }
            }
            LayoutModeKind::Maximized | LayoutModeKind::Fullscreen => {
                let tag_output = self.window.output(&state.pinnacle);
                if let Some(output_under_pointer) = output_under_pointer
                    && Some(&output_under_pointer) != tag_output.as_ref()
                {
                    state.move_window_to_output(&self.window, output_under_pointer.clone());
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

    fn unset(&mut self, state: &mut State) {
        // FIXME: granular
        for output in state.pinnacle.space.outputs().cloned().collect::<Vec<_>>() {
            state.schedule_render(&output);
        }
    }

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
            let Some(window) = self.pinnacle.window_for_surface(surface).cloned() else {
                warn!("Surface had no window, cancelling move request");
                return;
            };

            let Some(initial_window_loc) = self
                .pinnacle
                .space
                .element_location(&window)
                .map(|loc| loc.to_f64())
            else {
                warn!("Window was not mapped, cancelling move request");
                return;
            };

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
        let Some(window) = self.pinnacle.window_for_surface(surface).cloned() else {
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
