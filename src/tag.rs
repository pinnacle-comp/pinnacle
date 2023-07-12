// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::{
    cell::RefCell,
    hash::Hash,
    rc::Rc,
    sync::atomic::{AtomicU32, Ordering},
};

use smithay::output::Output;

use crate::{
    backend::Backend,
    layout::Layout,
    state::{State, WithState},
};

static TAG_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct TagId(u32);

impl TagId {
    fn next() -> Self {
        Self(TAG_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug)]
struct TagInner {
    /// The internal id of this tag.
    id: TagId,
    /// The name of this tag.
    name: String,
    /// Whether this tag is active or not.
    active: bool,
    /// What layout this tag has.
    layout: Layout,
}

impl PartialEq for TagInner {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for TagInner {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag(Rc<RefCell<TagInner>>);

impl Tag {
    pub fn id(&self) -> TagId {
        self.0.borrow().id
    }

    pub fn name(&self) -> String {
        self.0.borrow().name.clone()
    }

    pub fn active(&self) -> bool {
        self.0.borrow().active
    }

    pub fn set_active(&mut self, active: bool) {
        self.0.borrow_mut().active = active;
    }

    pub fn layout(&self) -> Layout {
        self.0.borrow().layout
    }
}

impl Tag {
    pub fn new(name: String) -> Self {
        Self(Rc::new(RefCell::new(TagInner {
            id: TagId::next(),
            name,
            active: false,
            layout: Layout::Dwindle, // TODO: get from config
        })))
    }
}

impl<B: Backend> State<B> {
    pub fn output_for_tag(&self, tag: &Tag) -> Option<Output> {
        self.space
            .outputs()
            .find(|output| output.with_state(|state| state.tags.iter().any(|tg| tg == tag)))
            .cloned()
    }
}
