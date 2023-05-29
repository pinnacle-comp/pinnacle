use smithay::desktop::Window;

use crate::State;

pub mod automatic;
pub mod manual;

pub trait Layout {
    fn layout_windows(&self, state: &mut State, windows: Vec<Window>);
}
