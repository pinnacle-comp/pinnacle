// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::cell::RefCell;

use smithay::{
    desktop::Window,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    wayland::{compositor, seat::WaylandFocus},
};

use crate::{backend::Backend, output::OutputState, state::State};

use self::window_state::{Float, WindowId, WindowState};

pub mod window_state;

// TODO: maybe get rid of this and move the fn into resize_surface state because it's the only user
pub trait SurfaceState: Default + 'static {
    /// Access the [`SurfaceState`] associated with a [`WlSurface`].
    ///
    /// # Panics
    ///
    /// This function will panic if you use it within itself due to the use of a [`RefCell`].
    fn with_state<F, T>(wl_surface: &WlSurface, function: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        compositor::with_states(wl_surface, |states| {
            states.data_map.insert_if_missing(RefCell::<Self>::default);
            let state = states
                .data_map
                .get::<RefCell<Self>>()
                .expect("This should never happen");

            function(&mut state.borrow_mut())
        })
    }
}

impl<B: Backend> State<B> {
    /// Returns the [Window] associated with a given [WlSurface].
    pub fn window_for_surface(&self, surface: &WlSurface) -> Option<Window> {
        self.space
            .elements()
            .find(|window| window.wl_surface().map(|s| s == *surface).unwrap_or(false))
            .cloned()
            .or_else(|| {
                self.windows
                    .iter()
                    .find(|&win| win.toplevel().wl_surface() == surface)
                    .cloned()
            })
    }
}

/// Toggle a window's floating status.
pub fn toggle_floating<B: Backend>(state: &mut State<B>, window: &Window) {
    WindowState::with(window, |window_state| {
        match window_state.floating {
            Float::Tiled(prev_loc_and_size) => {
                if let Some((prev_loc, prev_size)) = prev_loc_and_size {
                    window.toplevel().with_pending_state(|state| {
                        state.size = Some(prev_size);
                    });

                    window.toplevel().send_pending_configure();

                    state.space.map_element(window.clone(), prev_loc, false); // TODO: should it activate?
                }

                window_state.floating = Float::Floating;
            }
            Float::Floating => {
                window_state.floating = Float::Tiled(Some((
                    // We get the location this way because window.geometry().loc
                    // doesn't seem to be the actual location
                    state.space.element_location(window).unwrap(),
                    window.geometry().size,
                )));
            }
        }
    });

    state.re_layout();

    let output = state.focus_state.focused_output.as_ref().unwrap();
    let render = OutputState::with(output, |op_state| {
        state
            .windows
            .iter()
            .cloned()
            .filter(|win| {
                WindowState::with(win, |win_state| {
                    if win_state.floating.is_floating() {
                        return true;
                    }
                    for tag_id in win_state.tags.iter() {
                        if op_state.focused_tags().any(|tag| &tag.id == tag_id) {
                            return true;
                        }
                    }
                    false
                })
            })
            .collect::<Vec<_>>()
    });

    let clone = window.clone();
    state.schedule_on_commit(render, move |data| {
        data.state.space.raise_element(&clone, true);
    });
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct WindowProperties {
    pub id: WindowId,
    pub app_id: Option<String>,
    pub title: Option<String>,
    /// Width and height
    pub size: (i32, i32),
    /// x and y
    pub location: (i32, i32),
    pub floating: bool,
}
