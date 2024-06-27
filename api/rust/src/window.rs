// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Window management.
//!
//! This module provides [`Window`], which allows you to get [`WindowHandle`]s and move and resize
//! windows using the mouse.
//!
//! [`WindowHandle`]s allow you to do things like resize and move windows, toggle them between
//! floating and tiled, close them, and more.
//!
//! This module also allows you to set window rules; see the [rules] module for more information.

use std::sync::OnceLock;

use futures::FutureExt;
use num_enum::TryFromPrimitive;
use pinnacle_api_defs::pinnacle::{
    v0alpha1::SetOrToggle,
    window::{
        self,
        v0alpha1::{
            window_service_client::WindowServiceClient, AddWindowRuleRequest, CloseRequest,
            GetRequest, MoveGrabRequest, MoveToTagRequest, RaiseRequest, ResizeGrabRequest,
            SetFloatingRequest, SetFocusedRequest, SetFullscreenRequest, SetMaximizedRequest,
            SetTagRequest,
        },
    },
};
use tonic::transport::Channel;

use crate::{
    block_on_tokio,
    input::MouseButton,
    signal::{SignalHandle, WindowSignal},
    tag::TagHandle,
    util::{Batch, Geometry},
    ApiModules,
};

use self::rules::{WindowRule, WindowRuleCondition};

pub mod rules;

/// A struct containing methods that get [`WindowHandle`]s and move windows with the mouse.
///
/// See [`WindowHandle`] for more information.
#[derive(Debug, Clone)]
pub struct Window {
    window_client: WindowServiceClient<Channel>,
    api: OnceLock<ApiModules>,
}

impl Window {
    pub(crate) fn new(channel: Channel) -> Self {
        Self {
            window_client: WindowServiceClient::new(channel.clone()),
            api: OnceLock::new(),
        }
    }

    pub(crate) fn finish_init(&self, api: ApiModules) {
        self.api.set(api).unwrap();
    }

    pub(crate) fn new_handle(&self, id: u32) -> WindowHandle {
        WindowHandle {
            id,
            window_client: self.window_client.clone(),
            api: self.api.get().unwrap().clone(),
        }
    }

    /// Start moving the window with the mouse.
    ///
    /// This will begin moving the window under the pointer using the specified [`MouseButton`].
    /// The button must be held down at the time this method is called for the move to start.
    ///
    /// This is intended to be used with [`Input::mousebind`][crate::input::Input::mousebind].
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::input::{Mod, MouseButton, MouseEdge};
    ///
    /// // Set `Super + left click` to begin moving a window
    /// input.mousebind([Mod::Super], MouseButton::Left, MouseEdge::Press, || {
    ///     window.begin_move(MouseButton::Left);
    /// });
    /// ```
    pub fn begin_move(&self, button: MouseButton) {
        let mut client = self.window_client.clone();
        if let Err(status) = block_on_tokio(client.move_grab(MoveGrabRequest {
            button: Some(button as u32),
        })) {
            eprintln!("ERROR: {status}");
        }
    }

    /// Start resizing the window with the mouse.
    ///
    /// This will begin resizing the window under the pointer using the specified [`MouseButton`].
    /// The button must be held down at the time this method is called for the resize to start.
    ///
    /// This is intended to be used with [`Input::mousebind`][crate::input::Input::mousebind].
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::input::{Mod, MouseButton, MouseEdge};
    ///
    /// // Set `Super + right click` to begin moving a window
    /// input.mousebind([Mod::Super], MouseButton::Right, MouseEdge::Press, || {
    ///     window.begin_resize(MouseButton::Right);
    /// });
    /// ```
    pub fn begin_resize(&self, button: MouseButton) {
        let mut client = self.window_client.clone();
        block_on_tokio(client.resize_grab(ResizeGrabRequest {
            button: Some(button as u32),
        }))
        .unwrap();
    }

