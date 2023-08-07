// SPDX-License-Identifier: GPL-3.0-or-later

use itertools::{Either, Itertools};
use smithay::{
    desktop::{layer_map_for_output, Space},
    output::Output,
    utils::{Logical, Rectangle, Size},
};

use crate::{
    backend::Backend,
    state::{State, WithState},
    tag::Tag,
    window::WindowElement,
};

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
        space: &mut Space<WindowElement>,
        output: &Output,
    ) {
        let windows = filter_windows(&windows, tags);

        let Some(rect) = space.output_geometry(output).map(|op_geo| {
            let map = layer_map_for_output(output);
            if map.layers().peekable().peek().is_none() {
                // INFO: Sometimes the exclusive zone is some weird number that doesn't match the
                // |     output res, even when there are no layer surfaces mapped. In this case, we
                // |     just return the output geometry.
                op_geo
            } else {
                let zone = map.non_exclusive_zone();
                tracing::debug!("non_exclusive_zone is {zone:?}");
                Rectangle::from_loc_and_size(op_geo.loc + zone.loc, zone.size)
            }
        }) else {
            // TODO: maybe default to something like 800x800 like in anvil so people still see
            // |     windows open
            tracing::error!("Failed to get output geometry");
            return;
        };

        tracing::debug!("Laying out with rect {rect:?}");

        match self {
            Layout::MasterStack => master_stack(windows, space, rect),
            Layout::Dwindle => dwindle(windows, space, rect),
            Layout::Spiral => spiral(windows, space, rect),
            layout @ (Layout::CornerTopLeft
            | Layout::CornerTopRight
            | Layout::CornerBottomLeft
            | Layout::CornerBottomRight) => corner(layout, windows, space, rect),
        }
    }
}

fn master_stack(
    windows: Vec<WindowElement>,
    space: &mut Space<WindowElement>,
    rect: Rectangle<i32, Logical>,
) {
    let size = rect.size;
    let loc = rect.loc;

    let master = windows.first();
    let stack = windows.iter().skip(1);

    let Some(master) = master else { return };

    let stack_count = stack.clone().count();

    if stack_count == 0 {
        // one window
        master.request_size_change(space, loc, size);
    } else {
        let loc = (loc.x, loc.y).into();
        let new_master_size: Size<i32, Logical> = (size.w / 2, size.h).into();
        master.request_size_change(space, loc, new_master_size);

        let stack_count = stack_count;

        let height = size.h as f32 / stack_count as f32;
        let mut y_s = vec![];
        for i in 0..stack_count {
            y_s.push((i as f32 * height).round() as i32);
        }
        let heights = y_s
            .windows(2)
            .map(|pair| pair[1] - pair[0])
            .chain(vec![size.h - y_s.last().expect("vec was empty")])
            .collect::<Vec<_>>();

        for (i, win) in stack.enumerate() {
            win.request_size_change(
                space,
                (size.w / 2 + loc.x, y_s[i] + loc.y).into(),
                (size.w / 2, i32::max(heights[i], 40)).into(),
            );
        }
    }
}

fn dwindle(
    windows: Vec<WindowElement>,
    space: &mut Space<WindowElement>,
    rect: Rectangle<i32, Logical>,
) {
    let size = rect.size;
    let loc = rect.loc;

    let mut iter = windows.windows(2).peekable();

    if iter.peek().is_none() {
        if let Some(window) = windows.first() {
            window.request_size_change(space, loc, size);
        }
    } else {
        let mut win1_size = size;
        let mut win1_loc = loc;
        for (i, wins) in iter.enumerate() {
            let win1 = &wins[0];
            let win2 = &wins[1];

            enum Slice {
                Right,
                Below,
            }

            let slice = if i % 2 == 0 { Slice::Right } else { Slice::Below };

            match slice {
                Slice::Right => {
                    let width_partition = win1_size.w / 2;

                    win1.request_size_change(
                        space,
                        win1_loc,
                        (win1_size.w - width_partition, i32::max(win1_size.h, 40)).into(),
                    );

                    win1_loc = (win1_loc.x + (win1_size.w - width_partition), win1_loc.y).into();
                    win1_size = (width_partition, i32::max(win1_size.h, 40)).into();

                    win2.request_size_change(space, win1_loc, win1_size);
                }
                Slice::Below => {
                    let height_partition = win1_size.h / 2;

                    win1.request_size_change(
                        space,
                        win1_loc,
                        (win1_size.w, i32::max(win1_size.h - height_partition, 40)).into(),
                    );

                    win1_loc = (win1_loc.x, win1_loc.y + (win1_size.h - height_partition)).into();
                    win1_size = (win1_size.w, i32::max(height_partition, 40)).into();

                    win2.request_size_change(space, win1_loc, win1_size);
                }
            }
        }
    }
}

