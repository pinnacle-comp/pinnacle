//! Window rules.

use std::num::NonZeroU32;

use crate::{msg::Msg, output::OutputHandle, send_msg, tag::TagHandle};

use super::{FloatingOrTiled, FullscreenOrMaximized};

/// Add a window rule.
pub fn add(cond: WindowRuleCondition, rule: WindowRule) {
    let msg = Msg::AddWindowRule {
        cond: cond.0,
        rule: rule.0,
    };

    send_msg(msg).unwrap();
}

/// A window rule.
///
/// This is what will be applied to a window if it meets a [`WindowRuleCondition`].
///
/// `WindowRule`s are built using the builder pattern.
/// // TODO: show example
#[derive(Default)]
pub struct WindowRule(crate::msg::WindowRule);

impl WindowRule {
    /// Create a new, empty window rule.
    pub fn new() -> Self {
        Default::default()
    }

    /// This rule will force windows to open on the provided `output`.
    pub fn output(mut self, output: &OutputHandle) -> Self {
        self.0.output = Some(output.0.clone());
        self
    }

    /// This rule will force windows to open with the provided `tags`.
    pub fn tags(mut self, tags: &[TagHandle]) -> Self {
        self.0.tags = Some(tags.iter().map(|tag| tag.0).collect());
        self
    }

    /// This rule will force windows to open either floating or tiled.
    pub fn floating_or_tiled(mut self, floating_or_tiled: FloatingOrTiled) -> Self {
        self.0.floating_or_tiled = Some(floating_or_tiled);
        self
    }

    /// This rule will force windows to open either fullscreen, maximized, or neither.
    pub fn fullscreen_or_maximized(
        mut self,
        fullscreen_or_maximized: FullscreenOrMaximized,
    ) -> Self {
        self.0.fullscreen_or_maximized = Some(fullscreen_or_maximized);
        self
    }

    /// This rule will force windows to open with a specific size.
    ///
    /// This will only actually be visible if the window is also floating.
    pub fn size(mut self, width: NonZeroU32, height: NonZeroU32) -> Self {
        self.0.size = Some((width, height));
        self
    }

    /// This rule will force windows to open at a specific location.
    ///
    /// This will only actually be visible if the window is also floating.
    pub fn location(mut self, x: i32, y: i32) -> Self {
        self.0.location = Some((x, y));
        self
    }
}

/// A condition for a [`WindowRule`] to apply to a window.
#[derive(Default, Debug)]
pub struct WindowRuleCondition(crate::msg::WindowRuleCondition);

impl WindowRuleCondition {
    /// Create a new, empty `WindowRuleCondition`.
    pub fn new() -> Self {
        Default::default()
    }

    /// This condition requires that at least one provided condition is true.
    pub fn any(mut self, conds: &[WindowRuleCondition]) -> Self {
        self.0.cond_any = Some(conds.iter().map(|cond| cond.0.clone()).collect());
        self
    }

    /// This condition requires that all provided conditions are true.
    pub fn all(mut self, conds: &[WindowRuleCondition]) -> Self {
        self.0.cond_all = Some(conds.iter().map(|cond| cond.0.clone()).collect());
        self
    }

    /// This condition requires that the window's class matches.
    ///
    /// When used in a top level condition or inside of [`WindowRuleCondition::all`],
    /// *all* classes must match (this is impossible).
    ///
    /// When used in [`WindowRuleCondition::any`], at least one of the
    /// provided classes must match.
    pub fn class(mut self, classes: &[&str]) -> Self {
        self.0.class = Some(classes.iter().map(|s| s.to_string()).collect());
        self
    }

    /// This condition requires that the window's title matches.
    ///
    /// When used in a top level condition or inside of [`WindowRuleCondition::all`],
    /// *all* titles must match (this is impossible).
    ///
    /// When used in [`WindowRuleCondition::any`], at least one of the
    /// provided titles must match.
    pub fn title(mut self, titles: &[&str]) -> Self {
        self.0.title = Some(titles.iter().map(|s| s.to_string()).collect());
        self
    }

    /// This condition requires that the window's is opened on the given tags.
    ///
    /// When used in a top level condition or inside of [`WindowRuleCondition::all`],
    /// the window must open on *all* given tags.
    ///
    /// When used in [`WindowRuleCondition::any`], the window must open on at least
    /// one of the given tags.
    pub fn tag(mut self, tags: &[TagHandle]) -> Self {
        self.0.tag = Some(tags.iter().map(|tag| tag.0).collect());
        self
    }
}
