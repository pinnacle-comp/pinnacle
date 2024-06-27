// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Types for window rules.
//!
//! A window rule is a way to set the properties of a window on open.
//!
//! They are comprised of two parts: the [condition][WindowRuleCondition] and the actual [rule][WindowRule].
//!
//! # [`WindowRuleCondition`]s
//! `WindowRuleCondition`s are conditions that the window needs to open with in order to apply a
//! rule. For example, you may want to set a window to maximized if it has the class "steam", or
//! you might want to open all Firefox instances on tag "3".
//!
//! To do this, you must build a `WindowRuleCondition` to tell the compositor when to apply any
//! rules.
//!
//! ## Building `WindowRuleCondition`s
//! A condition is created through [`WindowRuleCondition::new`]:
//! ```
//! let cond = WindowRuleCondition::new();
//! ```
//!
//! In order to understand conditions, you must understand the concept of "any" and "all".
//!
//! **"Any"**
//!
//! "Any" conditions only need one of their constituent items to be true for the whole condition to
//! evaluate to true. Think of it as one big `if a || b || c || d || ... {}` block.
//!
//! **"All"**
//!
//! "All" conditions need *all* of their constituent items to be true for the condition to evaluate
//! to true. This is like a big `if a && b && c && d && ... {}` block.
//!
//! Note that any items in a top level `WindowRuleCondition` fall under "all", so all those items
//! must be true.
//!
//! With that out of the way, we can get started building conditions.
//!
//! ### `WindowRuleCondition::classes`
//! With [`WindowRuleCondition::classes`], you can specify what classes a window needs to have for
//! a rule to apply.
//!
//! The following will apply to windows with the class "firefox":
//! ```
//! let cond = WindowRuleCondition::new().classes(["firefox"]);
//! ```
//!
//! Note that you pass in some `impl IntoIterator<Item = impl Into<String>>`. This means you can
//! pass in more than one class here:
//! ```
//! let failing_cond = WindowRuleCondition::new().classes(["firefox", "steam"]);
//! ```
//! *HOWEVER*: this will not work. Recall that top level conditions are implicitly "all". This
//! means the above would require windows to have *both classes*, which is impossible. Thus, the
//! condition above will never be true.
//!
//! ### `WindowRuleCondition::titles`
//! Like `classes`, you can use `titles` to specify that the window needs to open with a specific
//! title for the condition to apply.
//!
//! ```
//! let cond = WindowRuleCondition::new().titles(["Steam"]);
//! ```
//!
//! Like `classes`, passing in multiple titles at the top level will cause the condition to always
//! fail.
//!
//! ### `WindowRuleCondition::tags`
//! You can specify that the window needs to open on the given tags in order to apply a rule.
//!
//! ```
//! let cond = WindowRuleCondition::new().tags([&tag.get("3", output.get_by_name("HDMI-1")?)?]);
//! ```
//!
//! Here, if you have tag "3" active on "HDMI-1" and spawn a window on that output, this condition
//! will apply.
//!
//! Unlike `classes` and `titles`, you can specify multiple tags at the top level:
//!
//! ```
//! let op = output.get_by_name("HDMI-1")?;
//! let tag1 = tag.get("1", &op)?;
//! let tag2 = tag.get("2", &op)?;
//!
//! let cond = WindowRuleCondition::new().tags([&tag1, &tag2]);
//! ```
//!
//! Now, you must have both tags "1" and "2" active and spawn a window for the condition to apply.
//!
//! ### `WindowRuleCondition::any`
//! Now we can get to ways to compose more complex conditions.
//!
//! `WindowRuleCondition::any` takes in conditions and will evaluate to true if *anything* in those
//! conditions are true.
//!
//! ```
//! let cond = WindowRuleCondition::new()
//!     .any([
//!         WindowRuleCondition::new().classes(["Alacritty"]),
//!         WindowRuleCondition::new().tags([&tag.get("2", None)?]),
//!     ]);
//! ```
//!
//! This condition will apply if the window is *either* "Alacritty" *or* opens on tag "2".
//!
//! ### `WindowRuleCondition::all`
//! With `WindowRuleCondition::all`, *all* specified conditions must be true for the condition to
//! be true.
//!
//! ```
//! let cond = WindowRuleCondition::new()
//!     .all([
//!         WindowRuleCondition::new().classes(["Alacritty"]),
//!         WindowRuleCondition::new().tags([&tag.get("2", None)?]),
//!     ]);
//! ```
//!
//! This condition applies if the window has the class "Alacritty" *and* opens on tag "2".
//!
//! You can write the above a bit shorter, as top level conditions are already "all":
//!
//! ```
//! let cond = WindowRuleCondition::new()
//!     .classes(["Alacritty"])
//!     .tags([&tag.get("2", None)?]);
//! ```
//!
//! ## Complex condition composition
//! You can arbitrarily nest `any` and `all` to achieve desired logic.
//!
//! ```
//! let op = output.get_by_name("HDMI-1")?;
//! let tag1 = tag.get("1", &op)?;
//! let tag2 = tag.get("2", &op)?;
//!
//! let complex_cond = WindowRuleCondition::new()
//!     .any([
//!         WindowRuleCondition::new().all([
//!             WindowRuleCondition::new()
//!                 .classes("Alacritty")
//!                 .tags([&tag1, &tag2])
//!         ]),
//!         WindowRuleCondition::new().all([
//!             WindowRuleCondition::new().any([
//!                 WindowRuleCondition::new().titles(["nvim", "emacs", "nano"]),
//!             ]),
//!             WindowRuleCondition::new().any([
//!                 WindowRuleCondition::new().tags([&tag1, &tag2]),
//!             ]),
//!         ])
//!     ])
//! ```
//!
//! The above is true if either of the following are true:
//! - The window has class "Alacritty" and opens on both tags "1" and "2", or
//! - The window's class is either "nvim", "emacs", or "nano" *and* it opens on either tag "1" or
//!   "2".
//!
//! # [`WindowRule`]s
//! `WindowRuleCondition`s are half of a window rule. The other half is the [`WindowRule`] itself.
//!
//! A `WindowRule` is what will apply to a window if a condition is true.
//!
//! ## Building `WindowRule`s
//!
//! Create a new window rule with [`WindowRule::new`]:
//!
//! ```
//! let rule = WindowRule::new();
//! ```
//!
//! There are several rules you can set currently.
//!
//! ### [`WindowRule::output`]
//! This will cause the window to open on the specified output.
//!
//! ### [`WindowRule::tags`]
//! This will cause the window to open with the given tags.
//!
//! ### [`WindowRule::floating`]
//! This will cause the window to open either floating or tiled.
//!
//! ### [`WindowRule::fullscreen_or_maximized`]
//! This will cause the window to open either fullscreen, maximized, or neither.
//!
//! ### [`WindowRule::x`]
//! This will cause the window to open at the given x-coordinate.
//!
//! Note: this only applies to floating windows; tiled windows' geometry will be overridden by
//! layouting.
//!
//! ### [`WindowRule::y`]
//! This will cause the window to open at the given y-coordinate.
//!
//! Note: this only applies to floating windows; tiled windows' geometry will be overridden by
//! layouting.
//!
//! ### [`WindowRule::width`]
//! This will cause the window to open with the given width in pixels.
//!
//! Note: this only applies to floating windows; tiled windows' geometry will be overridden by
//! layouting.
//!
//! ### [`WindowRule::height`]
//! This will cause the window to open with the given height in pixels.
//!
//! Note: this only applies to floating windows; tiled windows' geometry will be overridden by
//! layouting.

