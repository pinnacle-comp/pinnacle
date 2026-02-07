// SPDX-License-Identifier: GPL-3.0-or-later

use smithay::{
    // NOTE: maybe alias this to PointerGrabStartData because there's another GrabStartData in
    // |     input::keyboard
    backend::input::TouchSlot,
    input::{
        Seat, SeatHandler,
        pointer::{
            self, AxisFrame, ButtonEvent, CursorIcon, CursorImageStatus, Focus,
            GestureHoldBeginEvent, GestureHoldEndEvent, GesturePinchBeginEvent,
            GesturePinchEndEvent, GesturePinchUpdateEvent, GestureSwipeBeginEvent,
            GestureSwipeEndEvent, GestureSwipeUpdateEvent, MotionEvent, PointerGrab,
            PointerInnerHandle, RelativeMotionEvent,
        },
        touch::{DownEvent, TouchGrab, TouchInnerHandle, UpEvent},
    },
    output::Output,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{IsAlive, Logical, Point, Rectangle, Serial},
};
use tracing::{debug, warn};

use crate::{
    grab::InputGrabStartData,
    state::{State, WithState},
    window::{WindowElement, window_state::LayoutModeKind},
};

/// Data for moving a window.
pub struct MoveSurfaceGrab {
    pub start_data: InputGrabStartData<State>,
    /// The window being moved
    pub window: WindowElement,
    pub initial_window_loc: Point<f64, Logical>,
}

impl MoveSurfaceGrab {
    fn on_motion(
        &mut self,
        state: &mut State,
        location: Point<f64, Logical>,
        output_under: Option<Output>,
    ) {
        state.pinnacle.raise_window(self.window.clone());
        let mut layout_mode = self.window.with_state(|state| state.layout_mode.current());

        let win_output = self.window.output(&state.pinnacle);

        if matches!(layout_mode, LayoutModeKind::Spilled) && win_output != output_under {
            layout_mode = LayoutModeKind::Tiled;
        }

        match layout_mode {
            LayoutModeKind::Tiled => {
                let tag_output = self.window.output(&state.pinnacle);
                if let Some(output_under_pointer) = output_under
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
                            rect.contains(location.to_i32_round())
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
                let delta = location - self.start_data.location();
                let new_loc = self.initial_window_loc.to_f64() + delta;

                state
                    .pinnacle
                    .map_window_to(&self.window, new_loc.to_i32_round());

                self.window.with_state_mut(|state| {
                    state.set_floating_loc(new_loc.to_i32_round());
                });
            }
            LayoutModeKind::Maximized | LayoutModeKind::Fullscreen => {
                let tag_output = self.window.output(&state.pinnacle);
                if let Some(output_under_pointer) = output_under
                    && Some(&output_under_pointer) != tag_output.as_ref()
                {
                    state
                        .pinnacle
                        .move_window_to_output(&self.window, output_under_pointer.clone());

                    state.pinnacle.update_window_geometry(&self.window, false);
                }
            }
        }
    }

    fn on_unset(&mut self, state: &mut State) {
        // FIXME: granular
        for output in state.pinnacle.space.outputs().cloned().collect::<Vec<_>>() {
            state.schedule_render(&output);
        }
    }
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

        let output_under_pointer = state
            .pinnacle
            .pointer_contents
            .output_under
            .as_ref()
            .and_then(|op| op.upgrade());

        self.on_motion(state, event.location, output_under_pointer);
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

        if !handle
            .current_pressed()
            .contains(&PointerGrab::start_data(self).button)
        {
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

    fn start_data(&self) -> &pointer::GrabStartData<State> {
        self.start_data
            .as_pointer()
            .expect("start_data is not Pointer")
    }

    fn unset(&mut self, state: &mut State) {
        self.on_unset(state);
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

impl TouchGrab<State> for MoveSurfaceGrab {
    fn start_data(&self) -> &smithay::input::touch::GrabStartData<State> {
        self.start_data.as_touch().expect("start_data is not Touch")
    }

    fn down(
        &mut self,
        data: &mut State,
        handle: &mut TouchInnerHandle<'_, State>,
        _focus: Option<(<State as SeatHandler>::TouchFocus, Point<f64, Logical>)>,
        event: &DownEvent,
        seq: Serial,
    ) {
        handle.down(data, None, event, seq);
    }

    fn up(
        &mut self,
        data: &mut State,
        handle: &mut TouchInnerHandle<'_, State>,
        event: &UpEvent,
        seq: Serial,
    ) {
        handle.up(data, event, seq);

        let Some(start_data) = self.start_data.as_touch() else {
            return;
        };

        if event.slot == start_data.slot {
            handle.unset_grab(self, data);
        }
    }

    fn motion(
        &mut self,
        data: &mut State,
        handle: &mut TouchInnerHandle<'_, State>,
        focus: Option<(<State as SeatHandler>::TouchFocus, Point<f64, Logical>)>,
        event: &smithay::input::touch::MotionEvent,
        seq: Serial,
    ) {
        handle.motion(data, focus, event, seq);

        let Some(start_data) = self.start_data.as_touch() else {
            return;
        };

        if event.slot != start_data.slot {
            return;
        }

        if !self.window.alive() {
            handle.unset_grab(self, data);
        }

        let output_under = data
            .pinnacle
            .space
            .output_under(event.location)
            .next()
            .cloned();

        self.on_motion(data, event.location, output_under);
    }

    fn shape(
        &mut self,
        data: &mut State,
        handle: &mut TouchInnerHandle<'_, State>,
        event: &smithay::input::touch::ShapeEvent,
        seq: Serial,
    ) {
        handle.shape(data, event, seq);
    }

    fn orientation(
        &mut self,
        data: &mut State,
        handle: &mut TouchInnerHandle<'_, State>,
        event: &smithay::input::touch::OrientationEvent,
        seq: Serial,
    ) {
        handle.orientation(data, event, seq);
    }

    fn frame(&mut self, data: &mut State, handle: &mut TouchInnerHandle<'_, State>, seq: Serial) {
        handle.frame(data, seq);
    }

    fn cancel(&mut self, data: &mut State, handle: &mut TouchInnerHandle<'_, State>, seq: Serial) {
        handle.cancel(data, seq);
        handle.unset_grab(self, data);
    }

    fn unset(&mut self, data: &mut State) {
        self.on_unset(data);
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
                start_data: start_data.into(),
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
        }
        .into();

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

    pub fn touch_move_request_server(
        &mut self,
        surface: &WlSurface,
        seat: &Seat<State>,
        serial: Serial,
        slot: TouchSlot,
        location: Point<f64, Logical>,
    ) {
        let Some(touch) = seat.get_touch() else {
            tracing::warn!("seat had no touch");
            return;
        };

        let Some(window) = self.pinnacle.window_for_surface(surface).cloned() else {
            warn!("Surface had no window, cancelling move request");
            return;
        };

        let initial_window_loc = self
            .pinnacle
            .space
            .element_location(&window)
            .expect("move request was called on an unmapped window")
            .to_f64();

        let start_data = smithay::input::touch::GrabStartData {
            focus: None,
            slot,
            location,
        };

        let grab = MoveSurfaceGrab {
            start_data: start_data.into(),
            window,
            initial_window_loc,
        };

        touch.set_grab(self, grab, serial);
    }
}
