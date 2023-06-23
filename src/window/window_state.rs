use std::cell::RefCell;

use smithay::{
    desktop::Window,
    utils::{Logical, Point, Serial, Size},
};

pub struct WindowState {
    pub floating: Float,
    pub resize_state: WindowResizeState,
}

/// The state of a window's resize operation.
///
/// A naive implementation of window swapping would probably immediately call
/// [`space.map_element()`] right after setting its size through [`with_pending_state()`] and
/// sending a configure event. However, the client will probably not acknowledge the configure
/// until *after* the window has moved, causing flickering.
///
/// To solve this, we need to create two additional steps: [`WaitingForAck`] and [`WaitingForCommit`].
/// If we need to change a window's location when we change its size, instead of
/// calling `map_element()`, we change the window's [`WindowState`] and set
/// its [`resize_state`] to `WaitingForAck` with the new position we want.
///
/// When the client acks the configure, we can move the state to `WaitingForCommit` in
/// [`XdgShellHandler.ack_configure()`]. Finally, in [`CompositorHandler.commit()`], we set the
/// state back to [`Idle`] and map the window.
///
/// [`space.map_element()`]: smithay::desktop::space::Space#method.map_element
/// [`with_pending_state()`]: smithay::wayland::shell::xdg::ToplevelSurface#method.with_pending_state
/// [`Idle`]: WindowResizeState::Idle
/// [`WaitingForAck`]: WindowResizeState::WaitingForAck
/// [`WaitingForCommit`]: WindowResizeState::WaitingForCommit
/// [`resize_state`]: WindowState#structfield.resize_state
/// [`XdgShellHandler.ack_configure()`]: smithay::wayland::shell::xdg::XdgShellHandler#method.ack_configure
/// [`CompositorHandler.commit()`]: smithay::wayland::compositor::CompositorHandler#tymethod.commit
#[derive(Debug, Default)]
pub enum WindowResizeState {
    /// The window doesn't need to be moved.
    #[default]
    Idle,
    /// The window has received a configure request with a new size. The desired location and the
    /// configure request's serial should be provided here.
    WaitingForAck(Serial, Point<i32, Logical>),
    /// The client has received the configure request and has successfully changed its size. It's
    /// now safe to move the window in [`CompositorHandler.commit()`] without flickering.
    ///
    /// [`CompositorHandler.commit()`]: smithay::wayland::compositor::CompositorHandler#tymethod.commit
    WaitingForCommit(Point<i32, Logical>),
}

pub enum Float {
    /// An [Option] of a tuple of the previous location and previous size of the window
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
    pub fn new() -> Self {
        Default::default()
    }

    /// Access a [Window]'s state
    pub fn with_state<F, T>(window: &Window, mut func: F) -> T
    where
        F: FnMut(&mut Self) -> T,
    {
        window
            .user_data()
            .insert_if_missing(RefCell::<Self>::default);

        let mut state = window
            .user_data()
            .get::<RefCell<Self>>()
            .unwrap()
            .borrow_mut();
        func(&mut state)
    }
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            // TODO: get this from a config file instead of hardcoding
            floating: Float::Tiled(None),
            resize_state: Default::default(),
        }
    }
}
