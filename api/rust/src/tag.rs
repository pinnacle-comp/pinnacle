use std::collections::HashMap;

use crate::{
    msg::{Layout, Msg, OutputName, Request, RequestResponse, TagId},
    output::{Output, OutputHandle},
    request, send_msg,
};

pub struct Tag;

impl Tag {
    /// Get a tag by its name and output. If `output` is `None`, the currently focused output will
    /// be used instead.
    ///
    /// If multiple tags have the same name, this returns the first one.
    pub fn get(&self, name: &str, output: Option<&OutputHandle>) -> Option<TagHandle> {
        self.get_all()
            .filter(|tag| {
                tag.properties()
                    .output
                    .is_some_and(|op| Some(&op) == output)
            })
            .find(|tag| tag.properties().name.is_some_and(|s| s == name))
    }

    /// Get all tags.
    pub fn get_all(&self) -> impl Iterator<Item = TagHandle> {
        let RequestResponse::Tags { tag_ids } = request(Request::GetTags) else {
            unreachable!()
        };

        tag_ids.into_iter().map(TagHandle)
    }

    // TODO: return taghandles here
    /// Add tags with the names from `names` to `output`.
    pub fn add(&self, output: &OutputHandle, names: &[&str]) {
        let msg = Msg::AddTags {
            output_name: output.0.clone(),
            tag_names: names.iter().map(|s| s.to_string()).collect(),
        };

        send_msg(msg).unwrap();
    }

    pub fn layout_cycler(&self, layouts: &[Layout]) -> LayoutCycler {
        let mut indices = HashMap::<TagId, usize>::new();
        let layouts = layouts.to_vec();
        let len = layouts.len();
        let cycle = move |cycle: Cycle, output: Option<&OutputHandle>| {
            let Some(output) = output.cloned().or_else(|| Output.get_focused()) else {
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

            let index = indices.entry(tag.0).or_insert(0);

            match cycle {
                Cycle::Forward => {
                    if *index + 1 >= len {
                        *index = 0;
                    } else {
                        *index += 1;
                    }
                }
                Cycle::Backward => {
                    if index.wrapping_sub(1) == usize::MAX {
                        *index = len - 1;
                    } else {
                        *index -= 1;
                    }
                }
            }

            tag.set_layout(layouts[*index]);
        };

        LayoutCycler {
            cycle: Box::new(cycle),
        }
    }
}

/// Which direction to cycle layouts.
#[derive(Debug, Clone, Copy)]
enum Cycle {
    /// Cycle layouts forward.
    Forward,
    /// Cycle layouts backward.
    Backward,
}

/// A layout cycler that keeps track of tags and their layouts and provides methods to cycle
/// layouts on them.
#[allow(clippy::type_complexity)]
pub struct LayoutCycler {
    cycle: Box<dyn FnMut(Cycle, Option<&OutputHandle>)>,
}

impl LayoutCycler {
    pub fn next(&mut self, output: Option<&OutputHandle>) {
        (self.cycle)(Cycle::Forward, output);
    }

    pub fn prev(&mut self, output: Option<&OutputHandle>) {
        (self.cycle)(Cycle::Backward, output);
    }
}

pub struct TagHandle(pub TagId);

pub struct TagProperties {
    active: Option<bool>,
    name: Option<String>,
    output: Option<OutputHandle>,
}

impl TagHandle {
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

    pub fn toggle(&self) {
        let msg = Msg::ToggleTag { tag_id: self.0 };
        send_msg(msg).unwrap();
    }

    pub fn switch_to(&self) {
        let msg = Msg::SwitchToTag { tag_id: self.0 };
        send_msg(msg).unwrap();
    }

    pub fn set_layout(&self, layout: Layout) {
        let msg = Msg::SetLayout {
            tag_id: self.0,
            layout,
        };

        send_msg(msg).unwrap()
    }
}