    /// Get all windows.
    ///
    /// # Examples
    ///
    /// ```
    /// let windows = window.get_all();
    /// ```
    pub fn get_all(&self) -> Vec<WindowHandle> {
        block_on_tokio(self.get_all_async())
    }

    /// The async version of [`Window::get_all`].
    pub async fn get_all_async(&self) -> Vec<WindowHandle> {
        let mut client = self.window_client.clone();
        client
            .get(GetRequest {})
            .await
            .unwrap()
            .into_inner()
            .window_ids
            .into_iter()
            .map(move |id| self.new_handle(id))
            .collect::<Vec<_>>()
    }

    /// Get the currently focused window.
    ///
    /// # Examples
    ///
    /// ```
    /// let focused_window = window.get_focused()?;
    /// ```
    pub fn get_focused(&self) -> Option<WindowHandle> {
        block_on_tokio(self.get_focused_async())
    }

    /// The async version of [`Window::get_focused`].
    pub async fn get_focused_async(&self) -> Option<WindowHandle> {
        self.get_all_async().await.batch_find(
            |win| win.focused_async().boxed(),
            |focused| focused.is_some_and(|focused| focused),
        )
    }

    /// Add a window rule.
    ///
    /// A window rule is a set of criteria that a window must open with.
    /// For it to apply, a [`WindowRuleCondition`] must evaluate to true for the window in question.
    ///
    /// See the [`rules`] module for more information.
    pub fn add_window_rule(&self, cond: WindowRuleCondition, rule: WindowRule) {
        let mut client = self.window_client.clone();

        block_on_tokio(client.add_window_rule(AddWindowRuleRequest {
            cond: Some(cond.0),
            rule: Some(rule.0),
        }))
        .unwrap();
    }

    /// Connect to a window signal.
    ///
    /// The compositor will fire off signals that your config can listen for and act upon.
    /// You can pass in a [`WindowSignal`] along with a callback and it will get run
    /// with the necessary arguments every time a signal of that type is received.
    pub fn connect_signal(&self, signal: WindowSignal) -> SignalHandle {
        let mut signal_state = block_on_tokio(self.api.get().unwrap().signal.write());

        match signal {
            WindowSignal::PointerEnter(f) => signal_state.window_pointer_enter.add_callback(f),
            WindowSignal::PointerLeave(f) => signal_state.window_pointer_leave.add_callback(f),
        }
    }
}

/// A handle to a window.
///
/// This allows you to manipulate the window and get its properties.
#[derive(Debug, Clone)]
pub struct WindowHandle {
    id: u32,
    window_client: WindowServiceClient<Channel>,
    api: ApiModules,
}

impl PartialEq for WindowHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for WindowHandle {}

impl std::hash::Hash for WindowHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// Whether a window is fullscreen, maximized, or neither.
#[repr(i32)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, TryFromPrimitive)]
pub enum FullscreenOrMaximized {
    /// The window is neither fullscreen nor maximized
    Neither = 1,
    /// The window is fullscreen
    Fullscreen,
    /// The window is maximized
    Maximized,
}

/// A window's current display state.
#[repr(i32)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, TryFromPrimitive)]
pub enum WindowState {
    /// The window is tiled.
    Tiled = 1,
    /// The window is floating.
    Floating,
    /// The window is fullscreen.
    Fullscreen,
    /// The window is maximized.
    Maximized,
}

/// Properties of a window.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct WindowProperties {
    /// The location and size of the window
    pub geometry: Option<Geometry>,
    /// The window's class
    pub class: Option<String>,
    /// The window's title
    pub title: Option<String>,
    /// Whether the window is focused or not
    pub focused: Option<bool>,
    /// Whether the window is floating or not
    ///
    /// Note that a window can still be floating even if it's fullscreen or maximized; those two
    /// states will just override the floating state.
    #[deprecated = "use `state` instead"]
    pub floating: Option<bool>,
    /// Whether the window is fullscreen, maximized, or neither
    #[deprecated = "use `state` instead"]
    pub fullscreen_or_maximized: Option<FullscreenOrMaximized>,
    /// All the tags on the window
    pub tags: Vec<TagHandle>,
    /// The state of the window.
    pub state: Option<WindowState>,
}