use pinnacle_api_defs::pinnacle::window;

use crate::{output::OutputHandle, tag::TagHandle};

use super::FullscreenOrMaximized;

/// A condition for a [`WindowRule`] to apply to a window.
///
/// `WindowRuleCondition`s are built using the builder pattern.
#[derive(Default, Debug, Clone)]
pub struct WindowRuleCondition(pub(super) window::v0alpha1::WindowRuleCondition);

impl WindowRuleCondition {
    /// Create a new, empty `WindowRuleCondition`.
    pub fn new() -> Self {
        Default::default()
    }

    /// This condition requires that at least one provided condition is true.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::window::rules::WindowRuleCondition;
    ///
    /// // `cond` will be true if the window opens with *either* class "Alacritty" or "firefox"
    /// // *or* with title "Steam"
    /// let cond = WindowRuleCondition::new()
    ///     .any([
    ///         WindowRuleCondition::new().classes(["Alacritty", "firefox"]),
    ///         WindowRuleCondition::new().titles(["Steam"]).
    ///     ]);
    /// ```
    pub fn any(mut self, conds: impl IntoIterator<Item = WindowRuleCondition>) -> Self {
        self.0.any = conds.into_iter().map(|cond| cond.0).collect();
        self
    }

    /// This condition requires that all provided conditions are true.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::window::rules::WindowRuleCondition;
    ///
    /// // `cond` will be true if the window opens with class "Alacritty" *and* on tag "1"
    /// let cond = WindowRuleCondition::new()
    ///     .any([
    ///         WindowRuleCondition::new().tags([tag.get("1", None)?]),
    ///         WindowRuleCondition::new().titles(["Alacritty"]).
    ///     ]);
    /// ```
    pub fn all(mut self, conds: impl IntoIterator<Item = WindowRuleCondition>) -> Self {
        self.0.all = conds.into_iter().map(|cond| cond.0).collect();
        self
    }

