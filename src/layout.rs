// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::atomic::AtomicU32;

use smithay::{
    backend::renderer::utils::with_renderer_surface_state,
    desktop::layer_map_for_output,
    output::Output,
    reexports::wayland_server::Resource,
    utils::{Logical, Point, Rectangle, Serial, Size},
    wayland::compositor::{self, CompositorHandler},
};

use crate::{
    state::{State, WithState},
    window::{
        blocker::TiledWindowBlocker,
        window_state::{FloatingOrTiled, FullscreenOrMaximized, LocationRequestState},
        WindowElement,
    },
};

impl State {
    /// Compute the positions and sizes of tiled windows on
    /// `output` according to the provided [`Layout`].
    fn tile_windows(&self, output: &Output, windows: Vec<WindowElement>, layout: Layout) {
        let Some(rect) = self.space.output_geometry(output).map(|op_geo| {
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

        match layout {
            Layout::MasterStack => master_stack(windows, rect),
            Layout::Dwindle => dwindle(windows, rect),
            Layout::Spiral => spiral(windows, rect),
            layout @ (Layout::CornerTopLeft
            | Layout::CornerTopRight
            | Layout::CornerBottomLeft
            | Layout::CornerBottomRight) => corner(&layout, windows, rect),
        }
    }

    /// Compute tiled window locations and sizes, resize maximized and fullscreen windows correctly,
    /// and send configures and that cool stuff.
    pub fn update_windows(&mut self, output: &Output) {
        // HACK: With the blocker implementation, if I opened up a bunch of windows quickly they
        // |     would freeze up.
        // |     So instead of being smart and figuring something out I instead decided "f*** it"
        // |     and stuck a static here to detect if update_windows was called again, causing
        // |     previous blockers to be unblocked.
        static UPDATE_COUNT: AtomicU32 = AtomicU32::new(0);
        UPDATE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let current_update_count = UPDATE_COUNT.load(std::sync::atomic::Ordering::Relaxed);

        tracing::debug!("Updating windows");
        let Some(layout) =
            output.with_state(|state| state.focused_tags().next().map(|tag| tag.layout()))
        else {
            return;
        };

        let (windows_on_foc_tags, mut windows_not_on_foc_tags): (Vec<_>, _) =
            output.with_state(|state| {
                let focused_tags = state.focused_tags().collect::<Vec<_>>();
                self.windows
                    .iter()
                    .filter(|win| !win.is_x11_override_redirect())
                    .cloned()
                    .partition(|win| {
                        win.with_state(|state| {
                            state.tags.iter().any(|tg| focused_tags.contains(&tg))
                        })
                    })
            });

        // Don't unmap windows that aren't on `output` (that would clear all other monitors)
        windows_not_on_foc_tags.retain(|win| win.output(self) == Some(output.clone()));

        let tiled_windows = windows_on_foc_tags
            .iter()
            .filter(|win| {
                win.with_state(|state| {
                    state.floating_or_tiled.is_tiled() && state.fullscreen_or_maximized.is_neither()
                })
            })
            .cloned()
            .collect::<Vec<_>>();

        self.tile_windows(output, tiled_windows, layout);

        let output_geo = self.space.output_geometry(output).expect("no output geo");
        for window in windows_on_foc_tags.iter() {
            match window.with_state(|state| state.fullscreen_or_maximized) {
                FullscreenOrMaximized::Fullscreen => {
                    window.change_geometry(output_geo);
                }
                FullscreenOrMaximized::Maximized => {
                    let map = layer_map_for_output(output);
                    let geo = if map.layers().next().is_none() {
                        // INFO: Sometimes the exclusive zone is some weird number that doesn't match the
                        // |     output res, even when there are no layer surfaces mapped. In this case, we
                        // |     just return the output geometry.
                        output_geo
                    } else {
                        let zone = map.non_exclusive_zone();
                        tracing::debug!("non_exclusive_zone is {zone:?}");
                        Rectangle::from_loc_and_size(output_geo.loc + zone.loc, zone.size)
                    };
                    window.change_geometry(geo);
                }
                FullscreenOrMaximized::Neither => {
                    if let FloatingOrTiled::Floating(rect) =
                        window.with_state(|state| state.floating_or_tiled)
                    {
                        window.change_geometry(rect);
                    }
                }
            }
        }

        let mut pending_wins = Vec::<(WindowElement, Serial)>::new();
        let mut non_pending_wins = Vec::<(Point<_, _>, WindowElement)>::new();

        // TODO: completely refactor how tracking state changes works
        for window in windows_on_foc_tags.iter() {
            window.with_state(|state| {
                if let LocationRequestState::Sent(loc) = state.loc_request_state {
                    match &window {
                        WindowElement::Wayland(win) => {
                            // If the above didn't cause any change to size or other state, simply
                            // map the window.
                            let current_state = win.toplevel().current_state();
                            let is_pending = win
                                .toplevel()
                                .with_pending_state(|state| state.size != current_state.size);

                            if !is_pending {
                                tracing::debug!("No pending changes, mapping window");
                                // TODO: map win here, not down there
                                state.loc_request_state = LocationRequestState::Idle;
                                non_pending_wins.push((loc, window.clone()));
                            } else {
                                tracing::debug!("Pending changes, requesting commit");
                                let serial = win.toplevel().send_configure();
                                state.loc_request_state =
                                    LocationRequestState::Requested(serial, loc);
                                pending_wins.push((window.clone(), serial));
                            }
                        }
                        WindowElement::X11(surface) => {
                            // already configured, just need to map
                            // maybe wait for all wayland windows to commit before mapping
                            // self.space.map_element(window.clone(), loc, false);
                            surface
                                .set_mapped(true)
                                .expect("failed to set x11 win to mapped");
                            state.loc_request_state = LocationRequestState::Idle;
                            non_pending_wins.push((loc, window.clone()));
                        }
                        WindowElement::X11OverrideRedirect(_) => {
                            // filtered out up there somewhere
                            unreachable!();
                        }
                    }
                }
            });
        }

        let tiling_blocker = TiledWindowBlocker::new(pending_wins.clone());

        for (win, _) in pending_wins.iter() {
            if let Some(surface) = win.wl_surface() {
                let client = surface
                    .client()
                    .expect("Surface has no client/is no longer alive");
                self.client_compositor_state(&client)
                    .blocker_cleared(self, &self.display_handle.clone());

                tracing::debug!("blocker cleared");
                compositor::add_blocker(&surface, tiling_blocker.clone());
            }
        }

        let pending_wins_clone = pending_wins.clone();
        let pending_wins_clone2 = pending_wins.clone();

        self.schedule(
            move |data| {
                if UPDATE_COUNT.load(std::sync::atomic::Ordering::Relaxed) > current_update_count {
                    for (win, _) in pending_wins_clone2.iter() {
                        let Some(surface) = win.wl_surface() else {
                            continue;
                        };

                        let client = surface
                            .client()
                            .expect("Surface has no client/is no longer alive");
                        data.state
                            .client_compositor_state(&client)
                            .blocker_cleared(&mut data.state, &data.display_handle);

                        tracing::debug!("blocker cleared");
                    }
                }
                tiling_blocker.ready()
                    && pending_wins_clone2.iter().all(|(win, _)| {
                        if let Some(surface) = win.wl_surface() {
                            with_renderer_surface_state(&surface, |state| state.buffer().is_some())
                        } else {
                            true
                        }
                    })
            },
            move |data| {
                for (win, _) in pending_wins_clone {
                    let Some(surface) = win.wl_surface() else {
                        continue;
                    };

                    let client = surface
                        .client()
                        .expect("Surface has no client/is no longer alive");
                    data.state
                        .client_compositor_state(&client)
                        .blocker_cleared(&mut data.state, &data.display_handle);

                    tracing::debug!("blocker cleared");
                }

                for (loc, win) in non_pending_wins {
                    data.state.space.map_element(win, loc, false);
                }
            },
        );
    }
}

// -------------------------------------------

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

fn master_stack(windows: Vec<WindowElement>, rect: Rectangle<i32, Logical>) {
    let size = rect.size;
    let loc = rect.loc;

    let master = windows.first();
    let stack = windows.iter().skip(1);

    let Some(master) = master else { return };

    let stack_count = stack.clone().count();

    if stack_count == 0 {
        // one window
        master.change_geometry(Rectangle::from_loc_and_size(loc, size));
    } else {
        let loc: Point<i32, Logical> = (loc.x, loc.y).into();
        let new_master_size: Size<i32, Logical> = (size.w / 2, size.h).into();
        master.change_geometry(Rectangle::from_loc_and_size(loc, new_master_size));

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
            win.change_geometry(Rectangle::from_loc_and_size(
                Point::from((size.w / 2 + loc.x, y_s[i] + loc.y)),
                Size::from((size.w / 2, i32::max(heights[i], 40))),
            ));
        }
    }
}

fn dwindle(windows: Vec<WindowElement>, rect: Rectangle<i32, Logical>) {
    let size = rect.size;
    let loc = rect.loc;

    let mut iter = windows.windows(2).peekable();

    if iter.peek().is_none() {
        if let Some(window) = windows.first() {
            window.change_geometry(Rectangle::from_loc_and_size(loc, size));
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

                    win1.change_geometry(Rectangle::from_loc_and_size(
                        win1_loc,
                        Size::from((win1_size.w - width_partition, i32::max(win1_size.h, 40))),
                    ));

                    win1_loc = (win1_loc.x + (win1_size.w - width_partition), win1_loc.y).into();
                    win1_size = (width_partition, i32::max(win1_size.h, 40)).into();

                    win2.change_geometry(Rectangle::from_loc_and_size(win1_loc, win1_size));
                }
                Slice::Below => {
                    let height_partition = win1_size.h / 2;

                    win1.change_geometry(Rectangle::from_loc_and_size(
                        win1_loc,
                        Size::from((win1_size.w, i32::max(win1_size.h - height_partition, 40))),
                    ));

                    win1_loc = (win1_loc.x, win1_loc.y + (win1_size.h - height_partition)).into();
                    win1_size = (win1_size.w, i32::max(height_partition, 40)).into();

                    win2.change_geometry(Rectangle::from_loc_and_size(win1_loc, win1_size));
                }
            }
        }
    }
}

