// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use smithay::{desktop::Window, output::Output};

#[derive(Debug, Hash, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
pub struct TagId(String);

#[derive(Debug)]
pub struct Tag {
    pub id: TagId,
    pub windows: Vec<Window>,
    pub output: Output,
    // TODO: layout
}

#[derive(Debug, Default)]
pub struct TagState {
    pub tags: Vec<Tag>,
}

impl TagState {
    pub fn new() -> Self {
        Default::default()
    }
}
