use smithay::{
    desktop::Window,
    utils::{IsAlive, Serial},
};

use crate::{backend::Backend, state::State};

#[derive(Default)]
pub struct FocusState {
    focus_stack: Vec<Window>,
}

impl FocusState {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn current_focus(&mut self) -> Option<Window> {
        while let Some(window) = self.focus_stack.last() {
            if window.alive() {
                return Some(window.clone());
            }
            self.focus_stack.pop();
        }
        None
    }

    pub fn set_focus(&mut self, window: Window) {
        self.focus_stack.retain(|win| win != &window);
        self.focus_stack.push(window);
    }
}

impl<B: Backend> State<B> {
    pub fn set_focus(&mut self, window: Window, serial: Serial) {
        // INFO: this is inserted into the loop because foot didn't like it when you set the focus
        // |`    immediately after creating the toplevel
        // TODO: figure out why
        self.loop_handle.insert_idle(move |data| {
            data.state.focus_state.set_focus(window.clone());
            data.state.seat.get_keyboard().unwrap().set_focus(
                &mut data.state,
                Some(window.toplevel().wl_surface().clone()),
                serial,
            );
        });
    }
}
