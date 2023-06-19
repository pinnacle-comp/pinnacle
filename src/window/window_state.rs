use std::{borrow::BorrowMut, cell::RefCell};

use smithay::{
    desktop::Window,
    utils::{Logical, Point, Size},
};

pub struct WindowState {
    pub floating: Float,
}

pub enum Float {
    /// An [Option] of a tuple of the previous location and previous size of the window
    Tiled(Option<(Point<i32, Logical>, Size<i32, Logical>)>),
    Floating,
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
        }
    }
}
