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

use futures::FutureExt;
use pinnacle_api_defs::pinnacle::{
    util::v1::SetOrToggle,
    window::{
        self,
        v1::{
            GetAppIdRequest, GetFocusedRequest, GetLayoutModeRequest, GetLocRequest,
            GetSizeRequest, GetTagIdsRequest, GetTitleRequest, MoveGrabRequest, MoveToTagRequest,
            RaiseRequest, ResizeGrabRequest, SetDecorationModeRequest, SetFloatingRequest,
            SetFocusedRequest, SetFullscreenRequest, SetMaximizedRequest, SetTagRequest,
        },
    },
};
use tokio::sync::mpsc::unbounded_channel;
use tokio_stream::StreamExt;

use crate::{
    client::Client,
    input::MouseButton,
    signal::{SignalHandle, WindowSignal},
    tag::TagHandle,
    util::{Batch, Point, Size},
    BlockOnTokio,
};

pub fn get_all() -> impl Iterator<Item = WindowHandle> {
    get_all_async().block_on_tokio()
}

pub async fn get_all_async() -> impl Iterator<Item = WindowHandle> {
    let window_ids = Client::window()
        .get(pinnacle_api_defs::pinnacle::window::v1::GetRequest {})
        .await
        .unwrap()
        .into_inner()
        .window_ids;

    window_ids.into_iter().map(|id| WindowHandle { id })
}

pub fn get_focused() -> Option<WindowHandle> {
    get_focused_async().block_on_tokio()
}

pub async fn get_focused_async() -> Option<WindowHandle> {
    let windows = get_all_async().await;

    windows.batch_find(|win| win.focused_async().boxed(), |focused| *focused)
}

pub fn begin_move(button: MouseButton) {
    Client::window()
        .move_grab(MoveGrabRequest {
            button: button.into(),
        })
        .block_on_tokio()
        .unwrap();
}

pub fn begin_resize(button: MouseButton) {
    Client::window()
        .resize_grab(ResizeGrabRequest {
            button: button.into(),
        })
        .block_on_tokio()
        .unwrap();
}

/// Connect to a window signal.
///
/// The compositor will fire off signals that your config can listen for and act upon.
/// You can pass in a [`WindowSignal`] along with a callback and it will get run
/// with the necessary arguments every time a signal of that type is received.
pub fn connect_signal(signal: WindowSignal) -> SignalHandle {
    let mut signal_state = Client::signal_state();

    match signal {
        WindowSignal::PointerEnter(f) => signal_state.window_pointer_enter.add_callback(f),
        WindowSignal::PointerLeave(f) => signal_state.window_pointer_leave.add_callback(f),
    }
}

/// A handle to a window.
///
/// This allows you to manipulate the window and get its properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowHandle {
    pub(crate) id: u32,
}

/// A window's current layout mode.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum LayoutMode {
    /// The window is tiled.
    Tiled,
    /// The window is floating.
    Floating,
    /// The window is fullscreen.
    Fullscreen,
    /// The window is maximized.
    Maximized,
}

impl TryFrom<pinnacle_api_defs::pinnacle::window::v1::LayoutMode> for LayoutMode {
    type Error = ();

