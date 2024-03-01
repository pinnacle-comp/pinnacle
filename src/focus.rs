// SPDX-License-Identifier: GPL-3.0-or-later

use smithay::{output::Output, utils::SERIAL_COUNTER};

use crate::{
    state::{State, WithState},
    window::WindowElement,
};

pub mod keyboard;
pub mod pointer;

impl State {
    /// Get the currently focused window on `output`
    /// that isn't an override redirect window, if any.
    pub fn focused_window(&self, output: &Output) -> Option<WindowElement> {
        // TODO: see if the below is necessary
        // output.with_state(|state| state.focus_stack.stack.retain(|win| win.alive()));

        let windows = output.with_state(|state| {
            state
                .focus_stack
                .stack
                .iter()
                .rev()
                .filter(|win| {
                    let win_tags = win.with_state(|state| state.tags.clone());
                    let output_tags = state.focused_tags().cloned().collect::<Vec<_>>();

                    win_tags
                        .iter()
                        .any(|win_tag| output_tags.iter().any(|op_tag| win_tag == op_tag))
                })
                .cloned()
                .collect::<Vec<_>>()
        });

        windows
            .into_iter()
            .find(|win| !win.is_x11_override_redirect())
    }

    /// Update the keyboard focus.
    pub fn update_focus(&mut self, output: &Output) {
        let current_focus = self.focused_window(output);

        if let Some(win) = &current_focus {
            assert!(!win.is_x11_override_redirect());

            if let Some(toplevel) = win.toplevel() {
                toplevel.send_configure();
            }
        }

        self.seat.get_keyboard().expect("no keyboard").set_focus(
            self,
            current_focus.map(|win| win.into()),
            SERIAL_COUNTER.next_serial(),
        );
    }

    pub fn fixup_focus(&mut self) {
        for win in self.z_index_stack.stack.iter() {
            self.space.raise_element(win, false);
        }
    }
}

/// A vector of windows, with the last one being the one in focus and the first
/// being the one at the bottom of the focus stack.
#[derive(Debug)]
pub struct FocusStack<T> {
    pub stack: Vec<T>,
    focused: bool,
}

impl<T> Default for FocusStack<T> {
    fn default() -> Self {
        Self {
            stack: Default::default(),
            focused: Default::default(),
        }
    }
}

impl<T: PartialEq> FocusStack<T> {
    /// Set `focus` to be focused.
    ///
    /// If it's already in the stack, it will be removed then pushed.
    /// If it isn't, it will just be pushed.
    pub fn set_focus(&mut self, focus: T) {
        self.stack.retain(|foc| foc != &focus);
        self.stack.push(focus);
        self.focused = true;
    }

    pub fn unset_focus(&mut self) {
        self.focused = false;
    }

    pub fn current_focus(&self) -> Option<&T> {
        self.focused.then(|| self.stack.last())?
    }
}
