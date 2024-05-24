// SPDX-License-Identifier: GPL-3.0-or-later

use smithay::{desktop::space::SpaceElement, output::Output, utils::SERIAL_COUNTER};
use tracing::warn;

use crate::{
    state::{Pinnacle, State, WithState},
    window::WindowElement,
};

pub mod keyboard;
pub mod pointer;

impl State {
    /// Update the keyboard focus.
    pub fn update_keyboard_focus(&mut self, output: &Output) {
        let current_focus = self.pinnacle.focused_window(output);

        if let Some(win) = &current_focus {
            assert!(!win.is_x11_override_redirect());

            let wins = output.with_state(|state| state.focus_stack.stack.clone());

            for win in wins.iter() {
                win.set_activate(false);
                if let Some(toplevel) = win.toplevel() {
                    toplevel.send_configure();
                }
            }

            win.set_activate(true);
            if let Some(toplevel) = win.toplevel() {
                toplevel.send_configure();
            }
        }

        self.pinnacle
            .seat
            .get_keyboard()
            .expect("no keyboard")
            .set_focus(
                self,
                current_focus.map(|win| win.into()),
                SERIAL_COUNTER.next_serial(),
            );
    }
}

impl Pinnacle {
    /// Get the currently focused window on `output`.
    ///
    /// This returns the topmost window on the keyboard focus stack that is on an active tag.
    pub fn focused_window(&self, output: &Output) -> Option<WindowElement> {
        // TODO: see if the below is necessary
        // output.with_state(|state| state.focus_stack.stack.retain(|win| win.alive()));

        output
            .with_state(|state| {
                state.focus_stack.focused.then(|| {
                    state
                        .focus_stack
                        .stack
                        .iter()
                        .rev()
                        .filter(|win| win.is_on_active_tag())
                        .find(|win| !win.is_x11_override_redirect())
                        .cloned()
                })
            })
            .flatten()
    }

    pub fn fixup_z_layering(&mut self) {
        for win in self.z_index_stack.iter() {
            self.space.raise_element(win, false);
        }
    }

    /// Raise a window to the top of the z-index stack.
    ///
    /// This does nothing if the window is unmapped.
    pub fn raise_window(&mut self, window: WindowElement, activate: bool) {
        if self.space.elements().all(|win| win != window) {
            warn!("Tried to raise an unmapped window");
            return;
        }

        self.space.raise_element(&window, activate);

        self.z_index_stack.retain(|win| win != window);
        self.z_index_stack.push(window);

        self.fixup_xwayland_window_layering();
    }

    /// Get the currently focused output, or the first mapped output if there is none, or None.
    pub fn focused_output(&self) -> Option<&Output> {
        self.output_focus_stack
            .stack
            .last()
            .or_else(|| self.space.outputs().next())
    }
}

#[derive(Debug, Clone, Default)]
pub struct OutputFocusStack {
    stack: Vec<Output>,
}

impl OutputFocusStack {
    // Set the new focused output.
    pub fn set_focus(&mut self, output: Output) {
        self.stack.retain(|op| op != &output);
        self.stack.push(output);
    }
}

/// A stack of windows, with the top one being the one in focus.
#[derive(Debug, Default)]
pub struct WindowKeyboardFocusStack {
    pub stack: Vec<WindowElement>,
    focused: bool,
}

impl WindowKeyboardFocusStack {
    /// Set `window` to be focused.
    ///
    /// If it's already in the stack, it will be removed then pushed.
    /// If it isn't, it will just be pushed.
    pub fn set_focus(&mut self, window: WindowElement) {
        self.stack.retain(|win| win != window);
        self.stack.push(window);
        self.focused = true;
    }

    /// Unset the focus by marking this stack as unfocused.
    ///
    /// This will cause [`Self::current_focus`] to return `None`.
    pub fn unset_focus(&mut self) {
        self.focused = false;
    }
}
