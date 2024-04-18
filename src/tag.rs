// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    hash::Hash,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    },
};

use smithay::output::Output;

use crate::state::{State, WithState};

static TAG_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

/// A unique id for a [`Tag`].
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct TagId(pub u32);

impl TagId {
    /// Get the next available `TagId`.
    fn next() -> Self {
        Self(TAG_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the tag associated with this id.
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
}

impl PartialEq for TagInner {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for TagInner {}

/// A marker for windows.
///
/// A window may have 0 or more tags, and you can display 0 or more tags
/// on each output at a time.
#[derive(Debug, Clone)]
pub struct Tag(Arc<Mutex<TagInner>>);

impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

// RefCell Safety: These methods should never panic because they are all self-contained or Copy.
impl Tag {
    pub fn id(&self) -> TagId {
        self.0.lock().expect("tag already locked").id
    }

    pub fn name(&self) -> String {
        self.0.lock().expect("tag already locked").name.clone()
    }

    pub fn active(&self) -> bool {
        self.0.lock().expect("tag already locked").active
    }

    pub fn set_active(&self, active: bool, state: &mut State) {
        self.0.lock().expect("tag already locked").active = active;

        state.signal_state.tag_active.signal(|buf| {
            buf.push_back(
                pinnacle_api_defs::pinnacle::signal::v0alpha1::TagActiveResponse {
                    tag_id: Some(self.id().0),
                    active: Some(self.active()),
                },
            );
        })
    }
}

impl Tag {
    pub fn new(name: String) -> Self {
        Self(Arc::new(Mutex::new(TagInner {
            id: TagId::next(),
            name,
            active: false,
        })))
    }

    /// Get the output this tag is on.
    ///
    /// RefCell Safety: This uses RefCells on every mapped output.
    pub fn output(&self, state: &State) -> Option<Output> {
        state
            .space
            .outputs()
            .find(|output| output.with_state(|state| state.tags.iter().any(|tg| tg == self)))
            .cloned()
    }
}
