// SPDX-License-Identifier: GPL-3.0-or-later

use std::num::NonZeroU32;

use crate::{
    output::OutputName,
    state::{State, WithState},
    tag::TagId,
    window::{window_state::FullscreenOrMaximized, WindowElement},
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WindowRuleCondition {
    /// This condition is met when any of the conditions provided is met.
    #[serde(default)]
    cond_any: Option<Vec<WindowRuleCondition>>,
    /// This condition is met when all of the conditions provided are met.
    #[serde(default)]
    cond_all: Option<Vec<WindowRuleCondition>>,
    /// This condition is met when the class matches.
    #[serde(default)]
    class: Option<Vec<String>>,
    /// This condition is met when the title matches.
    #[serde(default)]
    title: Option<Vec<String>>,
    /// This condition is met when the tag matches.
    #[serde(default)]
    tag: Option<Vec<TagId>>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum AllOrAny {
    All,
    Any,
}

impl WindowRuleCondition {
    /// RefCell Safety: This method uses RefCells on `window`.
    pub fn is_met(&self, state: &State, window: &WindowElement) -> bool {
        Self::is_met_inner(self, state, window, AllOrAny::All)
    }

    fn is_met_inner(&self, state: &State, window: &WindowElement, all_or_any: AllOrAny) -> bool {
        tracing::debug!("{self:#?}");

        let WindowRuleCondition {
            cond_any,
            cond_all,
            class,
            title,
            tag,
        } = self;

        match all_or_any {
            AllOrAny::All => {
                let cond_any = if let Some(cond_any) = cond_any {
                    cond_any
                        .iter()
                        .any(|cond| Self::is_met_inner(cond, state, window, AllOrAny::Any))
                } else {
                    true
                };
                let cond_all = if let Some(cond_all) = cond_all {
                    cond_all
                        .iter()
                        .all(|cond| Self::is_met_inner(cond, state, window, AllOrAny::All))
                } else {
                    true
                };
                let classes = if let Some(classes) = class {
                    classes
                        .iter()
                        .all(|class| window.class().as_ref() == Some(class))
                } else {
                    true
                };
                let titles = if let Some(titles) = title {
                    titles
                        .iter()
                        .all(|title| window.title().as_ref() == Some(title))
                } else {
                    true
                };
                let tags = if let Some(tag_ids) = tag {
                    let mut tags = tag_ids.iter().filter_map(|tag_id| tag_id.tag(state));
                    tags.all(|tag| window.with_state(|state| state.tags.contains(&tag)))
                } else {
                    true
                };

                tracing::debug!("{cond_all} {cond_any} {classes} {titles} {tags}");
                cond_all && cond_any && classes && titles && tags
            }
            AllOrAny::Any => {
                let cond_any = if let Some(cond_any) = cond_any {
                    cond_any
                        .iter()
                        .any(|cond| Self::is_met_inner(cond, state, window, AllOrAny::Any))
                } else {
                    false
                };
                let cond_all = if let Some(cond_all) = cond_all {
                    cond_all
                        .iter()
                        .all(|cond| Self::is_met_inner(cond, state, window, AllOrAny::All))
                } else {
                    false
                };
                let classes = if let Some(classes) = class {
                    classes
                        .iter()
                        .any(|class| window.class().as_ref() == Some(class))
                } else {
                    false
                };
                let titles = if let Some(titles) = title {
                    titles
                        .iter()
                        .any(|title| window.title().as_ref() == Some(title))
                } else {
                    false
                };
                let tags = if let Some(tag_ids) = tag {
                    let mut tags = tag_ids.iter().filter_map(|tag_id| tag_id.tag(state));
                    tags.any(|tag| window.with_state(|state| state.tags.contains(&tag)))
                } else {
                    false
                };
                cond_all || cond_any || classes || titles || tags
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WindowRule {
    /// Set the output the window will open on.
    #[serde(default)]
    pub output: Option<OutputName>,
    /// Set the tags the output will have on open.
    #[serde(default)]
    pub tags: Option<Vec<TagId>>,
    /// Set the window to floating or tiled on open.
    #[serde(default)]
    pub floating_or_tiled: Option<FloatingOrTiled>,
    /// Set the window to fullscreen, maximized, or force it to neither.
    #[serde(default)]
    pub fullscreen_or_maximized: Option<FullscreenOrMaximized>,
    /// Set the window's initial size.
    #[serde(default)]
    pub size: Option<(NonZeroU32, NonZeroU32)>,
    /// Set the window's initial location. If the window is tiled, it will snap to this position
    /// when set to floating.
    #[serde(default)]
    pub location: Option<(i32, i32)>,
}

// TODO: just skip serializing fields on the other FloatingOrTiled
#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FloatingOrTiled {
    Floating,
    Tiled,
}
