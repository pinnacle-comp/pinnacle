use std::cell::RefCell;

use smithay::{reexports::wayland_server::protocol::wl_surface::WlSurface, wayland::compositor};

use crate::{backend::Backend, state::State};

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

pub fn toggle_floating<B: Backend>(state: &mut State<B>, wl_surface: &WlSurface) {
    WindowState::with_state(wl_surface, |window_state| {
        let window = state
            .space
            .elements()
            .find(|w| w.toplevel().wl_surface() == wl_surface)
            .unwrap()
            .clone();
        match window_state.floating {
            Float::NotFloating(prev_loc_and_size) => {
                if let Some((prev_loc, prev_size)) = prev_loc_and_size {
                    window.toplevel().with_pending_state(|state| {
                        state.size = Some(prev_size);
                    });

                    window.toplevel().send_pending_configure();

                    state.space.map_element(window, prev_loc, false); // TODO: should it activate?
                }

                window_state.floating = Float::Floating
            }
            Float::Floating => {
                // TODO: recompute all non-floating window positions

                window_state.floating = Float::NotFloating(Some((
                    state.space.element_location(&window).unwrap(), // We get the location this way
                    // because window.geometry().loc doesn't seem to be the actual location
                    window.geometry().size,
                )));
            }
        }
    })
}
