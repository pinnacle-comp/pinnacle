// SPDX-License-Identifier: GPL-3.0-or-later

use std::num::NonZeroU32;

use smithay::wayland::{compositor, shell::xdg::XdgToplevelSurfaceData};

use crate::{
    output::OutputName,
    state::{State, WithState},
    tag::TagId,
    window::{window_state::FullscreenOrMaximized, WindowElement},
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowRuleCondition {
    /// This condition is met when any of the conditions provided is met.
    CondAny(Vec<WindowRuleCondition>),
    /// This condition is met when all of the conditions provided are met.
    CondAll(Vec<WindowRuleCondition>),
    /// This condition is met when the class matches.
    Class(String),
    /// This condition is met when the title matches.
    Title(String),
    /// This condition is met when the tag matches.
    Tag(TagId),
}

impl WindowRuleCondition {
    /// RefCell Safety: This method uses RefCells on `window`.
    pub fn is_met(&self, state: &State, window: &WindowElement) -> bool {
        match self {
            WindowRuleCondition::CondAny(conds) => {
                conds.iter().any(|cond| Self::is_met(cond, state, window))
            }
            WindowRuleCondition::CondAll(conds) => {
                conds.iter().all(|cond| Self::is_met(cond, state, window))
            }
            WindowRuleCondition::Class(class) => {
                let Some(wl_surf) = window.wl_surface() else {
                    return false;
                };

                let current_class = compositor::with_states(&wl_surf, |states| {
                    states
                        .data_map
                        .get::<XdgToplevelSurfaceData>()
                        .expect("XdgToplevelSurfaceData wasn't in surface's data map")
                        .lock()
                        .expect("Failed to lock Mutex<XdgToplevelSurfaceData>")
                        .app_id
                        .clone()
                });

                current_class.as_ref() == Some(class)
            }
            WindowRuleCondition::Title(title) => {
                let Some(wl_surf) = window.wl_surface() else {
                    return false;
                };

                let current_title = compositor::with_states(&wl_surf, |states| {
                    states
                        .data_map
                        .get::<XdgToplevelSurfaceData>()
                        .expect("XdgToplevelSurfaceData wasn't in surface's data map")
                        .lock()
                        .expect("Failed to lock Mutex<XdgToplevelSurfaceData>")
                        .title
                        .clone()
                });

                current_title.as_ref() == Some(title)
            }
            WindowRuleCondition::Tag(tag) => {
                let Some(tag) = tag.tag(state) else {
                    tracing::warn!("WindowRuleCondition no tag");
                    return false;
                };

                window.with_state(|state| state.tags.contains(&tag))
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
