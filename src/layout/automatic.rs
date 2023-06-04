use smithay::desktop::Window;

use crate::{backend::Backend, state::State};

use super::{Layout, RemoveWindowError};

/// A layout which puts one "master" window on one half of the screen and splits other windows
/// among the other half.
pub struct MasterStack {
    pub windows: Vec<Window>,
    /// Which side of the screen the master window will be on
    pub side: MasterStackSide,
}

pub enum MasterStackSide {
    Left,
    Right,
    Top,
    Bottom,
}

impl<B: Backend> Layout<B> for MasterStack {
    fn layout_windows(&self, state: &mut State<B>, windows: Vec<Window>) {
        match self.side {
            MasterStackSide::Left => {
                // println!("MasterStack layout_windows");
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

                    state
                        .space
                        .map_element(window, output.current_location(), false);
                    return;
                }

                // INFO: this is in its own scope to drop the first_window reference so I can
                // |     move windows into the closure below
                {
                    let mut windows = windows.iter();
                    let first_window = windows.next().unwrap();

                    first_window.toplevel().with_pending_state(|tl_state| {
                        let mut size = state.space.output_geometry(&output).unwrap().size;
                        size.w /= 2;
                        tl_state.size = Some(size);
                    });

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

                        let mut new_loc = output.current_location();
                        new_loc.x = x;
                        new_loc.y = i as i32 * height;

                        state.space.map_element(win.clone(), new_loc, false);
                    }
                }

                state.backend_data.reset_buffers(&output);

                // INFO: We send configures when the event loop is idle so
                // |     CompositorHandler::commit() sends the initial configure
                state.loop_handle.insert_idle(|_calloop_data| {
                    for win in windows {
                        win.toplevel().send_pending_configure();
                    }
                });
            }
            MasterStackSide::Right => todo!(),
            MasterStackSide::Top => todo!(),
            MasterStackSide::Bottom => todo!(),
        }
    }

    fn add_window(&mut self, state: &mut State<B>, window: Window) {
        self.windows.push(window);
    }

    fn remove_window(
        &mut self,
        state: &mut State<B>,
        window: Window,
    ) -> Result<(), RemoveWindowError> {
        let pos = self
            .windows
            .iter()
            .position(|win| window == win.clone())
            .ok_or(RemoveWindowError::NotFound)?;

        self.windows.remove(pos);

        Ok(())
    }
}
