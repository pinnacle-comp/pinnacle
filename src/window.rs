// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::cell::RefCell;

use smithay::{
    desktop::Window,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    wayland::{compositor, seat::WaylandFocus},
};

use crate::{
    backend::Backend, layout::Layout, state::State, window::window_state::WindowResizeState,
};

use self::window_state::{Float, WindowState};

pub mod window_state;

pub trait SurfaceState: Default + 'static {
    /// Access the [SurfaceState] associated with a [WlSurface].
    ///
    /// # Panics
    ///
    /// This function will panic if you use it within itself due to the use of a [RefCell].
    fn with_state<F, T>(wl_surface: &WlSurface, function: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        compositor::with_states(wl_surface, |states| {
            states.data_map.insert_if_missing(RefCell::<Self>::default);
            let state = states.data_map.get::<RefCell<Self>>().unwrap();

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
    }

    /// Swap the positions and sizes of two windows.
    pub fn swap_window_positions(&mut self, win1: &Window, win2: &Window) {
        // FIXME: moving the mouse quickly will break swapping

        let win1_loc = self.space.element_location(win1).unwrap(); // TODO: handle unwraps
        let win2_loc = self.space.element_location(win2).unwrap();
        let win1_geo = win1.geometry();
        let win2_geo = win2.geometry();
        // tracing::info!("win1: {:?}, {:?}", win1_loc, win1_geo);
        // tracing::info!("win2: {:?}, {:?}", win2_loc, win2_geo);

        win1.toplevel().with_pending_state(|state| {
            state.size = Some(win2_geo.size);
        });
        win2.toplevel().with_pending_state(|state| {
            state.size = Some(win1_geo.size);
        });

        let serial = win1.toplevel().send_configure();
        WindowState::with_state(win1, |state| {
            state.resize_state = WindowResizeState::WaitingForAck(serial, win2_loc);
        });

        let serial = win2.toplevel().send_configure();
        WindowState::with_state(win2, |state| {
            state.resize_state = WindowResizeState::WaitingForAck(serial, win1_loc);
        });

        // self.space.map_element(win1.clone(), win2_loc, false);
        // self.space.map_element(win2.clone(), win1_loc, false);
    }
}

/// Toggle a window's floating status.
pub fn toggle_floating<B: Backend>(state: &mut State<B>, window: &Window) {
    WindowState::with_state(window, |window_state| {
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

    let windows = state.space.elements().cloned().collect::<Vec<_>>();
    Layout::master_stack(state, windows, crate::layout::Direction::Left);
    state.space.raise_element(window, true);
}
