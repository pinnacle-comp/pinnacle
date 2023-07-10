// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use smithay::{
    desktop::Window, reexports::wayland_server::protocol::wl_surface::WlSurface,
    wayland::seat::WaylandFocus,
};

use crate::{
    backend::Backend,
    state::{State, WithState},
};

use self::window_state::{Float, WindowId};

pub mod window_state;

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
    window.with_state(|window_state| {
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
    let render = output.with_state(|op_state| {
        state
            .windows
            .iter()
            .cloned()
            .filter(|win| {
                win.with_state(|win_state| {
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
