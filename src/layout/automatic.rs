// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use smithay::{
    desktop::Window,
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
                // TODO: change focused_output to be not an option
                let Some(output) = state
                    .focus_state
                    .focused_output
                    .as_ref()
                    .or_else(|| state.space.outputs().next()) 
                else {
                    tracing::warn!("no connected outputs");
                    return;
                    // TODO: no idea what happens if you spawn a window while no monitors are
                    // |     connected, figure that out
                };
                let output_size = state.space.output_geometry(output).unwrap().size;
                if window_count == 1 {
                    tracing::debug!("Laying out only window");
                    let window = windows[0].clone();

                    window.toplevel().with_pending_state(|tl_state| {
                        tl_state.size = Some(state.space.output_geometry(output).unwrap().size);
                    });

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
                    tracing::debug!("initial configure sent is {initial_configure_sent}");
                    if initial_configure_sent {
                        WindowState::with_state(&window, |state| {
                            tracing::debug!("sending configure");
                            state.resize_state = WindowResizeState::WaitingForAck(
                                window.toplevel().send_configure(),
                                output.current_location(),
                            );
                        });
                    }

                    return;
                }

                tracing::debug!("layed out first window");
                let mut windows = windows.iter();
                let first_window = windows.next().unwrap();

                first_window.toplevel().with_pending_state(|tl_state| {
                    let mut size = state.space.output_geometry(output).unwrap().size;
                    size.w /= 2;
                    tl_state.size = Some(size);
                });

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
                        new_size.w = new_size.w.clamp(1, i32::MAX);
                        new_size.h /= window_count;
                        // INFO: The newest window won't have its geometry.loc set until after here and I don't know
                        // |     why, so this is hardcoded to 40. I don't anticipate people using
                        // |     windows that are that short, so figuring it out is low priority.
                        // |     Kitty specifically will crash the compositor if it's resized such
                        // |     that the bottom border goes above the bottom of the title bar if
                        // |     this is set too low.
                        new_size.h = new_size.h.clamp(40, i32::MAX);
                        state.size = Some(new_size);
                    });

                    let mut new_loc = output.current_location();
                    new_loc.x = x;
                    new_loc.y = (i as i32) * height;

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
            Direction::Right => todo!(),
            Direction::Top => todo!(),
            Direction::Bottom => todo!(),
        }
    }
}
