// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::cell::RefCell;

use smithay::output::Output;

use crate::tag::TagId;

#[derive(Default)]
pub struct OutputState {
    pub focused_tags: Vec<TagId>,
}

impl OutputState {
    pub fn with<F, T>(output: &Output, mut func: F) -> T
    where
        F: FnMut(&mut Self) -> T,
    {
        output
            .user_data()
            .insert_if_missing(RefCell::<Self>::default);

        let state = output
            .user_data()
            .get::<RefCell<Self>>()
            .expect("RefCell doesn't exist in data map (This should NEVER happen. If you see this, something oofed big-time.)");

        func(&mut state.borrow_mut())
    }
}
