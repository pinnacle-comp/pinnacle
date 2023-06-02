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
    utils::{IsAlive, Logical, Point},
};

use crate::{backend::Backend, State};

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

        let delta = event.location - self.start_data.location;
        let new_loc = self.initial_window_loc.to_f64() + delta;
        data.space
            .map_element(self.window.clone(), new_loc.to_i32_round(), true);
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
