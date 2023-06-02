use std::{error::Error, fmt::Display};

use smithay::desktop::Window;

use crate::{backend::Backend, state::State};

pub mod automatic;
pub mod manual;

pub trait Layout<B: Backend> {
    fn layout_windows(&self, state: &mut State<B>, windows: Vec<Window>);
    fn add_window(&mut self, state: &mut State<B>, window: Window);
    fn remove_window(
        &mut self,
        state: &mut State<B>,
        window: Window,
    ) -> Result<(), RemoveWindowError>;
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