impl WindowHandle {
    /// Send a close request to this window.
    ///
    /// If the window is unresponsive, it may not close.
    ///
    /// # Examples
    ///
    /// ```
    /// // Close the focused window
    /// window.get_focused()?.close()
    /// ```
    pub fn close(&self) {
        let mut window_client = self.window_client.clone();
        block_on_tokio(window_client.close(CloseRequest {
            window_id: Some(self.id),
        }))
        .unwrap();
    }

    /// Set this window to fullscreen or not.
    ///
    /// If it is maximized, setting it to fullscreen will remove the maximized state.
    ///
    /// # Examples
    ///
    /// ```
    /// // Set the focused window to fullscreen.
    /// window.get_focused()?.set_fullscreen(true);
    /// ```
    pub fn set_fullscreen(&self, set: bool) {
        let mut client = self.window_client.clone();
        block_on_tokio(client.set_fullscreen(SetFullscreenRequest {
            window_id: Some(self.id),
            set_or_toggle: Some(match set {
                true => SetOrToggle::Set,
                false => SetOrToggle::Unset,
            } as i32),
        }))
        .unwrap();
    }

    /// Toggle this window between fullscreen and not.
    ///
    /// If it is maximized, toggling it to fullscreen will remove the maximized state.
    ///
    /// # Examples
    ///
    /// ```
    /// // Toggle the focused window to and from fullscreen.
    /// window.get_focused()?.toggle_fullscreen();
    /// ```
    pub fn toggle_fullscreen(&self) {
        let mut client = self.window_client.clone();
        block_on_tokio(client.set_fullscreen(SetFullscreenRequest {
            window_id: Some(self.id),
            set_or_toggle: Some(SetOrToggle::Toggle as i32),
        }))
        .unwrap();
    }

    /// Set this window to maximized or not.
    ///
    /// If it is fullscreen, setting it to maximized will remove the fullscreen state.
    ///
    /// # Examples
    ///
    /// ```
    /// // Set the focused window to maximized.
    /// window.get_focused()?.set_maximized(true);
    /// ```
    pub fn set_maximized(&self, set: bool) {
        let mut client = self.window_client.clone();
        block_on_tokio(client.set_maximized(SetMaximizedRequest {
            window_id: Some(self.id),
            set_or_toggle: Some(match set {
                true => SetOrToggle::Set,
                false => SetOrToggle::Unset,
            } as i32),
        }))
        .unwrap();
    }

    /// Toggle this window between maximized and not.
    ///
    /// If it is fullscreen, toggling it to maximized will remove the fullscreen state.
    ///
    /// # Examples
    ///
    /// ```
    /// // Toggle the focused window to and from maximized.
    /// window.get_focused()?.toggle_maximized();
    /// ```
    pub fn toggle_maximized(&self) {
        let mut client = self.window_client.clone();
        block_on_tokio(client.set_maximized(SetMaximizedRequest {
            window_id: Some(self.id),
            set_or_toggle: Some(SetOrToggle::Toggle as i32),
        }))
        .unwrap();
    }

    /// Set this window to floating or not.
    ///
    /// Floating windows will not be tiled and can be moved around and resized freely.
    ///
    /// Note that fullscreen and maximized windows can still be floating; those two states will
    /// just override the floating state.
    ///
    /// # Examples
    ///
    /// ```
    /// // Set the focused window to floating.
    /// window.get_focused()?.set_floating(true);
    /// ```
    pub fn set_floating(&self, set: bool) {
        let mut client = self.window_client.clone();
        block_on_tokio(client.set_floating(SetFloatingRequest {
            window_id: Some(self.id),
            set_or_toggle: Some(match set {
                true => SetOrToggle::Set,
                false => SetOrToggle::Unset,
            } as i32),
        }))
        .unwrap();
    }

