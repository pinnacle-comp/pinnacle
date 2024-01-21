//! Window management.
//!
//! This module provides [`Window`], which allows you to get [`WindowHandle`]s and move and resize
//! windows using the mouse.
//!
//! [`WindowHandle`]s allow you to do things like resize and move windows, toggle them between
//! floating and tiled, close them, and more.

use futures::executor::block_on;
use num_enum::TryFromPrimitive;
use pinnacle_api_defs::pinnacle::{
    output::v0alpha1::output_service_client::OutputServiceClient,
    tag::v0alpha1::tag_service_client::TagServiceClient,
    window::v0alpha1::{
        window_service_client::WindowServiceClient, CloseRequest, MoveToTagRequest, SetTagRequest,
    },
    window::{
        self,
        v0alpha1::{
            GetRequest, MoveGrabRequest, ResizeGrabRequest, SetFloatingRequest,
            SetFullscreenRequest, SetMaximizedRequest,
        },
    },
};
use tonic::transport::Channel;

use crate::{input::MouseButton, tag::TagHandle, util::Geometry};

/// A struct containing methods that get [`WindowHandle`]s and move windows with the mouse.
///
/// See [`WindowHandle`] for more information.
#[derive(Debug, Clone)]
pub struct Window {
    channel: Channel,
}

impl Window {
    pub(crate) fn new(channel: Channel) -> Self {
        Self { channel }
    }

    fn create_window_client(&self) -> WindowServiceClient<Channel> {
        WindowServiceClient::new(self.channel.clone())
    }

    fn create_tag_client(&self) -> TagServiceClient<Channel> {
        TagServiceClient::new(self.channel.clone())
    }

    fn create_output_client(&self) -> OutputServiceClient<Channel> {
        OutputServiceClient::new(self.channel.clone())
    }

