use std::{error::Error, fmt::Display};

use smithay::desktop::Window;

use crate::{backend::Backend, state::State};

pub mod automatic;
pub mod manual;

pub struct Layout;

pub enum Direction {
    Left,
    Right,
    Top,
    Bottom,
}
