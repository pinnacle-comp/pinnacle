// SPDX-License-Identifier: GPL-3.0-or-later

use itertools::{Either, Itertools};
use smithay::{
    desktop::Space,
    output::Output,
    utils::{Logical, Size},
};

use crate::{
    backend::Backend,
    state::{State, WithState},
    tag::Tag,
    window::WindowElement,
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
        windows: Vec<WindowElement>,
        tags: Vec<Tag>,
        space: &Space<WindowElement>,
        output: &Output,
    ) {
        let windows = filter_windows(&windows, tags);

        let Some(output_geo) = space.output_geometry(output) else {
            tracing::error!("could not get output geometry");
            return;
        };

        let output_loc = output.current_location();

        match self {
            Layout::MasterStack => master_stack(windows, space, output),
            Layout::Dwindle => dwindle(windows, space, output),
            Layout::Spiral => spiral(windows, space, output),
            layout @ (Layout::CornerTopLeft
            | Layout::CornerTopRight
            | Layout::CornerBottomLeft
            | Layout::CornerBottomRight) => corner(layout, windows, space, output),
        }
    }
}

fn master_stack(windows: Vec<WindowElement>, space: &Space<WindowElement>, output: &Output) {
    let Some(output_geo) = space.output_geometry(output) else {
        tracing::error!("could not get output geometry");
        return;
    };

    let output_loc = output.current_location();

    let master = windows.first();
    let stack = windows.iter().skip(1);

    let Some(master) = master else { return };

    let stack_count = stack.clone().count();

    if stack_count == 0 {
        // one window
        master.request_size_change(output_loc, output_geo.size);
    } else {
        let loc = (output_loc.x, output_loc.y).into();
        let new_master_size: Size<i32, Logical> = (output_geo.size.w / 2, output_geo.size.h).into();
        master.request_size_change(loc, new_master_size);

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
            win.request_size_change(
                (output_geo.size.w / 2 + output_loc.x, y_s[i] + output_loc.y).into(),
                (output_geo.size.w / 2, i32::max(heights[i], 40)).into(),
            );
        }
    }
}

fn dwindle(windows: Vec<WindowElement>, space: &Space<WindowElement>, output: &Output) {
    let Some(output_geo) = space.output_geometry(output) else {
        tracing::error!("could not get output geometry");
        return;
    };

    let output_loc = output.current_location();

    let mut iter = windows.windows(2).peekable();

    if iter.peek().is_none() {
        if let Some(window) = windows.first() {
            window.request_size_change(output_loc, output_geo.size);
        }
    } else {
        let mut win1_size = output_geo.size;
        let mut win1_loc = output_loc;
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

            match slice {
                Slice::Right => {
                    let width_partition = win1_size.w / 2;

                    win1.request_size_change(
                        win1_loc,
                        (win1_size.w - width_partition, i32::max(win1_size.h, 40)).into(),
                    );

                    win1_loc = (win1_loc.x + (win1_size.w - width_partition), win1_loc.y).into();
                    win1_size = (width_partition, i32::max(win1_size.h, 40)).into();

                    win2.request_size_change(win1_loc, win1_size);
                }
                Slice::Below => {
                    let height_partition = win1_size.h / 2;

                    win1.request_size_change(
                        win1_loc,
                        (win1_size.w, i32::max(win1_size.h - height_partition, 40)).into(),
                    );

                    win1_loc = (win1_loc.x, win1_loc.y + (win1_size.h - height_partition)).into();
                    win1_size = (win1_size.w, i32::max(height_partition, 40)).into();

                    win2.request_size_change(win1_loc, win1_size);
                }
            }
        }
    }
}

fn spiral(windows: Vec<WindowElement>, space: &Space<WindowElement>, output: &Output) {
    let Some(output_geo) = space.output_geometry(output) else {
        tracing::error!("could not get output geometry");
        return;
    };

    let output_loc = output.current_location();

    let mut iter = windows.windows(2).peekable();

    if iter.peek().is_none() {
        if let Some(window) = windows.first() {
            window.request_size_change(output_loc, output_geo.size);
        }
    } else {
        let mut win1_loc = output_loc;
        let mut win1_size = output_geo.size;

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

            match slice {
                Slice::Above => {
                    let height_partition = win1_size.h / 2;

                    win1.request_size_change(
                        (win1_loc.x, win1_loc.y + height_partition).into(),
                        (win1_size.w, i32::max(win1_size.h - height_partition, 40)).into(),
                    );

                    win1_size = (win1_size.w, i32::max(height_partition, 40)).into();
                    win2.request_size_change(win1_loc, win1_size);
                }
                Slice::Below => {
                    let height_partition = win1_size.h / 2;

                    win1.request_size_change(
                        win1_loc,
                        (win1_size.w, win1_size.h - i32::max(height_partition, 40)).into(),
                    );

                    win1_loc = (win1_loc.x, win1_loc.y + (win1_size.h - height_partition)).into();
                    win1_size = (win1_size.w, i32::max(height_partition, 40)).into();
                    win2.request_size_change(win1_loc, win1_size);
                }
                Slice::Left => {
                    let width_partition = win1_size.w / 2;

                    win1.request_size_change(
                        (win1_loc.x + width_partition, win1_loc.y).into(),
                        (win1_size.w - width_partition, i32::max(win1_size.h, 40)).into(),
                    );

                    win1_size = (width_partition, i32::max(win1_size.h, 40)).into();
                    win2.request_size_change(win1_loc, win1_size);
                }
                Slice::Right => {
                    let width_partition = win1_size.w / 2;

                    win1.request_size_change(
                        win1_loc,
                        (win1_size.w - width_partition, i32::max(win1_size.h, 40)).into(),
                    );

                    win1_loc = (win1_loc.x + (win1_size.w - width_partition), win1_loc.y).into();
                    win1_size = (width_partition, i32::max(win1_size.h, 40)).into();
                    win2.request_size_change(win1_loc, win1_size);
                }
            }
        }
    }
}