    /// This condition requires that the window's class matches.
    ///
    /// When used in a top level condition or inside of [`WindowRuleCondition::all`],
    /// *all* classes must match (this is impossible).
    ///
    /// When used in [`WindowRuleCondition::any`], at least one of the
    /// provided classes must match.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::window::rules::WindowRuleCondition;
    ///
    /// // `cond` will be true if the window opens with class "Alacritty"
    /// let cond = WindowRuleCondition::new().classes(["Alacritty"]);
    ///
    /// // Top level conditions need all items to be true,
    /// // so the following will never be true as windows can't have two classes at once
    /// let always_false = WindowRuleCondition::new().classes(["Alacritty", "firefox"]);
    ///
    /// // To make the above work, use [`WindowRuleCondition::any`].
    /// // The following will be true if the window is "Alacritty" or "firefox"
    /// let any_class = WindowRuleCondition::new()
    ///     .any([ WindowRuleCondition::new().classes(["Alacritty", "firefox"]) ]);
    /// ```
    pub fn classes(mut self, classes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.0.classes = classes.into_iter().map(Into::into).collect();
        self
    }

    /// This condition requires that the window's title matches.
    ///
    /// When used in a top level condition or inside of [`WindowRuleCondition::all`],
    /// *all* titles must match (this is impossible).
    ///
    /// When used in [`WindowRuleCondition::any`], at least one of the
    /// provided titles must match.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::window::rules::WindowRuleCondition;
    ///
    /// // `cond` will be true if the window opens with title "vim"
    /// let cond = WindowRuleCondition::new().titles(["vim"]);
    ///
    /// // Top level conditions need all items to be true,
    /// // so the following will never be true as windows can't have two titles at once
    /// let always_false = WindowRuleCondition::new().titles(["vim", "emacs"]);
    ///
    /// // To make the above work, use [`WindowRuleCondition::any`].
    /// // The following will be true if the window has the title "vim" or "emacs"
    /// let any_title = WindowRuleCondition::new()
    ///     .any([WindowRuleCondition::new().titles(["vim", "emacs"])]);
    /// ```
    pub fn titles(mut self, titles: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.0.titles = titles.into_iter().map(Into::into).collect();
        self
    }

    /// This condition requires that the window's is opened on the given tags.
    ///
    /// When used in a top level condition or inside of [`WindowRuleCondition::all`],
    /// the window must open on *all* given tags.
    ///
    /// When used in [`WindowRuleCondition::any`], the window must open on at least
    /// one of the given tags.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::window::rules::WindowRuleCondition;
    ///
    /// let tag1 = tag.get("1", None)?;
    /// let tag2 = tag.get("2", None)?;
    ///
    /// // `cond` will be true if the window opens with tag "1"
    /// let cond = WindowRuleCondition::new().tags([&tag1]);
    ///
    /// // Top level conditions need all items to be true,
    /// // so the following will be true if the window opens with both tags "1" and "2"
    /// let all_tags = WindowRuleCondition::new().tags([&tag1, &tag2]);
    ///
    /// // This does the same as the above
    /// let all_tags = WindowRuleCondition::new()
    ///     .all([WindowRuleCondition::new().tags([&tag1, &tag2])]);
    ///
    /// // The following will be true if the window opens with *either* tag "1" or "2"
    /// let any_tag = WindowRuleCondition::new()
    ///     .any([WindowRuleCondition::new().tags([&tag1, &tag2])]);
    /// ```
    pub fn tags<'a>(mut self, tags: impl IntoIterator<Item = &'a TagHandle>) -> Self {
        self.0.tags = tags.into_iter().map(|tag| tag.id).collect();
        self
    }
}

/// A window rule.
///
/// This is what will be applied to a window if it meets a [`WindowRuleCondition`].
///
/// `WindowRule`s are built using the builder pattern.
#[derive(Clone, Debug, Default)]
pub struct WindowRule(pub(super) window::v0alpha1::WindowRule);

impl WindowRule {
    /// Create a new, empty window rule.
    pub fn new() -> Self {
        Default::default()
    }

    /// This rule will force windows to open on the provided `output`.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::window::rules::WindowRule;
    ///
    /// // Force the window to open on "HDMI-1"
    /// let rule = WindowRule::new().output(output.get_by_name("HDMI-1")?);
    /// ```
    pub fn output(mut self, output: &OutputHandle) -> Self {
        self.0.output = Some(output.name.clone());
        self
    }

