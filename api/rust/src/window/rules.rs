use std::num::NonZeroU32;

use crate::{msg::Msg, output::OutputHandle, send_msg, tag::TagHandle};

use super::{FloatingOrTiled, FullscreenOrMaximized};

#[derive(Clone, Copy)]
pub struct WindowRules;

impl WindowRules {
    pub fn add(&self, cond: WindowRuleCondition, rule: WindowRule) {
        let msg = Msg::AddWindowRule {
            cond: cond.0,
            rule: rule.0,
        };

        send_msg(msg).unwrap();
    }
}

#[derive(Default)]
pub struct WindowRule(crate::msg::WindowRule);

impl WindowRule {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn output(mut self, output: &OutputHandle) -> Self {
        self.0.output = Some(output.0.clone());
        self
    }

    pub fn tags(mut self, tags: &[TagHandle]) -> Self {
        self.0.tags = Some(tags.iter().map(|tag| tag.0).collect());
        self
    }

    pub fn floating_or_tiled(mut self, floating_or_tiled: FloatingOrTiled) -> Self {
        self.0.floating_or_tiled = Some(floating_or_tiled);
        self
    }

    pub fn fullscreen_or_maximized(
        mut self,
        fullscreen_or_maximized: FullscreenOrMaximized,
    ) -> Self {
        self.0.fullscreen_or_maximized = Some(fullscreen_or_maximized);
        self
    }

    pub fn size(mut self, width: NonZeroU32, height: NonZeroU32) -> Self {
        self.0.size = Some((width, height));
        self
    }

    pub fn location(mut self, x: i32, y: i32) -> Self {
        self.0.location = Some((x, y));
        self
    }
}

#[derive(Default, Debug)]
pub struct WindowRuleCondition(crate::msg::WindowRuleCondition);

impl WindowRuleCondition {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn any(mut self, conds: &[WindowRuleCondition]) -> Self {
        self.0.cond_any = Some(conds.iter().map(|cond| cond.0.clone()).collect());
        self
    }

    pub fn all(mut self, conds: &[WindowRuleCondition]) -> Self {
        self.0.cond_all = Some(conds.iter().map(|cond| cond.0.clone()).collect());
        self
    }

    pub fn class(mut self, classes: &[&str]) -> Self {
        self.0.class = Some(classes.iter().map(|s| s.to_string()).collect());
        self
    }

    pub fn title(mut self, titles: &[&str]) -> Self {
        self.0.title = Some(titles.iter().map(|s| s.to_string()).collect());
        self
    }

    pub fn tag(mut self, tags: &[TagHandle]) -> Self {
        self.0.tag = Some(tags.iter().map(|tag| tag.0).collect());
        self
    }
}