    fn try_from(
        value: pinnacle_api_defs::pinnacle::window::v1::LayoutMode,
    ) -> Result<Self, Self::Error> {
        match value {
            window::v1::LayoutMode::Unspecified => Err(()),
            window::v1::LayoutMode::Tiled => Ok(LayoutMode::Tiled),
            window::v1::LayoutMode::Floating => Ok(LayoutMode::Floating),
            window::v1::LayoutMode::Fullscreen => Ok(LayoutMode::Fullscreen),
            window::v1::LayoutMode::Maximized => Ok(LayoutMode::Maximized),
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum DecorationMode {
    ClientSide,
    ServerSide,
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
        let window_id = self.id;
        Client::window()
            .close(pinnacle_api_defs::pinnacle::window::v1::CloseRequest { window_id })
            .block_on_tokio()
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
        let window_id = self.id;
        Client::window()
            .set_fullscreen(SetFullscreenRequest {
                window_id,
                set_or_toggle: match set {
                    true => SetOrToggle::Set,
                    false => SetOrToggle::Unset,
                }
                .into(),
            })
            .block_on_tokio()
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
        let window_id = self.id;
        Client::window()
            .set_fullscreen(SetFullscreenRequest {
                window_id,
                set_or_toggle: SetOrToggle::Toggle.into(),
            })
            .block_on_tokio()
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
        let window_id = self.id;
        Client::window()
            .set_maximized(SetMaximizedRequest {
                window_id,
                set_or_toggle: match set {
                    true => SetOrToggle::Set,
                    false => SetOrToggle::Unset,
                }
                .into(),
            })
            .block_on_tokio()
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
        let window_id = self.id;
        Client::window()
            .set_maximized(SetMaximizedRequest {
                window_id,
                set_or_toggle: SetOrToggle::Toggle.into(),
            })
            .block_on_tokio()
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
        let window_id = self.id;
        Client::window()
            .set_floating(SetFloatingRequest {
                window_id,
                set_or_toggle: match set {
                    true => SetOrToggle::Set,
                    false => SetOrToggle::Unset,
                }
                .into(),
            })
            .block_on_tokio()
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
        let window_id = self.id;
        Client::window()
            .set_floating(SetFloatingRequest {
                window_id,
                set_or_toggle: SetOrToggle::Toggle.into(),
            })
            .block_on_tokio()
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
        let window_id = self.id;
        Client::window()
            .set_focused(SetFocusedRequest {
                window_id,
                set_or_toggle: match set {
                    true => SetOrToggle::Set,
                    false => SetOrToggle::Unset,
                }
                .into(),
            })
            .block_on_tokio()
            .unwrap();
    }

    pub fn set_decoration_mode(&self, mode: DecorationMode) {
        Client::window()
            .set_decoration_mode(SetDecorationModeRequest {
                window_id: self.id,
                decoration_mode: match mode {
                    DecorationMode::ClientSide => window::v1::DecorationMode::ClientSide,
                    DecorationMode::ServerSide => window::v1::DecorationMode::ServerSide,
                }
                .into(),
            })
            .block_on_tokio()
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
        let window_id = self.id;
        Client::window()
            .set_focused(SetFocusedRequest {
                window_id,
                set_or_toggle: SetOrToggle::Toggle.into(),
            })
            .block_on_tokio()
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
        let window_id = self.id;
        let tag_id = tag.id;
        Client::window()
            .move_to_tag(MoveToTagRequest { window_id, tag_id })
            .block_on_tokio()
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
        let window_id = self.id;
        let tag_id = tag.id;
        Client::window()
            .set_tag(SetTagRequest {
                window_id,
                tag_id,
                set_or_toggle: match set {
                    true => SetOrToggle::Set,
                    false => SetOrToggle::Unset,
                }
                .into(),
            })
            .block_on_tokio()
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
        let window_id = self.id;
        let tag_id = tag.id;
        Client::window()
            .set_tag(SetTagRequest {
                window_id,
                tag_id,
                set_or_toggle: SetOrToggle::Toggle.into(),
            })
            .block_on_tokio()
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
        let window_id = self.id;
        Client::window()
            .raise(RaiseRequest { window_id })
            .block_on_tokio()
            .unwrap();
    }

    pub fn loc(&self) -> Option<Point> {
        self.loc_async().block_on_tokio()
    }

    /// Get this window's location.
    pub async fn loc_async(&self) -> Option<Point> {
        let window_id = self.id;
        Client::window()
            .get_loc(GetLocRequest { window_id })
            .await
            .unwrap()
            .into_inner()
            .loc
            .map(|loc| Point { x: loc.x, y: loc.y })
    }

    pub fn size(&self) -> Option<Size> {
        self.size_async().block_on_tokio()
    }

    /// Get this window's size.
    pub async fn size_async(&self) -> Option<Size> {
        let window_id = self.id;
        Client::window()
            .get_size(GetSizeRequest { window_id })
            .await
            .unwrap()
            .into_inner()
            .size
            .map(|size| Size {
                w: size.width,
                h: size.height,
            })
    }

    pub fn app_id(&self) -> String {
        self.app_id_async().block_on_tokio()
    }

    /// Get this window's app id.
    pub async fn app_id_async(&self) -> String {
        let window_id = self.id;
        Client::window()
            .get_app_id(GetAppIdRequest { window_id })
            .await
            .unwrap()
            .into_inner()
            .app_id
    }

    pub fn title(&self) -> String {
        self.title_async().block_on_tokio()
    }

    /// Get this window's title.
    pub async fn title_async(&self) -> String {
        let window_id = self.id;
        Client::window()
            .get_title(GetTitleRequest { window_id })
            .await
            .unwrap()
            .into_inner()
            .title
    }

    pub fn focused(&self) -> bool {
        self.focused_async().block_on_tokio()
    }

    /// Get whether or not this window is focused.
    pub async fn focused_async(&self) -> bool {
        let window_id = self.id;
        Client::window()
            .get_focused(GetFocusedRequest { window_id })
            .await
            .unwrap()
            .into_inner()
            .focused
    }

    pub fn layout_mode(&self) -> LayoutMode {
        self.layout_mode_async().block_on_tokio()
    }

    /// Get whether or not this window is tiled.
    pub async fn layout_mode_async(&self) -> LayoutMode {
        let window_id = self.id;
        Client::window()
            .get_layout_mode(GetLayoutModeRequest { window_id })
            .await
            .unwrap()
            .into_inner()
            .layout_mode()
            .try_into()
            .unwrap_or(LayoutMode::Tiled)
    }

    /// Get whether or not this window is floating.
    pub fn floating(&self) -> bool {
        self.floating_async().block_on_tokio()
    }

    pub async fn floating_async(&self) -> bool {
        self.layout_mode_async().await == LayoutMode::Floating
    }

    pub fn fullscreen(&self) -> bool {
        self.fullscreen_async().block_on_tokio()
    }

    pub async fn fullscreen_async(&self) -> bool {
        self.layout_mode_async().await == LayoutMode::Fullscreen
    }

    pub fn maximized(&self) -> bool {
        self.maximized_async().block_on_tokio()
    }

    pub async fn maximized_async(&self) -> bool {
        self.layout_mode_async().await == LayoutMode::Maximized
    }

    /// Get all the tags on this window.
    pub fn tags(&self) -> impl Iterator<Item = TagHandle> {
        self.tags_async().block_on_tokio()
    }

    pub async fn tags_async(&self) -> impl Iterator<Item = TagHandle> {
        let window_id = self.id;
        Client::window()
            .get_tag_ids(GetTagIdsRequest { window_id })
            .await
            .unwrap()
            .into_inner()
            .tag_ids
            .into_iter()
            .map(|id| TagHandle { id })
    }

    pub fn is_on_active_tag(&self) -> bool {
        self.is_on_active_tag_async().block_on_tokio()
    }

    /// Returns whether this window is on an active tag.
    pub async fn is_on_active_tag_async(&self) -> bool {
        self.tags_async()
            .await
            .batch_find(|tag| tag.active_async().boxed(), |active| *active)
            .is_some()
    }

    /// Get this window's raw compositor id.
    pub fn id(&self) -> u32 {
        self.id
    }
}

pub fn for_all_windows(mut for_all: impl FnMut(WindowHandle) + Send + 'static) {
    let (client_outgoing, client_outgoing_to_server) = unbounded_channel();
    let client_outgoing_to_server =
        tokio_stream::wrappers::UnboundedReceiverStream::new(client_outgoing_to_server);
    let mut client_incoming = Client::window()
        .window_rule(client_outgoing_to_server)
        .block_on_tokio()
        .unwrap()
        .into_inner();

    let fut = async move {
        while let Some(Ok(response)) = client_incoming.next().await {
            let Some(response) = response.response else {
                continue;
            };

            match response {
                window::v1::window_rule_response::Response::NewWindow(new_window_request) => {
                    let request_id = new_window_request.request_id;
                    let window_id = new_window_request.window_id;

                    for_all(WindowHandle { id: window_id });

                    let sent = client_outgoing
                        .send(window::v1::WindowRuleRequest {
                            request: Some(window::v1::window_rule_request::Request::Finished(
                                window::v1::window_rule_request::Finished { request_id },
                            )),
                        })
                        .is_ok();

                    if !sent {
                        break;
                    }
                }
            }
        }
    };

    tokio::spawn(fut);
}