    /// This rule will force windows to open with the provided `tags`.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::window::rules::WindowRule;
    ///
    /// let op = output.get_by_name("HDMI-1")?;
    /// let tag1 = tag.get("1", &op)?;
    /// let tag2 = tag.get("2", &op)?;
    ///
    /// // Force the window to open with tags "1" and "2"
    /// let rule = WindowRule::new().tags([&tag1, &tag2]);
    /// ```
    pub fn tags<'a>(mut self, tags: impl IntoIterator<Item = &'a TagHandle>) -> Self {
        self.0.tags = tags.into_iter().map(|tag| tag.id).collect();
        self
    }

    /// This rule will force windows to open either floating if true or tiled if false.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::window::rules::WindowRule;
    ///
    /// // Force the window to open floating
    /// let rule = WindowRule::new().floating(true);
    ///
    /// // Force the window to open tiled
    /// let rule = WindowRule::new().floating(false);
    /// ```
    pub fn floating(mut self, floating: bool) -> Self {
        self.0.set_state(match floating {
            true => window::v0alpha1::WindowState::Floating,
            false => window::v0alpha1::WindowState::Tiled,
        });
        self
    }

    /// This rule will force windows to open either fullscreen, maximized, or neither.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::window::rules::WindowRule;
    /// use pinnacle_api::window::FullscreenOrMaximized;
    ///
    /// // Force the window to open fullscreen
    /// let rule = WindowRule::new().fullscreen_or_maximized(FullscreenOrMaximized::Fullscreen);
    ///
    /// // Force the window to open maximized
    /// let rule = WindowRule::new().fullscreen_or_maximized(FullscreenOrMaximized::Maximized);
    ///
    /// // Force the window to open not fullscreen nor maximized
    /// let rule = WindowRule::new().fullscreen_or_maximized(FullscreenOrMaximized::Neither);
    /// ```
    #[deprecated = "use the `fullscreen` or `maximized` methods instead"]
    pub fn fullscreen_or_maximized(
        mut self,
        fullscreen_or_maximized: FullscreenOrMaximized,
    ) -> Self {
        self.0.fullscreen_or_maximized = Some(fullscreen_or_maximized as i32);
        self
    }

    /// This rule will force windows to open fullscreen.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::window::rules::WindowRule;
    ///
    /// // Force the window to open fullscreen
    /// let rule = WindowRule::new().fullscreen();
    /// ```
    pub fn fullscreen(mut self) -> Self {
        self.0.set_state(window::v0alpha1::WindowState::Fullscreen);
        self
    }

    /// This rule will force windows to open maximized.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::window::rules::WindowRule;
    ///
    /// // Force the window to open fullscreen
    /// let rule = WindowRule::new().maximized();
    /// ```
    pub fn maximized(mut self) -> Self {
        self.0.set_state(window::v0alpha1::WindowState::Maximized);
        self
    }

    /// This rule will force windows to open at a specific x-coordinate.
    ///
    /// This will only actually be visible if the window is also floating.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::window::rules::WindowRule;
    ///
    /// // Force the window to open at x = 480
    /// let rule = WindowRule::new().x(480);
    /// ```
    pub fn x(mut self, x: i32) -> Self {
        self.0.x = Some(x);
        self
    }

    /// This rule will force windows to open at a specific y-coordinate.
    ///
    /// This will only actually be visible if the window is also floating.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::window::rules::WindowRule;
    ///
    /// // Force the window to open at y = 240
    /// let rule = WindowRule::new().y(240);
    /// ```
    pub fn y(mut self, y: i32) -> Self {
        self.0.y = Some(y);
        self
    }

    /// This rule will force windows to open with a specific width.
    ///
    /// This will only actually be visible if the window is also floating.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::window::rules::WindowRule;
    ///
    /// // Force the window to open with a width of 500 pixels
    /// let rule = WindowRule::new().width(500);
    /// ```
    pub fn width(mut self, width: u32) -> Self {
        self.0.width = Some(width as i32);
        self
    }

    /// This rule will force windows to open with a specific height.
    ///
    /// This will only actually be visible if the window is also floating.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::window::rules::WindowRule;
    ///
    /// // Force the window to open with a height of 250 pixels
    /// let rule = WindowRule::new().height(250);
    /// ```
    pub fn height(mut self, height: u32) -> Self {
        self.0.height = Some(height as i32);
        self
    }

    /// This rule will force windows into the specified decoration mode.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::window::rules::WindowRule;
    /// use pinnacle_api::window::rules::DecorationMode;
    ///
    /// // Currently, disable client-side decorations.
    /// // In the future, Pinnacle will have the ability to draw its own decorations.
    /// let rule = WindowRule::new().decoration_mode(DecorationMode::ServerSide);
    /// ```
    pub fn decoration_mode(mut self, mode: DecorationMode) -> Self {
        self.0.ssd = Some(match mode {
            DecorationMode::ClientSide => false,
            DecorationMode::ServerSide => true,
        });
        self
    }
}

/// The desired decoration mode.
pub enum DecorationMode {
    /// The client will draw its own decorations.
    ClientSide,
    /// The server will draw the decorations.
    ServerSide,
}