fn spiral(
    windows: Vec<WindowElement>,
    space: &mut Space<WindowElement>,
    rect: Rectangle<i32, Logical>,
) {
    let size = rect.size;
    let loc = rect.loc;

    let mut iter = windows.windows(2).peekable();

    if iter.peek().is_none() {
        if let Some(window) = windows.first() {
            window.request_size_change(space, loc, size);
        }
    } else {
        let mut win1_loc = loc;
        let mut win1_size = size;

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
                        space,
                        (win1_loc.x, win1_loc.y + height_partition).into(),
                        (win1_size.w, i32::max(win1_size.h - height_partition, 40)).into(),
                    );

                    win1_size = (win1_size.w, i32::max(height_partition, 40)).into();
                    win2.request_size_change(space, win1_loc, win1_size);
                }
                Slice::Below => {
                    let height_partition = win1_size.h / 2;

                    win1.request_size_change(
                        space,
                        win1_loc,
                        (win1_size.w, win1_size.h - i32::max(height_partition, 40)).into(),
                    );

                    win1_loc = (win1_loc.x, win1_loc.y + (win1_size.h - height_partition)).into();
                    win1_size = (win1_size.w, i32::max(height_partition, 40)).into();
                    win2.request_size_change(space, win1_loc, win1_size);
                }
                Slice::Left => {
                    let width_partition = win1_size.w / 2;

                    win1.request_size_change(
                        space,
                        (win1_loc.x + width_partition, win1_loc.y).into(),
                        (win1_size.w - width_partition, i32::max(win1_size.h, 40)).into(),
                    );

                    win1_size = (width_partition, i32::max(win1_size.h, 40)).into();
                    win2.request_size_change(space, win1_loc, win1_size);
                }
                Slice::Right => {
                    let width_partition = win1_size.w / 2;

                    win1.request_size_change(
                        space,
                        win1_loc,
                        (win1_size.w - width_partition, i32::max(win1_size.h, 40)).into(),
                    );

                    win1_loc = (win1_loc.x + (win1_size.w - width_partition), win1_loc.y).into();
                    win1_size = (width_partition, i32::max(win1_size.h, 40)).into();
                    win2.request_size_change(space, win1_loc, win1_size);
                }
            }
        }
    }
}

fn corner(
    layout: &Layout,
    windows: Vec<WindowElement>,
    space: &mut Space<WindowElement>,
    rect: Rectangle<i32, Logical>,
) {
    let size = rect.size;
    let loc = rect.loc;

    match windows.len() {
        0 => (),
        1 => {
            windows[0].request_size_change(space, loc, size);
        }
        2 => {
            windows[0].request_size_change(space, loc, (size.w / 2, size.h).into());

            windows[1].request_size_change(
                space,
                (loc.x + size.w / 2, loc.y).into(),
                (size.w / 2, size.h).into(),
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
                space,
                match layout {
                    Layout::CornerTopLeft => (loc.x, loc.y),
                    Layout::CornerTopRight => (loc.x + size.w - size.w / div_factor, loc.y),
                    Layout::CornerBottomLeft => (loc.x, loc.y + size.h - size.h / div_factor),
                    Layout::CornerBottomRight => (
                        loc.x + size.w - size.w / div_factor,
                        loc.y + size.h - size.h / div_factor,
                    ),
                    _ => unreachable!(),
                }
                .into(),
                (size.w / div_factor, size.h / div_factor).into(),
            );

            let vert_stack_count = vert_stack.len();

            let height = size.h as f32 / vert_stack_count as f32;
            let mut y_s = vec![];
            for i in 0..vert_stack_count {
                y_s.push((i as f32 * height).round() as i32);
            }
            let heights = y_s
                .windows(2)
                .map(|pair| pair[1] - pair[0])
                .chain(vec![size.h - y_s.last().expect("vec was empty")])
                .collect::<Vec<_>>();

            for (i, win) in vert_stack.iter().enumerate() {
                win.request_size_change(
                    space,
                    (
                        match layout {
                            Layout::CornerTopLeft | Layout::CornerBottomLeft => size.w / 2 + loc.x,
                            Layout::CornerTopRight | Layout::CornerBottomRight => loc.x,
                            _ => unreachable!(),
                        },
                        y_s[i] + loc.y,
                    )
                        .into(),
                    (size.w / 2, i32::max(heights[i], 40)).into(),
                );
            }

            let horiz_stack_count = horiz_stack.len();

            let width = size.w as f32 / 2.0 / horiz_stack_count as f32;
            let mut x_s = vec![];
            for i in 0..horiz_stack_count {
                x_s.push((i as f32 * width).round() as i32);
            }
            let widths = x_s
                .windows(2)
                .map(|pair| pair[1] - pair[0])
                .chain(vec![size.w / 2 - x_s.last().expect("vec was empty")])
                .collect::<Vec<_>>();

            for (i, win) in horiz_stack.iter().enumerate() {
                win.request_size_change(
                    space,
                    match layout {
                        Layout::CornerTopLeft => (x_s[i] + loc.x, loc.y + size.h / 2),
                        Layout::CornerTopRight => (x_s[i] + loc.x + size.w / 2, loc.y + size.h / 2),
                        Layout::CornerBottomLeft => (x_s[i] + loc.x, loc.y),
                        Layout::CornerBottomRight => (x_s[i] + loc.x + size.w / 2, loc.y),
                        _ => unreachable!(),
                    }
                    .into(),
                    (i32::max(widths[i], 1), size.h / 2).into(),
                );
            }
        }
    }
}

fn filter_windows(windows: &[WindowElement], tags: Vec<Tag>) -> Vec<WindowElement> {
    windows
        .iter()
        .filter(|window| window.with_state(|state| state.floating.is_tiled()))
        .filter(|window| {
            window.with_state(|state| {
                for tag in state.tags.iter() {
                    if tags.iter().any(|tg| tg == tag) {
                        return true;
                    }
                }
                false
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

        // TODO: don't use the focused output, use the outputs the two windows are on
        let output = self
            .focus_state
            .focused_output
            .clone()
            .expect("no focused output");
        self.re_layout(&output);
    }
}
