// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use smithay::{
    desktop::Window,
    // NOTE: maybe alias this to PointerGrabStartData because there's another GrabStartData in
    // |     input::keyboard
    input::{
        pointer::PointerGrab,
        pointer::{
            AxisFrame, ButtonEvent, GrabStartData, MotionEvent, PointerInnerHandle,
            RelativeMotionEvent,
        },
        SeatHandler,
    },
    utils::{IsAlive, Logical, Point, Rectangle},
};

use crate::{
    backend::Backend,
    state::{State, WithState},
    window::window_state::WindowResizeState,
};

pub struct MoveSurfaceGrab<S: SeatHandler> {
    pub start_data: GrabStartData<S>,
    pub window: Window,
    pub initial_window_loc: Point<i32, Logical>,
}

impl<B: Backend> PointerGrab<State<B>> for MoveSurfaceGrab<State<B>> {
    fn motion(
        &mut self,
        data: &mut State<B>,
        handle: &mut PointerInnerHandle<'_, State<B>>,
        _focus: Option<(<State<B> as SeatHandler>::PointerFocus, Point<i32, Logical>)>,
        event: &MotionEvent,
    ) {
        handle.motion(data, None, event);

        if !self.window.alive() {
            handle.unset_grab(data, event.serial, event.time);
            return;
        }

        data.space.raise_element(&self.window, false);

        // tracing::info!("window geo is: {:?}", self.window.geometry());
        // tracing::info!("loc is: {:?}", data.space.element_location(&self.window));

        let tiled = self.window.with_state(|state| state.floating.is_tiled());

        if tiled {
            // INFO: this is being used instead of space.element_under(event.location) because that
            // |     uses the bounding box, which is different from the actual geometry
            let window_under = data
                .space
                .elements()
                .rev()
                .find(|&win| {
                    if let Some(loc) = data.space.element_location(win) {
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

                let is_floating = window_under.with_state(|state| state.floating.is_floating());

                if is_floating {
                    return;
                }

                let has_pending_resize = window_under
                    .with_state(|state| !matches!(state.resize_state, WindowResizeState::Idle));

                if has_pending_resize {
                    return;
                }

                data.swap_window_positions(&self.window, &window_under);
            }
        } else {
            let delta = event.location - self.start_data.location;
            let new_loc = self.initial_window_loc.to_f64() + delta;
            data.space
                .map_element(self.window.clone(), new_loc.to_i32_round(), true);
        }
    }

    fn relative_motion(
        &mut self,
        data: &mut State<B>,
        handle: &mut PointerInnerHandle<'_, State<B>>,
        focus: Option<(<State<B> as SeatHandler>::PointerFocus, Point<i32, Logical>)>,
        event: &RelativeMotionEvent,
    ) {
        handle.relative_motion(data, focus, event);
    }

    fn button(
        &mut self,
        data: &mut State<B>,
        handle: &mut PointerInnerHandle<'_, State<B>>,
        event: &ButtonEvent,
    ) {
        handle.button(data, event);

        const BUTTON_LEFT: u32 = 0x110;

        if !handle.current_pressed().contains(&BUTTON_LEFT) {
            handle.unset_grab(data, event.serial, event.time);
        }
    }

    fn axis(
        &mut self,
        data: &mut State<B>,
        handle: &mut PointerInnerHandle<'_, State<B>>,
        details: AxisFrame,
    ) {
        handle.axis(data, details);
    }

    fn start_data(&self) -> &GrabStartData<State<B>> {
        &self.start_data
    }
}
