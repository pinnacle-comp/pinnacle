// SPDX-License-Identifier: GPL-3.0-or-later

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

    pub fn tag<B: Backend>(&self, state: &State<B>) -> Option<Tag> {
        state
            .space
            .outputs()
            .flat_map(|op| op.with_state(|state| state.tags.clone()))
            .find(|tag| &tag.id() == self)
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

    pub fn set_active(&self, active: bool) {
        self.0.borrow_mut().active = active;
    }

    pub fn layout(&self) -> Layout {
        self.0.borrow().layout
    }

    pub fn set_layout(&self, layout: Layout) {
        self.0.borrow_mut().layout = layout;
    }
}

impl Tag {
    pub fn new(name: String) -> Self {
        Self(Rc::new(RefCell::new(TagInner {
            id: TagId::next(),
            name,
            active: false,
            layout: Layout::MasterStack, // TODO: get from config
        })))
    }

    pub fn output<B: Backend>(&self, state: &State<B>) -> Option<Output> {
        state
            .space
            .outputs()
            .find(|output| output.with_state(|state| state.tags.iter().any(|tg| tg == self)))
            .cloned()
    }
}