    /// Toggle this window to and from floating.
    ///
    /// Floating windows will not be tiled and can be moved around and resized freely.
    ///
    /// Note that fullscreen and maximized windows can still be floating; those two states will
    /// just override the floating state.
    ///
    /// # Examples
    ///
    /// ```
    /// // Toggle the focused window to and from floating.
    /// window.get_focused()?.toggle_floating();
    /// ```
    pub fn toggle_floating(&self) {
        let mut client = self.window_client.clone();
        block_on_tokio(client.set_floating(SetFloatingRequest {
            window_id: Some(self.id),
            set_or_toggle: Some(SetOrToggle::Toggle as i32),
        }))
        .unwrap();
    }

    /// Focus or unfocus this window.
    ///
    /// # Examples
    ///
    /// ```
    /// // Unfocus the focused window
    /// window.get_focused()?.set_focused(false);
    /// ```
    pub fn set_focused(&self, set: bool) {
        let mut client = self.window_client.clone();
        block_on_tokio(client.set_focused(SetFocusedRequest {
            window_id: Some(self.id),
            set_or_toggle: Some(match set {
                true => SetOrToggle::Set,
                false => SetOrToggle::Unset,
            } as i32),
        }))
        .unwrap();
    }

    /// Toggle this window to and from focused.
    ///
    /// # Examples
    ///
    /// ```
    /// // Toggle the focused window to and from floating.
    /// // Calling this a second time will do nothing because there won't
    /// // be a focused window.
    /// window.get_focused()?.toggle_focused();
    /// ```
    pub fn toggle_focused(&self) {
        let mut client = self.window_client.clone();
        block_on_tokio(client.set_focused(SetFocusedRequest {
            window_id: Some(self.id),
            set_or_toggle: Some(SetOrToggle::Toggle as i32),
        }))
        .unwrap();
    }

    /// Move this window to the given `tag`.
    ///
    /// This will remove all tags from this window then tag it with `tag`, essentially moving the
    /// window to that tag.
    ///
    /// # Examples
    ///
    /// ```
    /// // Move the focused window to tag "Code" on the focused output
    /// window.get_focused()?.move_to_tag(&tag.get("Code", None)?);
    /// ```
    pub fn move_to_tag(&self, tag: &TagHandle) {
        let mut client = self.window_client.clone();

        block_on_tokio(client.move_to_tag(MoveToTagRequest {
            window_id: Some(self.id),
            tag_id: Some(tag.id),
        }))
        .unwrap();
    }

    /// Set or unset a tag on this window.
    ///
    /// # Examples
    ///
    /// ```
    /// let focused = window.get_focused()?;
    /// let tg = tag.get("Potato", None)?;
    ///
    /// focused.set_tag(&tg, true); // `focused` now has tag "Potato"
    /// focused.set_tag(&tg, false); // `focused` no longer has tag "Potato"
    /// ```
    pub fn set_tag(&self, tag: &TagHandle, set: bool) {
        let mut client = self.window_client.clone();

        block_on_tokio(client.set_tag(SetTagRequest {
            window_id: Some(self.id),
            tag_id: Some(tag.id),
            set_or_toggle: Some(match set {
                true => SetOrToggle::Set,
                false => SetOrToggle::Unset,
            } as i32),
        }))
        .unwrap();
    }

    /// Toggle a tag on this window.
    ///
    /// # Examples
    ///
    /// ```
    /// let focused = window.get_focused()?;
    /// let tg = tag.get("Potato", None)?;
    ///
    /// // Assume `focused` does not have tag `tg`
    ///
    /// focused.toggle_tag(&tg); // `focused` now has tag "Potato"
    /// focused.toggle_tag(&tg); // `focused` no longer has tag "Potato"
    /// ```
    pub fn toggle_tag(&self, tag: &TagHandle) {
        let mut client = self.window_client.clone();

        block_on_tokio(client.set_tag(SetTagRequest {
            window_id: Some(self.id),
            tag_id: Some(tag.id),
            set_or_toggle: Some(SetOrToggle::Toggle as i32),
        }))
        .unwrap();
    }

