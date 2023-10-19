// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    cell::RefCell,
    hash::Hash,
    rc::Rc,
    sync::atomic::{AtomicU32, Ordering},
};

use smithay::output::Output;

use crate::{
    layout::Layout,
    state::{State, WithState},
};

static TAG_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum TagId {
    None,
    #[serde(untagged)]
    Some(u32),
}

impl TagId {
    fn next() -> Self {
        Self::Some(TAG_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    pub fn tag(&self, state: &State) -> Option<Tag> {
        state
            .space
            .outputs()
            .flat_map(|op| op.with_state(|state| state.tags.clone()))
            .find(|tag| &tag.id() == self)
    }

    /// Reset the global TagId counter.
    ///
    /// This is used, for example, when a config is reloaded and you want to keep
    /// windows on the same tags.
    pub fn reset() {
        TAG_ID_COUNTER.store(0, Ordering::SeqCst);
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

// RefCell Safety: These methods should never panic because they are all self-contained or Copy.
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

    pub fn output(&self, state: &State) -> Option<Output> {
        state
            .space
            .outputs()
            .find(|output| output.with_state(|state| state.tags.iter().any(|tg| tg == self)))
            .cloned()
    }
}
