// SPDX-License-Identifier: GPL-3.0-or-later

use smithay::{output::Output, utils::IsAlive};

use crate::window::WindowElement;

#[derive(Default)]
pub struct FocusState {
    focus_stack: Vec<WindowElement>,
    pub focused_output: Option<Output>,
}

impl FocusState {
    pub fn new() -> Self {
        Default::default()
    }

    // TODO: how does this work with unmapped windows?
    /// Get the currently focused window. If there is none, the previous focus is returned.
    pub fn current_focus(&mut self) -> Option<WindowElement> {
        while let Some(window) = self.focus_stack.last() {
            if window.alive() {
                return Some(window.clone());
            }
            self.focus_stack.pop();
        }
        None
    }

    /// Set the currently focused window.
    pub fn set_focus(&mut self, window: WindowElement) {
        self.focus_stack.retain(|win| win != &window);
        self.focus_stack.push(window);
    }
}