fn spiral(windows: Vec<WindowElement>, rect: Rectangle<i32, Logical>) {
    let size = rect.size;
    let loc = rect.loc;

    let mut window_pairs = windows.windows(2).peekable();

    if window_pairs.peek().is_none() {
        if let Some(window) = windows.first() {
            window.change_geometry(Rectangle::from_loc_and_size(loc, size));
        }
    } else {
        let mut win1_loc = loc;
        let mut win1_size = size;

        for (i, wins) in window_pairs.enumerate() {
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

                    win1.change_geometry(Rectangle::from_loc_and_size(
                        Point::from((win1_loc.x, win1_loc.y + height_partition)),
                        Size::from((win1_size.w, i32::max(win1_size.h - height_partition, 40))),
                    ));

                    win1_size = (win1_size.w, i32::max(height_partition, 40)).into();
                    win2.change_geometry(Rectangle::from_loc_and_size(win1_loc, win1_size));
                }
                Slice::Below => {
                    let height_partition = win1_size.h / 2;

                    win1.change_geometry(Rectangle::from_loc_and_size(
                        win1_loc,
                        Size::from((win1_size.w, win1_size.h - i32::max(height_partition, 40))),
                    ));

                    win1_loc = (win1_loc.x, win1_loc.y + (win1_size.h - height_partition)).into();
                    win1_size = (win1_size.w, i32::max(height_partition, 40)).into();
                    win2.change_geometry(Rectangle::from_loc_and_size(win1_loc, win1_size));
                }
                Slice::Left => {
                    let width_partition = win1_size.w / 2;

                    win1.change_geometry(Rectangle::from_loc_and_size(
                        Point::from((win1_loc.x + width_partition, win1_loc.y)),
                        Size::from((win1_size.w - width_partition, i32::max(win1_size.h, 40))),
                    ));

                    win1_size = (width_partition, i32::max(win1_size.h, 40)).into();
                    win2.change_geometry(Rectangle::from_loc_and_size(win1_loc, win1_size));
                }
                Slice::Right => {
                    let width_partition = win1_size.w / 2;

                    win1.change_geometry(Rectangle::from_loc_and_size(
                        win1_loc,
                        Size::from((win1_size.w - width_partition, i32::max(win1_size.h, 40))),
                    ));

                    win1_loc = (win1_loc.x + (win1_size.w - width_partition), win1_loc.y).into();
                    win1_size = (width_partition, i32::max(win1_size.h, 40)).into();
                    win2.change_geometry(Rectangle::from_loc_and_size(win1_loc, win1_size));
                }
            }
        }
    }
}

