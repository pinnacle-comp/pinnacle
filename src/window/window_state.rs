// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    cell::RefCell,
    sync::atomic::{AtomicU32, Ordering},
};

use smithay::{
    desktop::Window,
    utils::{Logical, Point, Rectangle, Serial},
};

use crate::{
    backend::Backend,
    state::{State, WithState},
    tag::Tag,
};

use super::WindowElement;

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WindowId(u32);

static WINDOW_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

impl WindowId {
    pub fn next() -> Self {
        Self(WINDOW_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the window that has this WindowId.
    pub fn window<B: Backend>(&self, state: &State<B>) -> Option<WindowElement> {
        state
            .windows
            .iter()
            .find(|win| win.with_state(|state| &state.id == self))
            .cloned()
    }
}

#[derive(Debug, Default)]
pub struct WindowState {
    pub minimized: bool,
}

impl WithState for Window {
    type State = WindowState;

    fn with_state<F, T>(&self, mut func: F) -> T
    where
        F: FnMut(&mut Self::State) -> T,
    {
        self.user_data()
            .insert_if_missing(RefCell::<Self::State>::default);

        let state = self
            .user_data()
            .get::<RefCell<Self::State>>()
            .expect("RefCell not in data map");

        func(&mut state.borrow_mut())
    }
}

#[derive(Debug)]
pub struct WindowElementState {
    /// The id of this window.
    pub id: WindowId,
    /// Whether the window is floating, tiled, fullscreen, or maximized.
    pub status: Status,
    /// The window's resize state. See [WindowResizeState] for more.
    pub loc_request_state: LocationRequestState,
    /// What tags the window is currently on.
    pub tags: Vec<Tag>,
}

/// The state of a window's resize operation.
///
/// A naive implementation of window swapping would probably immediately call
/// [`space.map_element()`] right after setting its size through [`with_pending_state()`] and
/// sending a configure event. However, the client will probably not acknowledge the configure
/// until *after* the window has moved, causing flickering.
///
/// To solve this, we need to create two additional steps: [`Requested`] and [`Acknowledged`].
/// If we need to change a window's location when we change its size, instead of
/// calling `map_element()`, we change the window's [`WindowState`] and set
/// its [`resize_state`] to `Requested` with the new position we want.
///
/// When the client acks the configure, we can move the state to `Acknowledged` in
/// [`XdgShellHandler.ack_configure()`]. Finally, in [`CompositorHandler.commit()`], we set the
/// state back to [`Idle`] and map the window.
///
/// [`space.map_element()`]: smithay::desktop::space::Space#method.map_element
/// [`with_pending_state()`]: smithay::wayland::shell::xdg::ToplevelSurface#method.with_pending_state
/// [`Idle`]: WindowResizeState::Idle
/// [`Requested`]: WindowResizeState::Requested
/// [`Acknowledged`]: WindowResizeState::Acknowledged
/// [`resize_state`]: WindowState#structfield.resize_state
/// [`XdgShellHandler.ack_configure()`]: smithay::wayland::shell::xdg::XdgShellHandler#method.ack_configure
/// [`CompositorHandler.commit()`]: smithay::wayland::compositor::CompositorHandler#tymethod.commit
#[derive(Debug, Default, Clone)]
pub enum LocationRequestState {
    /// The window doesn't need to be moved.
    #[default]
    Idle,
    Sent(Point<i32, Logical>),
    /// The window has received a configure request with a new size. The desired location and the
    /// configure request's serial should be provided here.
    Requested(Serial, Point<i32, Logical>),
    /// The client has received the configure request and has successfully changed its size. It's
    /// now safe to move the window in [`CompositorHandler.commit()`] without flickering.
    ///
    /// [`CompositorHandler.commit()`]: smithay::wayland::compositor::CompositorHandler#tymethod.commit
    Acknowledged(Point<i32, Logical>),
}

/// Whether the window is floating, tiled, fullscreen, or maximized.
#[derive(Debug, Clone, Copy)]
pub enum Status {
    Floating(Rectangle<i32, Logical>),
    Tiled(Option<Rectangle<i32, Logical>>),
    Fullscreen(Option<Rectangle<i32, Logical>>),
    Maximized(Option<Rectangle<i32, Logical>>),
}
// TODO: couple these somehow

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum StatusName {
    Floating,
    Tiled,
    Fullscreen,
    Maximized,
}

impl Status {
    /// Returns `true` if the float is [`Tiled`].
    ///
    /// [`Tiled`]: Float::Tiled
    #[must_use]
    pub fn is_tiled(&self) -> bool {
        matches!(self, Self::Tiled(..))
    }

    /// Returns `true` if the float is [`Floating`].
    ///
    /// [`Floating`]: Float::Floating
    #[must_use]
    pub fn is_floating(&self) -> bool {
        matches!(self, Self::Floating(_))
    }

    /// Returns `true` if the status is [`Fullscreen`].
    ///
    /// [`Fullscreen`]: Status::Fullscreen
    #[must_use]
    pub fn is_fullscreen(&self) -> bool {
        matches!(self, Self::Fullscreen(..))
    }

    /// Returns `true` if the status is [`Maximized`].
    ///
    /// [`Maximized`]: Status::Maximized
    #[must_use]
    pub fn is_maximized(&self) -> bool {
        matches!(self, Self::Maximized(..))
    }
}

impl WindowElementState {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for WindowElementState {
    fn default() -> Self {
        Self {
            // INFO: I think this will assign the id on use of the state, not on window spawn.
            id: WindowId::next(),
            status: Status::Tiled(None),
            loc_request_state: LocationRequestState::Idle,
            tags: vec![],
        }
    }
}
