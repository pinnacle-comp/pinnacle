// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    cell::RefCell,
    hash::Hash,
    rc::Rc,
    sync::atomic::{AtomicU32, Ordering},
};

use smithay::{
    backend::renderer::{
        element::{surface::WaylandSurfaceRenderElement, AsRenderElements},
        ImportAll, ImportMem, Renderer,
    },
    desktop::{space::SpaceElement, Space},
    output::Output,
    utils::Scale,
};

use crate::{
    layout::Layout,
    state::{State, WithState},
    window::WindowElement,
};

static TAG_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct TagId(u32);

impl TagId {
    fn next() -> Self {
        Self(TAG_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
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

    /// Get the render_elements for the provided tags.
    pub fn tag_render_elements<R, C>(
        tags: &[Tag],
        windows: &[WindowElement],
        space: &Space<WindowElement>,
        renderer: &mut R,
    ) -> Vec<C>
    where
        R: Renderer + ImportAll + ImportMem,
        <R as Renderer>::TextureId: 'static,
        C: From<WaylandSurfaceRenderElement<R>>,
    {
        let elements = windows
            .iter()
            .filter(|win| {
                win.with_state(|state| {
                    state
                        .tags
                        .iter()
                        .any(|tag| tags.iter().any(|tag2| tag == tag2))
                })
            })
            .flat_map(|win| {
                // subtract win.geometry().loc to align decorations correctly
                let loc = (space.element_location(win).unwrap_or((0, 0).into())
                    - win.geometry().loc)
                    .to_physical(1);
                win.render_elements::<C>(renderer, loc, Scale::from(1.0), 1.0)
            })
            .collect::<Vec<_>>();

        elements
    }
}
