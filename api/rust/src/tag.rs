//! Tag management.

use std::collections::HashMap;

use crate::{
    msg::{Msg, Request, RequestResponse},
    output::{OutputHandle, OutputName},
    request, send_msg,
};

/// Get a tag by its name and output. If `output` is `None`, the currently focused output will
/// be used instead.
///
/// If multiple tags have the same name, this returns the first one.
pub fn get(name: &str, output: Option<&OutputHandle>) -> Option<TagHandle> {
    get_all()
        .filter(|tag| {
            tag.properties().output.is_some_and(|op| match output {
                Some(output) => &op == output,
                None => Some(op) == crate::output::get_focused(),
            })
        })
        .find(|tag| tag.properties().name.is_some_and(|s| s == name))
}

/// Get all tags.
pub fn get_all() -> impl Iterator<Item = TagHandle> {
    let RequestResponse::Tags { tag_ids } = request(Request::GetTags) else {
        unreachable!()
    };

    tag_ids.into_iter().map(TagHandle)
}

// TODO: return taghandles here
/// Add tags with the names from `names` to `output`.
pub fn add(output: &OutputHandle, names: &[&str]) {
    let msg = Msg::AddTags {
        output_name: output.0.clone(),
        tag_names: names.iter().map(|s| s.to_string()).collect(),
    };

    send_msg(msg).unwrap();
}

/// Create a `LayoutCycler` to cycle layouts on tags.
///
/// Given a slice of layouts, this will create a `LayoutCycler` with two methods;
/// one will cycle forward the layout for the active tag, and one will cycle backward.
///
/// # Example
/// ```
/// todo!()
/// ```
pub fn layout_cycler(layouts: &[Layout]) -> LayoutCycler {
    let indices = std::rc::Rc::new(std::cell::RefCell::new(HashMap::<TagId, usize>::new()));
    let indices_clone = indices.clone();
    let layouts = layouts.to_vec();
    let layouts_clone = layouts.clone();
    let len = layouts.len();
    let next = move |output: Option<&OutputHandle>| {
        let Some(output) = output.cloned().or_else(crate::output::get_focused) else {
            return;
        };

        let Some(tag) = output
            .properties()
            .tags
            .into_iter()
            .find(|tag| tag.properties().active == Some(true))
        else {
            return;
        };

        let mut indices = indices.borrow_mut();
        let index = indices.entry(tag.0).or_insert(0);

        if *index + 1 >= len {
            *index = 0;
        } else {
            *index += 1;
        }

        tag.set_layout(layouts[*index]);
    };
    let prev = move |output: Option<&OutputHandle>| {
        let Some(output) = output.cloned().or_else(crate::output::get_focused) else {
            return;
        };

        let Some(tag) = output
            .properties()
            .tags
            .into_iter()
            .find(|tag| tag.properties().active == Some(true))
        else {
            return;
        };

        let mut indices = indices_clone.borrow_mut();
        let index = indices.entry(tag.0).or_insert(0);

        if index.wrapping_sub(1) == usize::MAX {
            *index = len - 1;
        } else {
            *index -= 1;
        }

        tag.set_layout(layouts_clone[*index]);
    };

    LayoutCycler {
        next: Box::new(next),
        prev: Box::new(prev),
    }
}

/// A layout cycler that keeps track of tags and their layouts and provides methods to cycle
/// layouts on them.
#[allow(clippy::type_complexity)]
pub struct LayoutCycler {
    /// Cycle to the next layout on the given output, or the focused output if `None`.
    pub next: Box<dyn FnMut(Option<&OutputHandle>)>,
    /// Cycle to the previous layout on the given output, or the focused output if `None`.
    pub prev: Box<dyn FnMut(Option<&OutputHandle>)>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub(crate) enum TagId {
    None,
    #[serde(untagged)]
    Some(u32),
}

/// A handle to a tag.
pub struct TagHandle(pub(crate) TagId);

/// Properties of a tag, retrieved through [`TagHandle::properties`].
#[derive(Debug)]
pub struct TagProperties {
    /// Whether or not the tag is active.
    pub active: Option<bool>,
    /// The tag's name.
    pub name: Option<String>,
    /// The output the tag is on.
    pub output: Option<OutputHandle>,
}

impl TagHandle {
    /// Get this tag's [`TagProperties`].
    pub fn properties(&self) -> TagProperties {
        let RequestResponse::TagProps {
            active,
            name,
            output_name,
        } = request(Request::GetTagProps { tag_id: self.0 })
        else {
            unreachable!()
        };

        TagProperties {
            active,
            name,
            output: output_name.map(|name| OutputHandle(OutputName(name))),
        }
    }

    /// Toggle this tag.
    pub fn toggle(&self) {
        let msg = Msg::ToggleTag { tag_id: self.0 };
        send_msg(msg).unwrap();
    }

    /// Switch to this tag, deactivating all others on its output.
    pub fn switch_to(&self) {
        let msg = Msg::SwitchToTag { tag_id: self.0 };
        send_msg(msg).unwrap();
    }

    /// Set this tag's [`Layout`].
    pub fn set_layout(&self, layout: Layout) {
        let msg = Msg::SetLayout {
            tag_id: self.0,
            layout,
        };

        send_msg(msg).unwrap()
    }
}

/// Layouts for tags.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum Layout {
    /// One master window on the left with all other windows stacked to the right.
    MasterStack,
    /// Windows split in half towards the bottom right corner.
    Dwindle,
    /// Windows split in half in a spiral
    Spiral,
    /// One main corner window in the top left with a column of windows on the right and a row on the bottom.
    CornerTopLeft,
    /// One main corner window in the top right with a column of windows on the left and a row on the bottom.
    CornerTopRight,
    /// One main corner window in the bottom left with a column of windows on the right and a row on the top.
    CornerBottomLeft,
    /// One main corner window in the bottom right with a column of windows on the left and a row on the top.
    CornerBottomRight,
}