fn corner(layout: &Layout, windows: Vec<WindowElement>, rect: Rectangle<i32, Logical>) {
    let size = rect.size;
    let loc = rect.loc;

    match windows.len() {
        0 => (),
        1 => {
            windows[0].change_geometry(rect);
        }
        2 => {
            windows[0].change_geometry(Rectangle::from_loc_and_size(
                loc,
                Size::from((size.w / 2, size.h)),
            ));

            windows[1].change_geometry(Rectangle::from_loc_and_size(
                Point::from((loc.x + size.w / 2, loc.y)),
                Size::from((size.w / 2, size.h)),
            ));
        }
        _ => {
            let mut windows = windows.into_iter();
            let Some(corner) = windows.next() else { unreachable!() };

            let mut horiz_stack = Vec::<WindowElement>::new();
            let mut vert_stack = Vec::<WindowElement>::new();

            for (i, win) in windows.enumerate() {
                if i % 2 == 0 {
                    horiz_stack.push(win);
                } else {
                    vert_stack.push(win);
                }
            }

            let div_factor = 2;

            corner.change_geometry(Rectangle::from_loc_and_size(
                Point::from(match layout {
                    Layout::CornerTopLeft => (loc.x, loc.y),
                    Layout::CornerTopRight => (loc.x + size.w - size.w / div_factor, loc.y),
                    Layout::CornerBottomLeft => (loc.x, loc.y + size.h - size.h / div_factor),
                    Layout::CornerBottomRight => (
                        loc.x + size.w - size.w / div_factor,
                        loc.y + size.h - size.h / div_factor,
                    ),
                    _ => unreachable!(),
                }),
                Size::from((size.w / div_factor, size.h / div_factor)),
            ));

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
                win.change_geometry(Rectangle::from_loc_and_size(
                    Point::from((
                        match layout {
                            Layout::CornerTopLeft | Layout::CornerBottomLeft => size.w / 2 + loc.x,
                            Layout::CornerTopRight | Layout::CornerBottomRight => loc.x,
                            _ => unreachable!(),
                        },
                        y_s[i] + loc.y,
                    )),
                    Size::from((size.w / 2, i32::max(heights[i], 40))),
                ));
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
                win.change_geometry(Rectangle::from_loc_and_size(
                    Point::from(match layout {
                        Layout::CornerTopLeft => (x_s[i] + loc.x, loc.y + size.h / 2),
                        Layout::CornerTopRight => (x_s[i] + loc.x + size.w / 2, loc.y + size.h / 2),
                        Layout::CornerBottomLeft => (x_s[i] + loc.x, loc.y),
                        Layout::CornerBottomRight => (x_s[i] + loc.x + size.w / 2, loc.y),
                        _ => unreachable!(),
                    }),
                    Size::from((i32::max(widths[i], 1), size.h / 2)),
                ));
            }
        }
    }
}

impl State {
    /// Swaps two windows in the main window vec and updates all windows.
    pub fn swap_window_positions(&mut self, win1: &WindowElement, win2: &WindowElement) {
        let win1_index = self.windows.iter().position(|win| win == win1);
        let win2_index = self.windows.iter().position(|win| win == win2);

        if let (Some(first), Some(second)) = (win1_index, win2_index) {
            self.windows.swap(first, second);
            if let Some(output) = win1.output(self) {
                self.update_windows(&output);
            }
        }
    }
}
