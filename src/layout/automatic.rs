use smithay::{
    desktop::Window,
    utils::SERIAL_COUNTER,
    wayland::{compositor, shell::xdg::XdgToplevelSurfaceData},
};

use crate::{
    backend::Backend,
    state::State,
    window::window_state::{WindowResizeState, WindowState},
};

use super::{Direction, Layout};

impl Layout {
    pub fn master_stack<B: Backend>(
        state: &mut State<B>,
        mut windows: Vec<Window>,
        side: Direction,
    ) {
        windows.retain(|win| WindowState::with_state(win, |state| state.floating.is_tiled()));
        match side {
            Direction::Left => {
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

                    // state.loop_handle.insert_idle(move |_calloop_data| {
                    //     window.toplevel().send_pending_configure();
                    // });
                    let initial_configure_sent =
                        compositor::with_states(window.toplevel().wl_surface(), |states| {
                            states
                                .data_map
                                .get::<XdgToplevelSurfaceData>()
                                .unwrap()
                                .lock()
                                .unwrap()
                                .initial_configure_sent
                        });
                    if initial_configure_sent {
                        WindowState::with_state(&window, |state| {
                            state.resize_state = WindowResizeState::WaitingForAck(
                                window.toplevel().send_configure(),
                                output.current_location(),
                            );
                        });
                    }

                    // state
                    //     .space
                    //     .map_element(window, output.current_location(), false);
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

                    // state
                    //     .space
                    //     .map_element(first_window.clone(), output.current_location(), false);

                    let initial_configure_sent =
                        compositor::with_states(first_window.toplevel().wl_surface(), |states| {
                            states
                                .data_map
                                .get::<XdgToplevelSurfaceData>()
                                .unwrap()
                                .lock()
                                .unwrap()
                                .initial_configure_sent
                        });
                    if initial_configure_sent {
                        WindowState::with_state(first_window, |state| {
                            state.resize_state = WindowResizeState::WaitingForAck(
                                first_window.toplevel().send_configure(),
                                output.current_location(),
                            );
                        });
                    }

                    let window_count = windows.len() as i32;
                    let height = output_size.h / window_count;
                    let x = output.current_location().x + output_size.w / 2;

                    for (i, win) in windows.enumerate() {
                        win.toplevel().with_pending_state(|state| {
                            let mut new_size = output_size;
                            new_size.w /= 2;
                            new_size.w = new_size.w.clamp(40, i32::MAX);
                            new_size.h /= window_count;
                            new_size.h = new_size.h.clamp(40, i32::MAX);
                            state.size = Some(new_size);
                        });

                        let mut new_loc = output.current_location();
                        new_loc.x = x;
                        new_loc.y = (i as i32) * height;

                        // state.space.map_element(win.clone(), new_loc, false);

                        let initial_configure_sent =
                            compositor::with_states(win.toplevel().wl_surface(), |states| {
                                states
                                    .data_map
                                    .get::<XdgToplevelSurfaceData>()
                                    .unwrap()
                                    .lock()
                                    .unwrap()
                                    .initial_configure_sent
                            });
                        if initial_configure_sent {
                            WindowState::with_state(win, |state| {
                                state.resize_state = WindowResizeState::WaitingForAck(
                                    win.toplevel().send_configure(),
                                    new_loc,
                                );
                            });
                        }
                    }
                }

                // for win in windows {
                //     let initial_configure_sent =
                //         compositor::with_states(win.toplevel().wl_surface(), |states| {
                //             states
                //                 .data_map
                //                 .get::<XdgToplevelSurfaceData>()
                //                 .unwrap()
                //                 .lock()
                //                 .unwrap()
                //                 .initial_configure_sent
                //         });
                //     if initial_configure_sent {
                //         win.toplevel().send_configure();
                //     }
                // }
            }
            Direction::Right => todo!(),
            Direction::Top => todo!(),
            Direction::Bottom => todo!(),
        }
    }
}
