// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use smithay::{
    desktop::{Space, Window},
    output::Output,
    utils::{Logical, Size},
};

use crate::{
    backend::Backend,
    state::{State, WithState},
    tag::Tag,
    window::window_state::WindowResizeState,
};

pub enum Direction {
    Left,
    Right,
    Top,
    Bottom,
}

// TODO: couple this with the layouts
#[derive(Debug, Clone, Copy)]
pub enum Layout {
    MasterStack,
    Dwindle,
}

impl Layout {
    pub fn layout(
        &self,
        windows: Vec<Window>,
        tags: Vec<Tag>,
        space: &Space<Window>,
        output: &Output,
    ) {
        let windows = filter_windows(&windows, tags);
        match self {
            Layout::MasterStack => {
                let master = windows.first();
                let stack = windows.iter().skip(1);

                let Some(master) = master else { return };

                let Some(output_geo) = space.output_geometry(output) else {
                    tracing::error!("could not get output geometry");
                    return;
                };

                let output_loc = output.current_location();

                let stack_count = stack.clone().count();

                if stack_count == 0 {
                    // one window
                    master.toplevel().with_pending_state(|state| {
                        state.size = Some(output_geo.size);
                    });

                    master.with_state(|state| {
                        state.resize_state = WindowResizeState::WaitingForAck(
                            master.toplevel().send_configure(),
                            (output_loc.x, output_loc.y).into(),
                        );
                    });
                } else {
                    let new_master_size: Size<i32, Logical> =
                        (output_geo.size.w / 2, output_geo.size.h).into();
                    master.toplevel().with_pending_state(|state| {
                        state.size = Some(new_master_size);
                    });
                    master.with_state(|state| {
                        state.resize_state = WindowResizeState::WaitingForAck(
                            master.toplevel().send_configure(),
                            (output_loc.x, output_loc.y).into(),
                        );
                    });

                    let stack_count = stack_count;

                    let Some(output_geo) = space.output_geometry(output) else {
                        tracing::error!("could not get output geometry");
                        return;
                    };

                    let output_loc = output.current_location();

                    let height = output_geo.size.h / stack_count as i32;

                    for (i, win) in stack.enumerate() {
                        win.toplevel().with_pending_state(|state| {
                            state.size = Some((output_geo.size.w / 2, height).into());
                        });

                        win.with_state(|state| {
                            state.resize_state = WindowResizeState::WaitingForAck(
                                win.toplevel().send_configure(),
                                (
                                    output_geo.size.w / 2 + output_loc.x,
                                    i as i32 * height + output_loc.y,
                                )
                                    .into(),
                            );
                        });
                    }
                }
            }
            Layout::Dwindle => {
                let mut iter = windows.windows(2).peekable();
                let Some(output_geo) = space.output_geometry(output) else {
                    tracing::error!("could not get output geometry");
                    return;
                };

                let output_loc = output.current_location();

                if iter.peek().is_none() {
                    if let Some(window) = windows.first() {
                        window.toplevel().with_pending_state(|state| {
                            state.size = Some(output_geo.size);
                        });

                        window.with_state(|state| {
                            state.resize_state = WindowResizeState::WaitingForAck(
                                window.toplevel().send_configure(),
                                (output_loc.x, output_loc.y).into(),
                            );
                        });
                    }
                } else {
                    let mut div_factor_w = 1;
                    let mut div_factor_h = 1;
                    let mut x_factor_1: f32;
                    let mut y_factor_1: f32;
                    let mut x_factor_2: f32 = 0.0;
                    let mut y_factor_2: f32 = 0.0;

                    for (i, wins) in iter.enumerate() {
                        let win1 = &wins[0];
                        let win2 = &wins[1];

                        if i % 2 == 0 {
                            div_factor_w *= 2;
                        } else {
                            div_factor_h *= 2;
                        }

                        win1.toplevel().with_pending_state(|state| {
                            let new_size = (
                                output_geo.size.w / div_factor_w,
                                output_geo.size.h / div_factor_h,
                            )
                                .into();
                            state.size = Some(new_size);
                        });
                        win2.toplevel().with_pending_state(|state| {
                            let new_size = (
                                output_geo.size.w / div_factor_w,
                                output_geo.size.h / div_factor_h,
                            )
                                .into();
                            state.size = Some(new_size);
                        });

                        x_factor_1 = x_factor_2;
                        y_factor_1 = y_factor_2;

                        if i % 2 == 0 {
                            x_factor_2 += (1.0 - x_factor_2) / 2.0;
                        } else {
                            y_factor_2 += (1.0 - y_factor_2) / 2.0;
                        }

                        win1.with_state(|state| {
                            let new_loc = (
                                (output_geo.size.w as f32 * x_factor_1 + output_loc.x as f32)
                                    as i32,
                                (output_geo.size.h as f32 * y_factor_1 + output_loc.y as f32)
                                    as i32,
                            )
                                .into();
                            state.resize_state = WindowResizeState::WaitingForAck(
                                win1.toplevel().send_configure(),
                                new_loc,
                            );
                        });
                        win2.with_state(|state| {
                            let new_loc = (
                                (output_geo.size.w as f32 * x_factor_2 + output_loc.x as f32)
                                    as i32,
                                (output_geo.size.h as f32 * y_factor_2 + output_loc.y as f32)
                                    as i32,
                            )
                                .into();
                            state.resize_state = WindowResizeState::WaitingForAck(
                                win2.toplevel().send_configure(),
                                new_loc,
                            );
                        });
                    }
                }
            }
        }
    }
}

fn filter_windows(windows: &[Window], tags: Vec<Tag>) -> Vec<Window> {
    windows
        .iter()
        .filter(|window| {
            window.with_state(|state| {
                state.floating.is_tiled() && {
                    for tag in state.tags.iter() {
                        if tags.iter().any(|tg| tg == tag) {
                            return true;
                        }
                    }
                    false
                }
            })
        })
        .cloned()
        .collect()
}

impl<B: Backend> State<B> {
    pub fn swap_window_positions(&mut self, win1: &Window, win2: &Window) {
        // FIXME: moving the mouse quickly will break swapping

        let win1_loc = self.space.element_location(win1).unwrap(); // TODO: handle unwraps
        let win2_loc = self.space.element_location(win2).unwrap();
        let win1_geo = win1.geometry();
        let win2_geo = win2.geometry();

        win1.toplevel().with_pending_state(|state| {
            state.size = Some(win2_geo.size);
        });
        win2.toplevel().with_pending_state(|state| {
            state.size = Some(win1_geo.size);
        });

        let serial = win1.toplevel().send_configure();
        win1.with_state(|state| {
            state.resize_state = WindowResizeState::WaitingForAck(serial, win2_loc);
        });

        let serial = win2.toplevel().send_configure();
        win2.with_state(|state| {
            state.resize_state = WindowResizeState::WaitingForAck(serial, win1_loc);
        });

        let mut elems = self
            .windows
            .iter_mut()
            .filter(|win| *win == win1 || *win == win2);

        let (first, second) = (elems.next(), elems.next());

        if let Some(first) = first {
            if let Some(second) = second {
                std::mem::swap(first, second);
            }
        }
    }
}
