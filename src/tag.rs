// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::{
    hash::Hash,
    sync::atomic::{AtomicU32, Ordering},
};

static TAG_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Hash, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
pub struct TagId(u32);

impl TagId {
    fn next() -> Self {
        Self(TAG_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug)]
pub struct Tag {
    /// The internal id of this tag.
    pub id: TagId,
    /// The name of this tag.
    pub name: String,
    /// Whether this tag is active or not.
    pub active: bool,
    // TODO: layout
}

impl Tag {
    pub fn new(name: String) -> Self {
        Self {
            id: TagId::next(),
            name,
            active: false,
        }
    }
}
