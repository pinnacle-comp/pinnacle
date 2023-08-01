// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    cell::RefCell,
    fmt,
    sync::atomic::{AtomicU32, Ordering},
};

use smithay::{
    desktop::Window,
    utils::{Logical, Point, Serial, Size},
};

use crate::{
    backend::Backend,
    state::{State, WithState},
    tag::Tag,
};

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WindowId(u32);

static WINDOW_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

impl WindowId {
    pub fn next() -> Self {
        Self(WINDOW_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the window that has this WindowId.
    pub fn window<B: Backend>(&self, state: &State<B>) -> Option<Window> {
        state
            .windows
            .iter()
            .find(|win| win.with_state(|state| &state.id == self))
            .cloned()
    }
}

pub struct WindowState {
    /// The id of this window.
    pub id: WindowId,
    /// Whether the window is floating or tiled.
    pub floating: Float,
    /// The window's resize state. See [WindowResizeState] for more.
    pub resize_state: WindowResizeState,
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
#[derive(Default, Clone)]
pub enum WindowResizeState {
    /// The window doesn't need to be moved.
    #[default]
    Idle,
    /// The window has received a configure request with a new size. The desired location and the
    /// configure request's serial should be provided here.
    Requested(Serial, Point<i32, Logical>),
    /// The client has received the configure request and has successfully changed its size. It's
    /// now safe to move the window in [`CompositorHandler.commit()`] without flickering.
    ///
    /// [`CompositorHandler.commit()`]: smithay::wayland::compositor::CompositorHandler#tymethod.commit
    Acknowledged(Point<i32, Logical>),
}

impl fmt::Debug for WindowResizeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::Requested(_arg0, _arg1) => write!(f, "Requested"),
            Self::Acknowledged(_arg0) => write!(f, "Acknowledged"),
        }
    }
}

pub enum Float {
    /// The previous location and size of the window when it was floating, if any.
    Tiled(Option<(Point<i32, Logical>, Size<i32, Logical>)>),
    Floating,
}

impl Float {
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
        matches!(self, Self::Floating)
    }
}

impl WindowState {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Default::default()
    }
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

impl Default for WindowState {
    fn default() -> Self {
        Self {
            // INFO: I think this will assign the id on use of the state, not on window spawn.
            id: WindowId::next(),
            floating: Float::Tiled(None),
            resize_state: WindowResizeState::Idle,
            tags: vec![],
        }
    }
}
