// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    cell::{Cell, RefCell},
    hash::Hash,
    rc::Rc,
    sync::atomic::{AtomicU32, Ordering},
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
        pinnacle.outputs.keys().find_map(|op| {
            op.with_state(|state| state.tags.iter().find(|tag| &tag.id() == self).cloned())
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

#[derive(Debug)]
struct TagInner {
    /// The internal id of this tag.
    id: Cell<TagId>,
    /// The name of this tag.
    name: RefCell<String>,
    /// Whether this tag is active or not.
    active: Cell<bool>,
    /// This tag is defunct as a result of a config reload
    /// and will be replaced by the next added tag.
    defunct: Cell<bool>,
}

/// A marker for windows.
///
/// A window may have 0 or more tags, and you can display 0 or more tags
/// on each output at a time.
#[derive(Debug, Clone)]
pub struct Tag {
    inner: Rc<TagInner>,
}

impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for Tag {}

impl Hash for Tag {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let ptr = Rc::as_ptr(&self.inner);
        ptr.hash(state);
    }
}

// RefCell Safety: These methods should never panic because they are all self-contained or Copy.
impl Tag {
    pub fn new(name: String) -> Self {
        Self {
            inner: Rc::new(TagInner {
                id: Cell::new(TagId::next()),
                name: RefCell::new(name),
                active: Cell::new(false),
                defunct: Cell::new(false),
            }),
        }
    }

    /// Get the output this tag is on.
    ///
    /// RefCell Safety: This uses RefCells on every mapped output.
    pub fn output(&self, pinnacle: &Pinnacle) -> Option<Output> {
        pinnacle
            .outputs
            .keys()
            .find(|output| output.with_state(|state| state.tags.contains(self)))
            .cloned()
    }

    /// Replace all inner fields of this tag with ones from the `new_tag`.
    pub fn replace(&self, new_tag: Tag) {
        self.inner.id.set(new_tag.inner.id.get());
        self.inner
            .name
            .borrow_mut()
            .clone_from(&new_tag.inner.name.borrow());
        self.inner.active.set(new_tag.inner.active.get());
        self.inner.defunct.set(false);
    }

    /// Gets this tag's unique numeric ID.
    pub fn id(&self) -> TagId {
        self.inner.id.get()
    }

    /// Gets this tag's name.
    pub fn name(&self) -> String {
        self.inner.name.borrow().clone()
    }

    /// Gets whether this tag is active.
    pub fn active(&self) -> bool {
        self.inner.active.get()
    }

    /// Sets this tag's active state.
    ///
    /// Returns whether the new state is different from the old one,
    pub fn set_active(&self, active: bool) -> bool {
        self.inner.active.replace(active) != active
    }

    /// Gets whether this tag is defunct as a result of a config reload.
    pub fn defunct(&self) -> bool {
        self.inner.defunct.get()
    }

    /// Make this tag defunct.
    pub fn make_defunct(&self) {
        self.inner.defunct.set(true);
    }
}
