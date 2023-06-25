// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use smithay::{desktop::Window, output::Output, utils::IsAlive};

#[derive(Default)]
pub struct FocusState {
    focus_stack: Vec<Window>,
    pub focused_output: Option<Output>,
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
