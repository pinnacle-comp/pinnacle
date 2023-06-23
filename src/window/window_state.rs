use std::cell::RefCell;

use smithay::{
    desktop::Window,
    utils::{Logical, Point, Serial, Size},
};

pub struct WindowState {
    pub floating: Float,
    pub resize_state: WindowResizeState,
}

#[derive(Debug, Default)]
pub enum WindowResizeState {
    #[default]
    Idle,
    WaitingForAck(Serial, Point<i32, Logical>),
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
