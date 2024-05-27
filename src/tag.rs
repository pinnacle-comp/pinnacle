// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    cell::RefCell,
    hash::Hash,
    rc::Rc,
    sync::atomic::{AtomicU32, Ordering},
};

use smithay::output::Output;

use crate::state::{Pinnacle, WithState};

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
    pub fn tag(&self, pinnacle: &Pinnacle) -> Option<Tag> {
        pinnacle
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
    /// The name of this tag.
    name: String,
    /// Whether this tag is active or not.
    active: bool,
}

/// A marker for windows.
///
/// A window may have 0 or more tags, and you can display 0 or more tags
/// on each output at a time.
#[derive(Debug, Clone)]
pub struct Tag {
    /// The internal id of this tag.
    id: TagId,
    inner: Rc<RefCell<TagInner>>,
}

impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Tag {}

impl Hash for Tag {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

// RefCell Safety: These methods should never panic because they are all self-contained or Copy.
impl Tag {
    pub fn id(&self) -> TagId {
        self.id
    }

    pub fn name(&self) -> String {
        self.inner.borrow().name.clone()
    }

    pub fn active(&self) -> bool {
        self.inner.borrow().active
    }

    pub fn set_active(&self, active: bool, pinnacle: &mut Pinnacle) {
        self.inner.borrow_mut().active = active;

        pinnacle.signal_state.tag_active.signal(|buf| {
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
        Self {
            id: TagId::next(),
            inner: Rc::new(RefCell::new(TagInner {
                name,
                active: false,
            })),
        }
    }

    /// Get the output this tag is on.
    ///
    /// RefCell Safety: This uses RefCells on every mapped output.
    pub fn output(&self, pinnacle: &Pinnacle) -> Option<Output> {
        pinnacle
            .space
            .outputs()
            .find(|output| output.with_state(|state| state.tags.iter().any(|tg| tg == self)))
            .cloned()
    }
}
