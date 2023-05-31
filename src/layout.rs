use std::{error::Error, fmt::Display};

use smithay::desktop::Window;

use crate::State;

pub mod automatic;
pub mod manual;

pub trait Layout {
    fn layout_windows(&self, state: &mut State, windows: Vec<Window>);
    fn add_window(&mut self, state: &mut State, window: Window);
    fn remove_window(&mut self, state: &mut State, window: Window)
        -> Result<(), RemoveWindowError>;
}

#[derive(Debug)]
pub enum RemoveWindowError {
    NotFound,
}

impl Display for RemoveWindowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for RemoveWindowError {}
