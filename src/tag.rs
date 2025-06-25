// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    hash::Hash,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    },
};

use smithay::output::Output;

use crate::state::{Pinnacle, WithState};

static TAG_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

/// A unique id for a [`Tag`].
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct TagId(u32);

impl TagId {
    /// Creates a new tag ID.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the next available `TagId`.
    fn next() -> Self {
        Self(TAG_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the tag associated with this id.
    pub fn tag(&self, pinnacle: &Pinnacle) -> Option<Tag> {
        let _span = tracy_client::span!("TagId::tag");

        pinnacle.outputs.iter().find_map(|op| {
            op.with_state(|state| {
                state
                    .tags
                    .iter()
                    // FIXME: a better tag tracking system
                    .filter(|tag| !tag.defunct())
                    .find(|tag| &tag.id() == self)
                    .cloned()
            })
        })
    }

    /// Reset the global TagId counter.
    ///
    /// This is used, for example, when a config is reloaded and you want to keep
    /// windows on the same tags.
    pub fn reset() {
        TAG_ID_COUNTER.store(0, Ordering::SeqCst);
    }

    /// Gets the inner numeric ID.
    pub fn to_inner(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone)]
struct TagInner {
    /// The internal id of this tag.
    id: TagId,
    /// The name of this tag.
    name: String,
    /// Whether this tag is active or not.
    active: bool,
    /// This tag is defunct as a result of a config reload
    /// and will be replaced by the next added tag.
    defunct: bool,
}

/// A marker for windows.
///
/// A window may have 0 or more tags, and you can display 0 or more tags
/// on each output at a time.
#[derive(Debug, Clone)]
pub struct Tag {
    inner: Arc<Mutex<TagInner>>,
}

impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for Tag {}

impl Hash for Tag {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let ptr = Arc::as_ptr(&self.inner);
        ptr.hash(state);
    }
}

// RefCell Safety: These methods should never panic because they are all self-contained or Copy.
impl Tag {
    pub fn new(name: String) -> Self {
        Self {
            inner: Arc::new(Mutex::new(TagInner {
                id: TagId::next(),
                name: name.clone(),
                active: false,
                defunct: false,
            })),
        }
    }

    /// Get the output this tag is on.
    ///
    /// RefCell Safety: This uses RefCells on every mapped output.
    pub fn output(&self, pinnacle: &Pinnacle) -> Option<Output> {
        let _span = tracy_client::span!("Tag::output");

        pinnacle
            .outputs
            .iter()
            .find(|output| output.with_state(|state| state.tags.contains(self)))
            .cloned()
    }

    /// Replace all inner fields of this tag with ones from the `new_tag`.
    pub fn replace(&self, new_tag: Tag) {
        let mut tag = self.inner.lock().unwrap();
        *tag = new_tag.inner.lock().unwrap().clone();
        tag.defunct = false;
    }

    /// Gets this tag's unique numeric ID.
    pub fn id(&self) -> TagId {
        self.inner.lock().unwrap().id
    }

    /// Gets this tag's name.
    pub fn name(&self) -> String {
        self.inner.lock().unwrap().name.clone()
    }

    /// Gets whether this tag is active.
    pub fn active(&self) -> bool {
        self.inner.lock().unwrap().active
    }

    /// Sets this tag's active state.
    ///
    /// Returns whether the new state is different from the old one,
    pub fn set_active(&self, active: bool) -> bool {
        std::mem::replace(&mut self.inner.lock().unwrap().active, active) != active
    }

    /// Gets whether this tag is defunct as a result of a config reload.
    pub fn defunct(&self) -> bool {
        self.inner.lock().unwrap().defunct
    }

    /// Make this tag defunct.
    pub fn make_defunct(&self) {
        self.inner.lock().unwrap().defunct = true;
    }
}
