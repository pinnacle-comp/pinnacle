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
    state::State,
    window::window_state::{Float, WindowState},
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

        let tiled = WindowState::with_state(&self.window, |state| {
            matches!(state.floating, Float::Tiled(_))
        });

        if tiled {
            let window_under = data
                .space
                .elements()
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

                let window_under_floating = WindowState::with_state(&window_under, |state| {
                    matches!(state.floating, Float::Floating)
                });

                if window_under_floating {
                    return;
                }

                tracing::info!("{:?}, {:?}", self.window.geometry(), self.window.bbox());
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
