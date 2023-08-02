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
    backend::Backend,
    state::{State, WithState},
    window::{
        window_state::{Float, WindowResizeState},
        WindowElement,
    },
};

pub struct MoveSurfaceGrab<S: SeatHandler> {
    pub start_data: GrabStartData<S>,
    pub window: WindowElement,
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
        if let WindowElement::X11(surface) = &self.window {
            data.xwm
                .as_mut()
                .expect("no xwm")
                .raise_window(surface)
                .expect("failed to raise x11 win");
        }

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
            let new_loc = (self.initial_window_loc.to_f64() + delta).to_i32_round();
            data.space.map_element(self.window.clone(), new_loc, true);
            self.window.with_state(|state| {
                if state.floating.is_floating() {
                    state.floating = Float::Floating(new_loc);
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

pub fn move_request_client<B: Backend>(
    state: &mut State<B>,
    surface: &WlSurface,
    seat: &Seat<State<B>>,
    serial: smithay::utils::Serial,
) {
    let pointer = seat.get_pointer().expect("seat had no pointer");
    if let Some(start_data) = crate::pointer::pointer_grab_start_data(&pointer, surface, serial) {
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
        };

        pointer.set_grab(state, grab, serial, Focus::Clear);
    } else {
        tracing::warn!("no grab start data");
    }
}

pub fn move_request_server<B: Backend>(
    state: &mut State<B>,
    surface: &WlSurface,
    seat: &Seat<State<B>>,
    serial: smithay::utils::Serial,
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

    const BUTTON_LEFT: u32 = 0x110;

    let start_data = smithay::input::pointer::GrabStartData {
        focus: pointer
            .current_focus()
            .map(|focus| (focus, initial_window_loc)),
        button: BUTTON_LEFT,
        location: pointer.current_location(),
    };

    let grab = MoveSurfaceGrab {
        start_data,
        window,
        initial_window_loc,
    };

    pointer.set_grab(state, grab, serial, Focus::Clear);
}
