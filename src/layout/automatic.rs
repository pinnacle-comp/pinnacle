use smithay::desktop::Window;

use crate::State;

use super::Layout;

/// A layout which puts one "master" window on one half of the screen and splits other windows
/// among the other half.
pub struct MasterStack {
    /// Which side of the screen the master window will be on
    pub side: MasterStackSide,
}

pub enum MasterStackSide {
    Left,
    Right,
    Top,
    Bottom,
}

impl Layout for MasterStack {
    fn layout_windows(&self, state: &mut State, windows: Vec<Window>) {
        match self.side {
            MasterStackSide::Left => {
                let window_count = windows.len();
                if window_count == 0 {
                    return;
                }
                let output = state
                    .space
                    .output_under(state.pointer_location)
                    .next()
                    .unwrap()
                    .clone();
                let output_size = state.space.output_geometry(&output).unwrap().size;
                if window_count == 1 {
                    let window = windows[0].clone();

                    window.toplevel().with_pending_state(|tl_state| {
                        tl_state.size = Some(state.space.output_geometry(&output).unwrap().size);
                    });

                    window.toplevel().send_pending_configure();

                    state
                        .space
                        .map_element(window, output.current_location(), false);
                    return;
                }
                let mut windows = windows.iter();
                let first_window = windows.next().unwrap();

                first_window.toplevel().with_pending_state(|tl_state| {
                    let mut size = state.space.output_geometry(&output).unwrap().size;
                    size.w /= 2;
                    tl_state.size = Some(size);
                });

                first_window.toplevel().send_pending_configure();
                state
                    .space
                    .map_element(first_window.clone(), output.current_location(), false);

                let window_count = windows.len() as i32;
                let height = output_size.h / window_count;
                let x = output.current_location().x + output_size.w / 2;

                for (i, win) in windows.enumerate() {
                    win.toplevel().with_pending_state(|state| {
                        let mut new_size = output_size;
                        new_size.w /= 2;
                        new_size.h /= window_count;
                        state.size = Some(new_size);
                    });

                    win.toplevel().send_pending_configure();

                    let mut new_loc = output.current_location();
                    new_loc.x = x;
                    new_loc.y = i as i32 * height;

                    state.space.map_element(win.clone(), new_loc, false);
                }
            }
            MasterStackSide::Right => todo!(),
            MasterStackSide::Top => todo!(),
            MasterStackSide::Bottom => todo!(),
        }
    }
}
