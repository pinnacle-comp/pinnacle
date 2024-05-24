// SPDX-License-Identifier: GPL-3.0-or-later

pub mod rules;

use std::{cell::RefCell, ops::Deref};

use smithay::{
    desktop::{space::SpaceElement, Window, WindowSurface},
    output::Output,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{IsAlive, Logical, Point, Rectangle, Serial},
    wayland::{compositor, seat::WaylandFocus, shell::xdg::XdgToplevelSurfaceData},
};
use tracing::{error, warn};

use crate::state::{Pinnacle, WithState};

use self::window_state::WindowElementState;

pub mod window_state;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WindowElement(Window);

impl Deref for WindowElement {
    type Target = Window;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq<&WindowElement> for WindowElement {
    fn eq(&self, other: &&WindowElement) -> bool {
        self == *other
    }
}

impl PartialEq<WindowElement> for &WindowElement {
    fn eq(&self, other: &WindowElement) -> bool {
        *self == other
    }
}

impl WindowElement {
    pub fn new(window: Window) -> Self {
        Self(window)
    }

    /// Send a geometry change without mapping windows or sending
    /// configures to Wayland windows.
    ///
    /// Xwayland windows will still receive a configure.
    ///
    /// RefCell Safety: This method uses a [`RefCell`] on this window.
    pub fn change_geometry(&self, new_geo: Rectangle<i32, Logical>) {
        match self.0.underlying_surface() {
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
        self.with_state_mut(|state| {
            state.target_loc = Some(new_geo.loc);
        });
    }

    /// Get this window's class (app id in Wayland but hey old habits die hard).
    pub fn class(&self) -> Option<String> {
        match self.0.underlying_surface() {
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

    /// Get this window's title.
    pub fn title(&self) -> Option<String> {
        match self.0.underlying_surface() {
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

    /// Send a close request to this window.
    pub fn close(&self) {
        match self.underlying_surface() {
            WindowSurface::Wayland(toplevel) => toplevel.send_close(),
            WindowSurface::X11(surface) => {
                if !surface.is_override_redirect() {
                    if let Err(err) = surface.close() {
                        error!("failed to close x11 window: {err}");
                    }
                } else {
                    warn!("tried to close OR window");
                }
            }
        }
    }

    /// Get the output this window is on.
    ///
    /// This method gets the first tag the window has and returns its output.
    ///
    /// RefCell Safety: This method uses a [`RefCell`] on this window and every mapped output.
    pub fn output(&self, pinnacle: &Pinnacle) -> Option<Output> {
        self.with_state(|st| st.tags.first().and_then(|tag| tag.output(pinnacle)))
    }

    /// Returns whether or not this window has an active tag.
    ///
    /// RefCell Safety: This calls `with_state` on `self`.
    pub fn is_on_active_tag(&self) -> bool {
        self.with_state(|state| state.tags.iter().any(|tag| tag.active()))
    }

    /// Place this window on the given output, giving it the output's focused tags.
    ///
    /// RefCell Safety: Uses `with_state_mut` on the window and `with_state` on the output
    pub fn place_on_output(&self, output: &Output) {
        self.with_state_mut(|state| {
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
        matches!(self.x11_surface(), Some(surface) if surface.is_override_redirect())
    }

    /// Marks the currently acked configure as committed.
    pub fn mark_serial_as_committed(&self) {
        let Some(toplevel) = self.toplevel() else { return };
        let serial = compositor::with_states(toplevel.wl_surface(), |states| {
            states
                .data_map
                .get::<XdgToplevelSurfaceData>()
                .unwrap()
                .lock()
                .unwrap()
                .configure_serial
        });

        self.with_state_mut(|state| state.committed_serial = serial);
    }

    /// Returns whether the currently committed serial is >= the given serial.
    pub fn is_serial_committed(&self, serial: Serial) -> bool {
        match self.with_state(|state| state.committed_serial) {
            Some(committed_serial) => committed_serial >= serial,
            None => false,
        }
    }
}

impl SpaceElement for WindowElement {
    fn bbox(&self) -> Rectangle<i32, Logical> {
        self.0.bbox()
    }

    fn is_in_input_region(&self, point: &Point<f64, Logical>) -> bool {
        self.0.is_in_input_region(point)
    }

    fn set_activate(&self, activated: bool) {
        self.0.set_activate(activated)
    }

    fn output_enter(&self, output: &Output, overlap: Rectangle<i32, Logical>) {
        self.0.output_enter(output, overlap)
    }

    fn output_leave(&self, output: &Output) {
        self.0.output_leave(output)
    }

    fn geometry(&self) -> Rectangle<i32, Logical> {
        self.0.geometry()
    }

    fn z_index(&self) -> u8 {
        self.0.z_index()
    }

    fn refresh(&self) {
        self.0.refresh();
    }
}

impl IsAlive for WindowElement {
    fn alive(&self) -> bool {
        self.0.alive()
    }
}

impl WithState for WindowElement {
    type State = WindowElementState;

    fn with_state<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&Self::State) -> T,
    {
        let state = self
            .user_data()
            .get_or_insert(|| RefCell::new(WindowElementState::new()));

        func(&state.borrow())
    }

    fn with_state_mut<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&mut Self::State) -> T,
    {
        let state = self
            .user_data()
            .get_or_insert(|| RefCell::new(WindowElementState::new()));

        func(&mut state.borrow_mut())
    }
}

impl Pinnacle {
    /// Returns the [Window] associated with a given [WlSurface].
    pub fn window_for_surface(&self, surface: &WlSurface) -> Option<WindowElement> {
        self.windows
            .iter()
            .find(|&win| win.wl_surface().is_some_and(|surf| &*surf == surface))
            .cloned()
    }

    /// [`Self::window_for_surface`] but for windows that don't have a buffer.
    pub fn unmapped_window_for_surface(&self, surface: &WlSurface) -> Option<WindowElement> {
        self.unmapped_windows
            .iter()
            .find(|&win| win.wl_surface().is_some_and(|surf| &*surf == surface))
            .cloned()
    }

    /// Removes a window from the main window vec, z_index stack, and focus stacks.
    ///
    /// If `unmap` is true the window has become unmapped and will be pushed to `unmapped_windows`.
    pub fn remove_window(&mut self, window: &WindowElement, unmap: bool) {
        self.windows.retain(|win| win != window);
        self.unmapped_windows.retain(|win| win != window);
        if unmap {
            self.unmapped_windows.push(window.clone());
        }

        self.z_index_stack.retain(|win| win != window);

        for output in self.space.outputs() {
            output.with_state_mut(|state| state.focus_stack.stack.retain(|win| win != window));
        }
    }
}
