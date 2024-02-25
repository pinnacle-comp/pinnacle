// SPDX-License-Identifier: GPL-3.0-or-later

pub mod rules;

use std::{cell::RefCell, ops::Deref};

use smithay::{
    desktop::{space::SpaceElement, Window, WindowSurface, WindowSurfaceType},
    output::Output,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{IsAlive, Logical, Point, Rectangle},
    wayland::{compositor, seat::WaylandFocus, shell::xdg::XdgToplevelSurfaceData},
};

use crate::{
    focus::PointerFocusTarget,
    state::{State, WithState},
};

use self::window_state::WindowElementState;

pub mod window_state;

#[derive(Debug, Clone, PartialEq)]
pub struct WindowElement {
    window: Window,
}

impl Deref for WindowElement {
    type Target = Window;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

impl WindowElement {
    pub fn new(window: Window) -> Self {
        Self { window }
    }

    /// Send a geometry change without mapping windows or sending
    /// configures to Wayland windows.
    ///
    /// Xwayland windows will still receive a configure.
    ///
    /// RefCell Safety: This method uses a [`RefCell`] on this window.
    // TODO: ^ does that make things flicker?
    pub fn change_geometry(&self, new_geo: Rectangle<i32, Logical>) {
        match self.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                toplevel.with_pending_state(|state| {
                    state.size = Some(new_geo.size);
                });
            }
            WindowSurface::X11(surface) => {
                if !surface.is_override_redirect() {
                    surface
                        .configure(new_geo)
                        .expect("failed to configure x11 win");
                }
            }
        }

        self.with_state(|state| {
            state.target_loc = Some(new_geo.loc);
        });
    }

    pub fn class(&self) -> Option<String> {
        match self.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                compositor::with_states(toplevel.wl_surface(), |states| {
                    states
                        .data_map
                        .get::<XdgToplevelSurfaceData>()
                        .expect("XdgToplevelSurfaceData wasn't in surface's data map")
                        .lock()
                        .expect("Failed to lock Mutex<XdgToplevelSurfaceData>")
                        .app_id
                        .clone()
                })
            }
            WindowSurface::X11(surface) => Some(surface.class()),
        }
    }

    pub fn title(&self) -> Option<String> {
        match self.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                compositor::with_states(toplevel.wl_surface(), |states| {
                    states
                        .data_map
                        .get::<XdgToplevelSurfaceData>()
                        .expect("XdgToplevelSurfaceData wasn't in surface's data map")
                        .lock()
                        .expect("Failed to lock Mutex<XdgToplevelSurfaceData>")
                        .title
                        .clone()
                })
            }
            WindowSurface::X11(surface) => Some(surface.title()),
        }
    }

    /// Get the output this window is on.
    ///
    /// This method gets the first tag the window has and returns its output.
    ///
    /// RefCell Safety: This method uses a [`RefCell`] on this window and every mapped output.
    pub fn output(&self, state: &State) -> Option<Output> {
        self.with_state(|st| st.tags.first().and_then(|tag| tag.output(state)))
    }

    /// Returns whether or not this window has an active tag.
    ///
    /// RefCell Safety: This uses RefCells on both `self` and everything in `outputs`.
    pub fn is_on_active_tag<'a>(&self, outputs: impl IntoIterator<Item = &'a Output>) -> bool {
        let tags = outputs
            .into_iter()
            .flat_map(|op| op.with_state(|state| state.focused_tags().cloned().collect::<Vec<_>>()))
            .collect::<Vec<_>>();

        self.with_state(|state| {
            state
                .tags
                .iter()
                .any(|tag| tags.iter().any(|tag2| tag == tag2))
        })
    }

    /// Place this window on the given output, giving it the output's focused tags.
    ///
    /// RefCell Safety: Uses refcells on both the window and the output.
    pub fn place_on_output(&self, output: &Output) {
        self.with_state(|state| {
            state.tags = output.with_state(|state| {
                let output_tags = state.focused_tags().cloned().collect::<Vec<_>>();
                if !output_tags.is_empty() {
                    output_tags
                } else if let Some(first_tag) = state.tags.first() {
                    vec![first_tag.clone()]
                } else {
                    vec![]
                }
            });

            tracing::debug!(
                "Placed window on {} with tags {:?}",
                output.name(),
                state.tags
            );
        });
    }

    pub fn is_x11_override_redirect(&self) -> bool {
        matches!(
            self.window.underlying_surface(),
            WindowSurface::X11(surface) if surface.is_override_redirect()
        )
    }

    /// Return the window-owned surface under `location` for this window along its location
    /// **relative to this window**
    ///
    /// `location` should be relative to this window's origin.
    pub fn surface_under(
        &self,
        location: Point<f64, Logical>,
        window_type: WindowSurfaceType,
    ) -> Option<(PointerFocusTarget, Point<i32, Logical>)> {
        let surface_under = self.window.surface_under(location, window_type);
        match self.window.underlying_surface() {
            WindowSurface::Wayland(_) => {
                surface_under.map(|(surface, loc)| (PointerFocusTarget::WlSurface(surface), loc))
            }
            WindowSurface::X11(surf) => {
                surface_under.map(|(_, loc)| (PointerFocusTarget::X11Surface(surf.clone()), loc))
            }
        }
    }
}

impl IsAlive for WindowElement {
    fn alive(&self) -> bool {
        self.window.alive()
    }
}

impl SpaceElement for WindowElement {
    fn bbox(&self) -> Rectangle<i32, Logical> {
        self.window.bbox()
    }

    fn is_in_input_region(&self, point: &smithay::utils::Point<f64, Logical>) -> bool {
        self.window.is_in_input_region(point)
    }

    fn set_activate(&self, activated: bool) {
        self.window.set_activate(activated)
    }

    fn output_enter(&self, output: &Output, overlap: Rectangle<i32, Logical>) {
        self.window.output_enter(output, overlap)
    }

    fn output_leave(&self, output: &Output) {
        self.window.output_leave(output)
    }
}

impl WithState for WindowElement {
    type State = WindowElementState;

    fn with_state<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&mut Self::State) -> T,
    {
        let state = self
            .user_data()
            .get_or_insert(|| RefCell::new(WindowElementState::new()));

        func(&mut state.borrow_mut())
    }
}

impl State {
    /// Returns the [Window] associated with a given [WlSurface].
    pub fn window_for_surface(&self, surface: &WlSurface) -> Option<WindowElement> {
        self.space
            .elements()
            .find(|window| window.wl_surface().map(|s| s == *surface).unwrap_or(false))
            .cloned()
            .or_else(|| {
                self.windows
                    .iter()
                    .find(|&win| win.wl_surface().is_some_and(|surf| &surf == surface))
                    .cloned()
            })
    }
}