fn corner(
    layout: &Layout,
    windows: Vec<WindowElement>,
    space: &Space<WindowElement>,
    output: &Output,
) {
    let Some(output_geo) = space.output_geometry(output) else {
        tracing::error!("could not get output geometry");
        return;
    };

    let output_loc = output.current_location();
    match windows.len() {
        0 => (),
        1 => {
            windows[0].request_size_change(output_loc, output_geo.size);
        }
        2 => {
            windows[0].request_size_change(
                output_loc,
                (output_geo.size.w / 2, output_geo.size.h).into(),
            );

            windows[1].request_size_change(
                (output_loc.x + output_geo.size.w / 2, output_loc.y).into(),
                (output_geo.size.w / 2, output_geo.size.h).into(),
            );
        }
        _ => {
            let mut windows = windows.into_iter();
            let Some(corner) = windows.next() else { unreachable!() };
            let (horiz_stack, vert_stack): (Vec<WindowElement>, Vec<WindowElement>) =
                windows.enumerate().partition_map(|(i, win)| {
                    if i % 2 == 0 {
                        Either::Left(win)
                    } else {
                        Either::Right(win)
                    }
                });

            let div_factor = 2;

            corner.request_size_change(
                match layout {
                    Layout::CornerTopLeft => (output_loc.x, output_loc.y),
                    Layout::CornerTopRight => (
                        output_loc.x + output_geo.size.w - output_geo.size.w / div_factor,
                        output_loc.y,
                    ),
                    Layout::CornerBottomLeft => (
                        output_loc.x,
                        output_loc.y + output_geo.size.h - output_geo.size.h / div_factor,
                    ),
                    Layout::CornerBottomRight => (
                        output_loc.x + output_geo.size.w - output_geo.size.w / div_factor,
                        output_loc.y + output_geo.size.h - output_geo.size.h / div_factor,
                    ),
                    _ => unreachable!(),
                }
                .into(),
                (
                    output_geo.size.w / div_factor,
                    output_geo.size.h / div_factor,
                )
                    .into(),
            );

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
                win.request_size_change(
                    (
                        match layout {
                            Layout::CornerTopLeft | Layout::CornerBottomLeft => {
                                output_geo.size.w / 2 + output_loc.x
                            }
                            Layout::CornerTopRight | Layout::CornerBottomRight => output_loc.x,
                            _ => unreachable!(),
                        },
                        y_s[i] + output_loc.y,
                    )
                        .into(),
                    (output_geo.size.w / 2, i32::max(heights[i], 40)).into(),
                );
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
                win.request_size_change(
                    match layout {
                        Layout::CornerTopLeft => {
                            (x_s[i] + output_loc.x, output_loc.y + output_geo.size.h / 2)
                        }
                        Layout::CornerTopRight => (
                            x_s[i] + output_loc.x + output_geo.size.w / 2,
                            output_loc.y + output_geo.size.h / 2,
                        ),
                        Layout::CornerBottomLeft => (x_s[i] + output_loc.x, output_loc.y),
                        Layout::CornerBottomRight => {
                            (x_s[i] + output_loc.x + output_geo.size.w / 2, output_loc.y)
                        }
                        _ => unreachable!(),
                    }
                    .into(),
                    (i32::max(widths[i], 1), output_geo.size.h / 2).into(),
                );
            }
        }
    }
}

fn filter_windows(windows: &[WindowElement], tags: Vec<Tag>) -> Vec<WindowElement> {
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
    pub fn swap_window_positions(&mut self, win1: &WindowElement, win2: &WindowElement) {
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

        let output = self.focus_state.focused_output.clone().unwrap(); // FIXME: unwrap
        self.re_layout(&output);
    }
}
