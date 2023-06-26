// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use smithay::desktop::Window;

use crate::{backend::Backend, state::State};

use super::window_state::WindowState;

#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Tag(String);

impl Tag {
    /// Returns all windows that have this tag.
    pub fn windows<B: Backend>(&self, state: &State<B>) -> Vec<Window> {
        state
            .space
            .elements()
            .filter(|&window| {
                WindowState::with_state(window, |win_state| win_state.tags.contains(self))
            })
            .cloned()
            .collect()
    }
}
