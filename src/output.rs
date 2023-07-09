// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::cell::RefCell;

use smithay::output::Output;

use crate::tag::Tag;

#[derive(Default)]
pub struct OutputState {
    pub tags: Vec<Tag>,
}

impl OutputState {
    pub fn focused_tags(&mut self) -> impl Iterator<Item = &mut Tag> {
        self.tags.iter_mut().filter(|tag| tag.active)
    }
}

impl OutputState {
    pub fn with<F, T>(output: &Output, mut func: F) -> T
    where
        F: FnMut(&mut Self) -> T,
    {
        output
            .user_data()
            .insert_if_missing(RefCell::<Self>::default);

        let mut state = output
            .user_data()
            .get::<RefCell<Self>>()
            .expect("RefCell not in data map");

        func(&mut state.borrow_mut())
    }
}
