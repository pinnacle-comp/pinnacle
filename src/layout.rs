// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use itertools::{Either, Itertools};
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

// TODO: couple this with the layouts
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum Layout {
    MasterStack,
    Dwindle,
    Spiral,
    CornerTopLeft,
    CornerTopRight,
    CornerBottomLeft,
    CornerBottomRight,
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

        let Some(output_geo) = space.output_geometry(output) else {
            tracing::error!("could not get output geometry");
            return;
        };

        let output_loc = output.current_location();

        match self {
            Layout::MasterStack => {
                let master = windows.first();
                let stack = windows.iter().skip(1);

                let Some(master) = master else { return };

                let stack_count = stack.clone().count();

                if stack_count == 0 {
                    // one window
                    master.toplevel().with_pending_state(|state| {
                        state.size = Some(output_geo.size);
                    });

                    master.with_state(|state| {
                        state.resize_state = WindowResizeState::Requested(
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
                        state.resize_state = WindowResizeState::Requested(
                            master.toplevel().send_configure(),
                            (output_loc.x, output_loc.y).into(),
                        );
                    });

                    let stack_count = stack_count;

                    let height = output_geo.size.h as f32 / stack_count as f32;
                    let mut y_s = vec![];
                    for i in 0..stack_count {
                        y_s.push((i as f32 * height).round() as i32);
                    }
                    let heights = y_s
                        .windows(2)
                        .map(|pair| pair[1] - pair[0])
                        .chain(vec![output_geo.size.h - y_s.last().expect("vec was empty")])
                        .collect::<Vec<_>>();

                    for (i, win) in stack.enumerate() {
                        win.toplevel().with_pending_state(|state| {
                            // INFO: Some windows crash the compositor if they become too short in height,
                            // |     so they're limited to a minimum of 40 pixels as a workaround.
                            state.size =
                                Some((output_geo.size.w / 2, i32::max(heights[i], 40)).into());
                        });

                        win.with_state(|state| {
                            state.resize_state = WindowResizeState::Requested(
                                win.toplevel().send_configure(),
                                (output_geo.size.w / 2 + output_loc.x, y_s[i] + output_loc.y)
                                    .into(),
                            );
                        });
                    }
                }
            }
            Layout::Dwindle => {
                let mut iter = windows.windows(2).peekable();

                if iter.peek().is_none() {
                    if let Some(window) = windows.first() {
                        window.toplevel().with_pending_state(|state| {
                            state.size = Some(output_geo.size);
                        });

                        window.with_state(|state| {
                            state.resize_state = WindowResizeState::Requested(
                                window.toplevel().send_configure(),
                                (output_loc.x, output_loc.y).into(),
                            );
                        });
                    }
                } else {
                    for (i, wins) in iter.enumerate() {
                        let win1 = &wins[0];
                        let win2 = &wins[1];

                        enum Slice {
                            Right,
                            Below,
                        }

                        let slice = if i % 2 == 0 {
                            Slice::Right
                        } else {
                            Slice::Below
                        };

                        if i == 0 {
                            win1.toplevel()
                                .with_pending_state(|state| state.size = Some(output_geo.size));
                            win1.with_state(|state| {
                                state.resize_state = WindowResizeState::Requested(
                                    win1.toplevel().send_configure(),
                                    output_loc,
                                )
                            });
                        }

                        let win1_size = win1.toplevel().with_pending_state(|state| {
                            state.size.expect("size should have been set")
                        });
                        let win1_loc = win1.with_state(|state| {
                            let WindowResizeState::Requested(_, loc) = state.resize_state else { unreachable!() };
                            loc
                        });

                        match slice {
                            Slice::Right => {
                                let width_partition = win1_size.w / 2;
                                win1.toplevel().with_pending_state(|state| {
                                    state.size = Some(
                                        (win1_size.w - width_partition, i32::max(win1_size.h, 40))
                                            .into(),
                                    );
                                });
                                win1.with_state(|state| {
                                    state.resize_state = WindowResizeState::Requested(
                                        win1.toplevel().send_configure(),
                                        win1_loc,
                                    );
                                });
                                win2.toplevel().with_pending_state(|state| {
                                    state.size =
                                        Some((width_partition, i32::max(win1_size.h, 40)).into());
                                });
                                win2.with_state(|state| {
                                    state.resize_state = WindowResizeState::Requested(
                                        win2.toplevel().send_configure(),
                                        (win1_loc.x + (win1_size.w - width_partition), win1_loc.y)
                                            .into(),
                                    );
                                });
                            }
                            Slice::Below => {
                                let height_partition = win1_size.h / 2;
                                win1.toplevel().with_pending_state(|state| {
                                    state.size = Some(
                                        (win1_size.w, i32::max(win1_size.h - height_partition, 40))
                                            .into(),
                                    );
                                });
                                win1.with_state(|state| {
                                    state.resize_state = WindowResizeState::Requested(
                                        win1.toplevel().send_configure(),
                                        win1_loc,
                                    );
                                });
                                win2.toplevel().with_pending_state(|state| {
                                    state.size =
                                        Some((win1_size.w, i32::max(height_partition, 40)).into());
                                });
                                win2.with_state(|state| {
                                    state.resize_state = WindowResizeState::Requested(
                                        win2.toplevel().send_configure(),
                                        (win1_loc.x, win1_loc.y + (win1_size.h - height_partition))
                                            .into(),
                                    );
                                });
                            }
                        }
                    }
                }
            }
            Layout::Spiral => {
                let mut iter = windows.windows(2).peekable();

                if iter.peek().is_none() {
                    if let Some(window) = windows.first() {
                        window.toplevel().with_pending_state(|state| {
                            state.size = Some(output_geo.size);
                        });

                        window.with_state(|state| {
                            state.resize_state = WindowResizeState::Requested(
                                window.toplevel().send_configure(),
                                (output_loc.x, output_loc.y).into(),
                            );
                        });
                    }
                } else {
                    for (i, wins) in iter.enumerate() {
                        let win1 = &wins[0];
                        let win2 = &wins[1];

                        enum Slice {
                            Above,
                            Below,
                            Left,
                            Right,
                        }

                        let slice = match i % 4 {
                            0 => Slice::Right,
                            1 => Slice::Below,
                            2 => Slice::Left,
                            3 => Slice::Above,
                            _ => unreachable!(),
                        };

                        if i == 0 {
                            win1.toplevel()
                                .with_pending_state(|state| state.size = Some(output_geo.size));
                            win1.with_state(|state| {
                                state.resize_state = WindowResizeState::Requested(
                                    win1.toplevel().send_configure(),
                                    output_loc,
                                )
                            });
                        }

                        let win1_size = win1.toplevel().with_pending_state(|state| {
                            state.size.expect("size should have been set")
                        });
                        let win1_loc = win1.with_state(|state| {
                            let WindowResizeState::Requested(_, loc) = state.resize_state else { unreachable!() };
                            loc
                        });

                        match slice {
                            Slice::Above => {
                                let height_partition = win1_size.h / 2;
                                win1.toplevel().with_pending_state(|state| {
                                    state.size = Some(
                                        (win1_size.w, i32::max(win1_size.h - height_partition, 40))
                                            .into(),
                                    );
                                });
                                win1.with_state(|state| {
                                    state.resize_state = WindowResizeState::Requested(
                                        win1.toplevel().send_configure(),
                                        (win1_loc.x, win1_loc.y + height_partition).into(),
                                    );
                                });
                                win2.toplevel().with_pending_state(|state| {
                                    state.size =
                                        Some((win1_size.w, i32::max(height_partition, 40)).into());
                                });
                                win2.with_state(|state| {
                                    state.resize_state = WindowResizeState::Requested(
                                        win2.toplevel().send_configure(),
                                        win1_loc,
                                    );
                                });
                            }
                            Slice::Below => {
                                let height_partition = win1_size.h / 2;
                                win1.toplevel().with_pending_state(|state| {
                                    state.size = Some(
                                        (win1_size.w, win1_size.h - i32::max(height_partition, 40))
                                            .into(),
                                    );
                                });
                                win1.with_state(|state| {
                                    state.resize_state = WindowResizeState::Requested(
                                        win1.toplevel().send_configure(),
                                        win1_loc,
                                    );
                                });
                                win2.toplevel().with_pending_state(|state| {
                                    state.size =
                                        Some((win1_size.w, i32::max(height_partition, 40)).into());
                                });
                                win2.with_state(|state| {
                                    state.resize_state = WindowResizeState::Requested(
                                        win2.toplevel().send_configure(),
                                        (win1_loc.x, win1_loc.y + (win1_size.h - height_partition))
                                            .into(),
                                    );
                                });
                            }
                            Slice::Left => {
                                let width_partition = win1_size.w / 2;
                                win1.toplevel().with_pending_state(|state| {
                                    state.size = Some(
                                        (win1_size.w - width_partition, i32::max(win1_size.h, 40))
                                            .into(),
                                    );
                                });
                                win1.with_state(|state| {
                                    state.resize_state = WindowResizeState::Requested(
                                        win1.toplevel().send_configure(),
                                        (win1_loc.x + width_partition, win1_loc.y).into(),
                                    );
                                });
                                win2.toplevel().with_pending_state(|state| {
                                    state.size =
                                        Some((width_partition, i32::max(win1_size.h, 40)).into());
                                });
                                win2.with_state(|state| {
                                    state.resize_state = WindowResizeState::Requested(
                                        win2.toplevel().send_configure(),
                                        win1_loc,
                                    );
                                });
                            }
                            Slice::Right => {
                                let width_partition = win1_size.w / 2;
                                win1.toplevel().with_pending_state(|state| {
                                    state.size = Some(
                                        (win1_size.w - width_partition, i32::max(win1_size.h, 40))
                                            .into(),
                                    );
                                });
                                win1.with_state(|state| {
                                    state.resize_state = WindowResizeState::Requested(
                                        win1.toplevel().send_configure(),
                                        win1_loc,
                                    );
                                });
                                win2.toplevel().with_pending_state(|state| {
                                    state.size =
                                        Some((width_partition, i32::max(win1_size.h, 40)).into());
                                });
                                win2.with_state(|state| {
                                    state.resize_state = WindowResizeState::Requested(
                                        win2.toplevel().send_configure(),
                                        (win1_loc.x + (win1_size.w - width_partition), win1_loc.y)
                                            .into(),
                                    );
                                });
                            }
                        }
                    }
                }
            }
            layout @ (Layout::CornerTopLeft
            | Layout::CornerTopRight
            | Layout::CornerBottomLeft
            | Layout::CornerBottomRight) => match windows.len() {
                0 => (),
                1 => {
                    windows[0].toplevel().with_pending_state(|state| {
                        state.size = Some(output_geo.size);
                    });

                    windows[0].with_state(|state| {
                        state.resize_state = WindowResizeState::Requested(
                            windows[0].toplevel().send_configure(),
                            (output_loc.x, output_loc.y).into(),
                        );
                    });
                }
                2 => {
                    windows[0].toplevel().with_pending_state(|state| {
                        state.size = Some((output_geo.size.w / 2, output_geo.size.h).into());
                    });
                    windows[0].with_state(|state| {
                        state.resize_state = WindowResizeState::Requested(
                            windows[0].toplevel().send_configure(),
                            (output_loc.x, output_loc.y).into(),
                        );
                    });
                    windows[1].toplevel().with_pending_state(|state| {
                        state.size = Some((output_geo.size.w / 2, output_geo.size.h).into());
                    });
                    windows[1].with_state(|state| {
                        state.resize_state = WindowResizeState::Requested(
                            windows[1].toplevel().send_configure(),
                            (output_loc.x + output_geo.size.w / 2, output_loc.y).into(),
                        );
                    });
                }
                _ => {
                    let mut windows = windows.into_iter();
                    let Some(corner) = windows.next() else { unreachable!() };
                    let (horiz_stack, vert_stack): (Vec<Window>, Vec<Window>) =
                        windows.enumerate().partition_map(|(i, win)| {
                            if i % 2 == 0 {
                                Either::Left(win)
                            } else {
                                Either::Right(win)
                            }
                        });

                    let div_factor = 2;

                    corner.toplevel().with_pending_state(|state| {
                        state.size = Some(
                            (
                                output_geo.size.w / div_factor,
                                output_geo.size.h / div_factor,
                            )
                                .into(),
                        );
                    });
                    corner.with_state(|state| {
                        state.resize_state = WindowResizeState::Requested(
                            corner.toplevel().send_configure(),
                            match layout {
                                Layout::CornerTopLeft => (output_loc.x, output_loc.y),
                                Layout::CornerTopRight => (
                                    output_loc.x + output_geo.size.w
                                        - output_geo.size.w / div_factor,
                                    output_loc.y,
                                ),
                                Layout::CornerBottomLeft => (
                                    output_loc.x,
                                    output_loc.y + output_geo.size.h
                                        - output_geo.size.h / div_factor,
                                ),
                                Layout::CornerBottomRight => (
                                    output_loc.x + output_geo.size.w
                                        - output_geo.size.w / div_factor,
                                    output_loc.y + output_geo.size.h
                                        - output_geo.size.h / div_factor,
                                ),
                                _ => unreachable!(),
                            }
                            .into(),
                        );
                    });

                    let vert_stack_count = vert_stack.len();

                    let height = output_geo.size.h as f32 / vert_stack_count as f32;
                    let mut y_s = vec![];
                    for i in 0..vert_stack_count {
                        y_s.push((i as f32 * height).round() as i32);
                    }
                    let heights = y_s
                        .windows(2)
                        .map(|pair| pair[1] - pair[0])
                        .chain(vec![output_geo.size.h - y_s.last().expect("vec was empty")])
                        .collect::<Vec<_>>();

                    for (i, win) in vert_stack.iter().enumerate() {
                        win.toplevel().with_pending_state(|state| {
                            // INFO: Some windows crash the compositor if they become too short in height,
                            // |     so they're limited to a minimum of 40 pixels as a workaround.
                            state.size =
                                Some((output_geo.size.w / 2, i32::max(heights[i], 40)).into());
                        });

                        win.with_state(|state| {
                            state.resize_state = WindowResizeState::Requested(
                                win.toplevel().send_configure(),
                                (
                                    match layout {
                                        Layout::CornerTopLeft | Layout::CornerBottomLeft => {
                                            output_geo.size.w / 2 + output_loc.x
                                        }
                                        Layout::CornerTopRight | Layout::CornerBottomRight => {
                                            output_loc.x
                                        }
                                        _ => unreachable!(),
                                    },
                                    y_s[i] + output_loc.y,
                                )
                                    .into(),
                            );
                        });
                    }

                    let horiz_stack_count = horiz_stack.len();

                    let width = output_geo.size.w as f32 / 2.0 / horiz_stack_count as f32;
                    let mut x_s = vec![];
                    for i in 0..horiz_stack_count {
                        x_s.push((i as f32 * width).round() as i32);
                    }
                    let widths = x_s
                        .windows(2)
                        .map(|pair| pair[1] - pair[0])
                        .chain(vec![
                            output_geo.size.w / 2 - x_s.last().expect("vec was empty"),
                        ])
                        .collect::<Vec<_>>();

                    for (i, win) in horiz_stack.iter().enumerate() {
                        win.toplevel().with_pending_state(|state| {
                            // INFO: Some windows crash the compositor if they become too short in height,
                            // |     so they're limited to a minimum of 40 pixels as a workaround.
                            state.size =
                                Some((i32::max(widths[i], 1), output_geo.size.h / 2).into());
                        });

                        win.with_state(|state| {
                            state.resize_state = WindowResizeState::Requested(
                                win.toplevel().send_configure(),
                                match layout {
                                    Layout::CornerTopLeft => (
                                        x_s[i] + output_loc.x,
                                        output_loc.y + output_geo.size.h / 2,
                                    ),
                                    Layout::CornerTopRight => (
                                        x_s[i] + output_loc.x + output_geo.size.w / 2,
                                        output_loc.y + output_geo.size.h / 2,
                                    ),
                                    Layout::CornerBottomLeft => {
                                        (x_s[i] + output_loc.x, output_loc.y)
                                    }
                                    Layout::CornerBottomRight => (
                                        x_s[i] + output_loc.x + output_geo.size.w / 2,
                                        output_loc.y,
                                    ),
                                    _ => unreachable!(),
                                }
                                .into(),
                            );
                        });
                    }
                }
            },
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
            state.resize_state = WindowResizeState::Requested(serial, win2_loc);
        });

        let serial = win2.toplevel().send_configure();
        win2.with_state(|state| {
            state.resize_state = WindowResizeState::Requested(serial, win1_loc);
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
