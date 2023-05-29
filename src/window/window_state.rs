use smithay::utils::{Logical, Point, Size};

use super::SurfaceState;

pub struct WindowState {
    pub floating: Float,
}

pub enum Float {
    NotFloating(Option<(Point<i32, Logical>, Size<i32, Logical>)>),
    /// An [Option] of a tuple of the previous location and previous size of the window
    Floating,
}

impl Default for WindowState {
    fn default() -> Self {
        Self::new() // TODO: maybe actual defaults
    }
}

impl WindowState {
    pub fn new() -> Self {
        Self {
            floating: Float::NotFloating(None), // TODO: get this from a config file instead of
                                                // |     hardcoding
        }
    }
}

impl SurfaceState for WindowState {}
