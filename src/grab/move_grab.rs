// SPDX-License-Identifier: GPL-3.0-or-later

use smithay::{
    desktop::space::SpaceElement,
    // NOTE: maybe alias this to PointerGrabStartData because there's another GrabStartData in
    // |     input::keyboard
    input::{
        pointer::{
            AxisFrame, ButtonEvent, GrabStartData, MotionEvent, PointerInnerHandle,
            RelativeMotionEvent,
        },
        pointer::{Focus, PointerGrab},
        Seat, SeatHandler,
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{IsAlive, Logical, Point, Rectangle},
};

use crate::{
    state::{State, WithState},
    window::{
        window_state::{FloatingOrTiled, LocationRequestState},
        WindowElement,
    },
};

pub struct MoveSurfaceGrab<S: SeatHandler> {
    pub start_data: GrabStartData<S>,
    pub window: WindowElement,
    pub initial_window_loc: Point<i32, Logical>,
    pub button_used: u32,
}

impl PointerGrab<State> for MoveSurfaceGrab<State> {
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
            handle.unset_grab(state, event.serial, event.time);
            return;
        }

        state.space.raise_element(&self.window, false);
        if let WindowElement::X11(surface) = &self.window {
            state
                .xwm
                .as_mut()
                .expect("no xwm")
                .raise_window(surface)
                .expect("failed to raise x11 win");
        }

        // tracing::info!("window geo is: {:?}", self.window.geometry());
        // tracing::info!("loc is: {:?}", data.space.element_location(&self.window));

        let tiled = self
            .window
            .with_state(|state| state.floating_or_tiled.is_tiled());

        if tiled {
            // INFO: this is being used instead of space.element_under(event.location) because that
            // |     uses the bounding box, which is different from the actual geometry
            let window_under = state
                .space
                .elements()
                .rev()
                .find(|&win| {
                    if let Some(loc) = state.space.element_location(win) {
                        let size = win.geometry().size;
                        let rect = Rectangle { size, loc };
                        rect.contains(event.location.to_i32_round())
                    } else {
                        false
                    }
                })
                .cloned();

            if let Some(window_under) = window_under {
                if window_under == self.window {
                    return;
                }

                if state
                    .space
                    .element_geometry(&self.window)
                    .is_some_and(|geo| {
                        state
                            .space
                            .element_geometry(&window_under)
                            .is_some_and(|geo2| geo.overlaps(geo2))
                    })
                {
                    return;
                }

                let is_floating =
                    window_under.with_state(|state| state.floating_or_tiled.is_floating());

                if is_floating {
                    return;
                }

                let has_pending_resize = window_under.with_state(|state| {
                    !matches!(state.loc_request_state, LocationRequestState::Idle)
                });

                if has_pending_resize {
                    return;
                }

                tracing::debug!("Swapping window positions");
                state.swap_window_positions(&self.window, &window_under);
            }
        } else {
            let delta = event.location - self.start_data.location;
            let new_loc = (self.initial_window_loc.to_f64() + delta).to_i32_round();
            state.space.map_element(self.window.clone(), new_loc, true);

            let size = state
                .space
                .element_geometry(&self.window)
                .expect("window wasn't mapped")
                .size;

            self.window.with_state(|state| {
                if state.floating_or_tiled.is_floating() {
                    state.floating_or_tiled =
                        FloatingOrTiled::Floating(Rectangle::from_loc_and_size(new_loc, size));
                }
            });

            if let WindowElement::X11(surface) = &self.window {
                let geo = surface.geometry();
                let new_geo = Rectangle::from_loc_and_size(new_loc, geo.size);
                surface
                    .configure(new_geo)
                    .expect("failed to configure x11 win");
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

        if !handle.current_pressed().contains(&self.button_used) {
            handle.unset_grab(data, event.serial, event.time);
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

    fn gesture_swipe_begin(
        &mut self,
        _data: &mut State,
        _handle: &mut PointerInnerHandle<'_, State>,
        _event: &smithay::input::pointer::GestureSwipeBeginEvent,
    ) {
        todo!()
    }

    fn gesture_swipe_update(
        &mut self,
        _data: &mut State,
        _handle: &mut PointerInnerHandle<'_, State>,
        _event: &smithay::input::pointer::GestureSwipeUpdateEvent,
    ) {
        todo!()
    }

    fn gesture_swipe_end(
        &mut self,
        _data: &mut State,
        _handle: &mut PointerInnerHandle<'_, State>,
        _event: &smithay::input::pointer::GestureSwipeEndEvent,
    ) {
        todo!()
    }

    fn gesture_pinch_begin(
        &mut self,
        _data: &mut State,
        _handle: &mut PointerInnerHandle<'_, State>,
        _event: &smithay::input::pointer::GesturePinchBeginEvent,
    ) {
        todo!()
    }

    fn gesture_pinch_update(
        &mut self,
        _data: &mut State,
        _handle: &mut PointerInnerHandle<'_, State>,
        _event: &smithay::input::pointer::GesturePinchUpdateEvent,
    ) {
        todo!()
    }

    fn gesture_pinch_end(
        &mut self,
        _data: &mut State,
        _handle: &mut PointerInnerHandle<'_, State>,
        _event: &smithay::input::pointer::GesturePinchEndEvent,
    ) {
        todo!()
    }

    fn gesture_hold_begin(
        &mut self,
        _data: &mut State,
        _handle: &mut PointerInnerHandle<'_, State>,
        _event: &smithay::input::pointer::GestureHoldBeginEvent,
    ) {
        todo!()
    }

    fn gesture_hold_end(
        &mut self,
        _data: &mut State,
        _handle: &mut PointerInnerHandle<'_, State>,
        _event: &smithay::input::pointer::GestureHoldEndEvent,
    ) {
        todo!()
    }
}

pub fn move_request_client(
    state: &mut State,
    surface: &WlSurface,
    seat: &Seat<State>,
    serial: smithay::utils::Serial,
    button_used: u32,
) {
    let pointer = seat.get_pointer().expect("seat had no pointer");
    if let Some(start_data) = crate::grab::pointer_grab_start_data(&pointer, surface, serial) {
        let Some(window) = state.window_for_surface(surface) else {
            tracing::error!("Surface had no window, cancelling move request");
            return;
        };

        let initial_window_loc = state
            .space
            .element_location(&window)
            .expect("move request was called on an unmapped window");

        let grab = MoveSurfaceGrab {
            start_data,
            window,
            initial_window_loc,
            button_used,
        };

        pointer.set_grab(state, grab, serial, Focus::Clear);
    } else {
        tracing::warn!("no grab start data");
    }
}

pub fn move_request_server(
    state: &mut State,
    surface: &WlSurface,
    seat: &Seat<State>,
    serial: smithay::utils::Serial,
    button_used: u32,
) {
    let pointer = seat.get_pointer().expect("seat had no pointer");
    let Some(window) = state.window_for_surface(surface) else {
        tracing::error!("Surface had no window, cancelling move request");
        return;
    };

    let initial_window_loc = state
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
        button_used,
    };

    pointer.set_grab(state, grab, serial, Focus::Clear);
}