    /// Raise this window.
    ///
    /// This will raise this window all the way to the top of the z-stack.
    ///
    /// # Examples
    ///
    /// ```
    /// window.get_focused()?.raise();
    /// ```
    pub fn raise(&self) {
        let mut client = self.window_client.clone();

        block_on_tokio(client.raise(RaiseRequest {
            window_id: Some(self.id),
        }))
        .unwrap();
    }

    /// Get all properties of this window.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::window::WindowProperties;
    ///
    /// let WindowProperties {
    ///     geometry,
    ///     class,
    ///     title,
    ///     focused,
    ///     floating,
    ///     fullscreen_or_maximized,
    ///     tags,
    /// } = window.get_focused()?.props();
    /// ```
    pub fn props(&self) -> WindowProperties {
        block_on_tokio(self.props_async())
    }

    /// The async version of [`props`][Self::props].
    pub async fn props_async(&self) -> WindowProperties {
        let mut client = self.window_client.clone();

        let response = match client
            .get_properties(window::v0alpha1::GetPropertiesRequest {
                window_id: Some(self.id),
            })
            .await
        {
            Ok(response) => response.into_inner(),
            Err(status) => {
                eprintln!("ERROR: {status}");
                return WindowProperties::default();
            }
        };

        let state = match response.state() {
            window::v0alpha1::WindowState::Unspecified => None,
            window::v0alpha1::WindowState::Tiled => Some(WindowState::Tiled),
            window::v0alpha1::WindowState::Floating => Some(WindowState::Floating),
            window::v0alpha1::WindowState::Fullscreen => Some(WindowState::Fullscreen),
            window::v0alpha1::WindowState::Maximized => Some(WindowState::Maximized),
        };

        #[allow(deprecated)]
        let fullscreen_or_maximized = response
            .fullscreen_or_maximized
            .unwrap_or_default()
            .try_into()
            .ok();

        let geometry = response.geometry.map(|geo| Geometry {
            x: geo.x(),
            y: geo.y(),
            width: geo.width() as u32,
            height: geo.height() as u32,
        });

        #[allow(deprecated)]
        WindowProperties {
            geometry,
            class: response.class,
            title: response.title,
            focused: response.focused,
            floating: response.floating,
            fullscreen_or_maximized,
            tags: response
                .tag_ids
                .into_iter()
                .map(|id| self.api.tag.new_handle(id))
                .collect(),
            state,
        }
    }

    /// Get this window's location and size.
    ///
    /// Shorthand for `self.props().geometry`.
    pub fn geometry(&self) -> Option<Geometry> {
        self.props().geometry
    }

    /// The async version of [`geometry`][Self::geometry].
    pub async fn geometry_async(&self) -> Option<Geometry> {
        self.props_async().await.geometry
    }

    /// Get this window's class.
    ///
    /// Shorthand for `self.props().class`.
    pub fn class(&self) -> Option<String> {
        self.props().class
    }

    /// The async version of [`class`][Self::class].
    pub async fn class_async(&self) -> Option<String> {
        self.props_async().await.class
    }

    /// Get this window's title.
    ///
    /// Shorthand for `self.props().title`.
    pub fn title(&self) -> Option<String> {
        self.props().title
    }

    /// The async version of [`title`][Self::title].
    pub async fn title_async(&self) -> Option<String> {
        self.props_async().await.title
    }

    /// Get whether or not this window is focused.
    ///
    /// Shorthand for `self.props().focused`.
    pub fn focused(&self) -> Option<bool> {
        self.props().focused
    }

    /// The async version of [`focused`][Self::focused].
    pub async fn focused_async(&self) -> Option<bool> {
        self.props_async().await.focused
    }