    /// Start moving the window with the mouse.
    ///
    /// This will begin moving the window under the pointer using the specified [`MouseButton`].
    /// The button must be held down at the time this method is called for the move to start.
    ///
    /// This is intended to be used with [`Input::keybind`][pinnacle_api::input::Keybind].
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
        let mut client = self.create_window_client();
        block_on(client.move_grab(MoveGrabRequest {
            button: Some(button as u32),
        }))
        .unwrap();
    }

    /// Start resizing the window with the mouse.
    ///
    /// This will begin resizing the window under the pointer using the specified [`MouseButton`].
    /// The button must be held down at the time this method is called for the resize to start.
    ///
    /// This is intended to be used with [`Input::keybind`][pinnacle_api::input::Keybind].
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
        let mut client = self.create_window_client();
        block_on(client.resize_grab(ResizeGrabRequest {
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
    pub fn get_all(&self) -> impl Iterator<Item = WindowHandle> {
        let mut client = self.create_window_client();
        let tag_client = self.create_tag_client();
        let output_client = self.create_output_client();
        block_on(client.get(GetRequest {}))
            .unwrap()
            .into_inner()
            .window_ids
            .into_iter()
            .map(move |id| WindowHandle {
                client: client.clone(),
                id,
                tag_client: tag_client.clone(),
                output_client: output_client.clone(),
            })
    }

    /// Get the currently focused window.
    ///
    /// # Examples
    ///
    /// ```
    /// let focused_window = window.get_focused()?;
    /// ```
    pub fn get_focused(&self) -> Option<WindowHandle> {
        self.get_all()
            .find(|window| matches!(window.props().focused, Some(true)))
    }
}

/// A handle to a window.
///
/// This allows you to manipulate the window and get its properties.
#[derive(Debug, Clone)]
pub struct WindowHandle {
    pub(crate) client: WindowServiceClient<Channel>,
    pub(crate) id: u32,
    pub(crate) tag_client: TagServiceClient<Channel>,
    pub(crate) output_client: OutputServiceClient<Channel>,
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

/// Properties of a window.
#[derive(Debug, Clone)]
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
    /// state will just override the floating state.
    pub floating: Option<bool>,
    /// Whether the window is fullscreen, maximized, or neither
    pub fullscreen_or_maximized: Option<FullscreenOrMaximized>,
    /// All the tags on the window
    pub tags: Vec<TagHandle>,
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
    pub fn close(mut self) {
        block_on(self.client.close(CloseRequest {
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
        let mut client = self.client.clone();
        block_on(client.set_fullscreen(SetFullscreenRequest {
            window_id: Some(self.id),
            set_or_toggle: Some(window::v0alpha1::set_fullscreen_request::SetOrToggle::Set(
                set,
            )),
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
        let mut client = self.client.clone();
        block_on(client.set_fullscreen(SetFullscreenRequest {
            window_id: Some(self.id),
            set_or_toggle: Some(window::v0alpha1::set_fullscreen_request::SetOrToggle::Toggle(())),
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
        let mut client = self.client.clone();
        block_on(client.set_maximized(SetMaximizedRequest {
            window_id: Some(self.id),
            set_or_toggle: Some(window::v0alpha1::set_maximized_request::SetOrToggle::Set(
                set,
            )),
        }))
        .unwrap();
    }

    /// Toggle this window between maximized and not.
    ///
    /// If it is fullscreen, setting it to maximized will remove the fullscreen state.
    ///
    /// # Examples
    ///
    /// ```
    /// // Toggle the focused window to and from maximized.
    /// window.get_focused()?.toggle_maximized();
    /// ```
    pub fn toggle_maximized(&self) {
        let mut client = self.client.clone();
        block_on(client.set_maximized(SetMaximizedRequest {
            window_id: Some(self.id),
            set_or_toggle: Some(window::v0alpha1::set_maximized_request::SetOrToggle::Toggle(())),
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
        let mut client = self.client.clone();
        block_on(client.set_floating(SetFloatingRequest {
            window_id: Some(self.id),
            set_or_toggle: Some(window::v0alpha1::set_floating_request::SetOrToggle::Set(
                set,
            )),
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
        let mut client = self.client.clone();
        block_on(client.set_floating(SetFloatingRequest {
            window_id: Some(self.id),
            set_or_toggle: Some(window::v0alpha1::set_floating_request::SetOrToggle::Toggle(
                (),
            )),
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
        let mut client = self.client.clone();

        block_on(client.move_to_tag(MoveToTagRequest {
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
        let mut client = self.client.clone();

        block_on(client.set_tag(SetTagRequest {
            window_id: Some(self.id),
            tag_id: Some(tag.id),
            set_or_toggle: Some(window::v0alpha1::set_tag_request::SetOrToggle::Set(set)),
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
        let mut client = self.client.clone();

        block_on(client.set_tag(SetTagRequest {
            window_id: Some(self.id),
            tag_id: Some(tag.id),
            set_or_toggle: Some(window::v0alpha1::set_tag_request::SetOrToggle::Toggle(())),
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
        let mut client = self.client.clone();
        let tag_client = self.tag_client.clone();
        let response = block_on(
            client.get_properties(window::v0alpha1::GetPropertiesRequest {
                window_id: Some(self.id),
            }),
        )
        .unwrap()
        .into_inner();

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
                .map(|id| TagHandle {
                    client: tag_client.clone(),
                    output_client: self.output_client.clone(),
                    id,
                })
                .collect(),
        }
    }

    /// Get this window's location and size.
    ///
    /// Shorthand for `self.props().geometry`.
    pub fn geometry(&self) -> Option<Geometry> {
        self.props().geometry
    }

    /// Get this window's class.
    ///
    /// Shorthand for `self.props().class`.
    pub fn class(&self) -> Option<String> {
        self.props().class
    }

    /// Get this window's title.
    ///
    /// Shorthand for `self.props().title`.
    pub fn title(&self) -> Option<String> {
        self.props().title
    }

    /// Get whether or not this window is focused.
    ///
    /// Shorthand for `self.props().focused`.
    pub fn focused(&self) -> Option<bool> {
        self.props().focused
    }

    /// Get whether or not this window is floating.
    ///
    /// Shorthand for `self.props().floating`.
    pub fn floating(&self) -> Option<bool> {
        self.props().floating
    }

    /// Get whether this window is fullscreen, maximized, or neither.
    ///
    /// Shorthand for `self.props().fullscreen_or_maximized`.
    pub fn fullscreen_or_maximized(&self) -> Option<FullscreenOrMaximized> {
        self.props().fullscreen_or_maximized
    }

    /// Get all the tags on this window.
    ///
    /// Shorthand for `self.props().tags`.
    pub fn tags(&self) -> Vec<TagHandle> {
        self.props().tags
    }
}
