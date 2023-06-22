use std::cell::RefCell;

use smithay::{
    desktop::Window,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    wayland::{compositor, seat::WaylandFocus},
};

use crate::{backend::Backend, layout::Layout, state::State};

use self::window_state::{Float, WindowState};

pub mod window_state;

pub trait SurfaceState: Default + 'static {
    /// Access the [SurfaceState] associated with a [WlSurface]
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

    pub fn swap_window_positions(&mut self, win1: &Window, win2: &Window) {
        let win1_loc = self.space.element_location(win1).unwrap(); // TODO: handle unwraps
        let win2_loc = self.space.element_location(win2).unwrap();
        let win1_geo = self.space.element_geometry(win1).unwrap();
        let win2_geo = self.space.element_geometry(win2).unwrap();

        win1.toplevel().with_pending_state(|state| {
            state.size = Some(win2_geo.size);
        });
        win2.toplevel().with_pending_state(|state| {
            state.size = Some(win1_geo.size);
        });

        self.space.map_element(win1.clone(), win2_loc, false);
        self.space.map_element(win2.clone(), win1_loc, false);

        win1.toplevel().send_pending_configure();
        win2.toplevel().send_pending_configure();
    }
}

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