    /// Get whether or not this window is floating.
    ///
    /// Shorthand for `self.props().floating`.
    pub fn floating(&self) -> Option<bool> {
        self.props()
            .state
            .map(|state| state == WindowState::Floating)
    }

    /// The async version of [`floating`][Self::floating]
    pub async fn floating_async(&self) -> Option<bool> {
        self.props_async()
            .await
            .state
            .map(|state| state == WindowState::Floating)
    }

    /// Get whether this window is fullscreen, maximized, or neither.
    ///
    /// Shorthand for `self.props().fullscreen_or_maximized`.
    #[deprecated = "use the `fullscreen` or `maximized` methods instead"]
    pub fn fullscreen_or_maximized(&self) -> Option<FullscreenOrMaximized> {
        self.props().state.map(|state| match state {
            WindowState::Tiled | WindowState::Floating => FullscreenOrMaximized::Neither,
            WindowState::Fullscreen => FullscreenOrMaximized::Fullscreen,
            WindowState::Maximized => FullscreenOrMaximized::Maximized,
        })
    }

    /// The async version of [`fullscreen_or_maximized`][Self::fullscreen_or_maximized].
    #[deprecated = "use the `fullscreen_async` or `maximized_async` methods instead"]
    pub async fn fullscreen_or_maximized_async(&self) -> Option<FullscreenOrMaximized> {
        self.props_async().await.state.map(|state| match state {
            WindowState::Tiled | WindowState::Floating => FullscreenOrMaximized::Neither,
            WindowState::Fullscreen => FullscreenOrMaximized::Fullscreen,
            WindowState::Maximized => FullscreenOrMaximized::Maximized,
        })
    }

    /// Get whether or not this window is tiled.
    pub fn tiled(&self) -> Option<bool> {
        self.props().state.map(|state| state == WindowState::Tiled)
    }

    /// The async version of [`tiled`][Self::tiled].
    pub async fn tiled_async(&self) -> Option<bool> {
        self.props_async()
            .await
            .state
            .map(|state| state == WindowState::Tiled)
    }

    /// Get whether or not this window is fullscreen.
    pub fn fullscreen(&self) -> Option<bool> {
        self.props()
            .state
            .map(|state| state == WindowState::Fullscreen)
    }

    /// The async version of [`fullscreen`][Self::fullscreen].
    pub async fn fullscreen_async(&self) -> Option<bool> {
        self.props_async()
            .await
            .state
            .map(|state| state == WindowState::Fullscreen)
    }

    /// Get whether or not this window is maximized.
    pub fn maximized(&self) -> Option<bool> {
        self.props()
            .state
            .map(|state| state == WindowState::Maximized)
    }

    /// The async version of [`maximized`][Self::maximized].
    pub async fn maximized_async(&self) -> Option<bool> {
        self.props_async()
            .await
            .state
            .map(|state| state == WindowState::Maximized)
    }

    /// Get all the tags on this window.
    ///
    /// Shorthand for `self.props().tags`.
    pub fn tags(&self) -> Vec<TagHandle> {
        self.props().tags
    }

    /// The async version of [`tags`][Self::tags].
    pub async fn tags_async(&self) -> Vec<TagHandle> {
        self.props_async().await.tags
    }

    /// Returns whether this window is on an active tag.
    pub fn is_on_active_tag(&self) -> bool {
        self.tags()
            .batch_find(
                |tag| tag.active_async().boxed(),
                |active| active.unwrap_or_default(),
            )
            .is_some()
    }

    /// The async version of [`WindowHandle::is_on_active_tag`].
    pub async fn is_on_active_tag_async(&self) -> bool {
        let tags = self.tags_async().await;
        crate::util::batch_async(tags.iter().map(|tag| tag.active_async()))
            .await
            .contains(&Some(true))
    }

    /// Get this window's raw compositor id.
    pub fn id(&self) -> u32 {
        self.id
    }
}
