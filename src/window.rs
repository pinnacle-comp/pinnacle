// SPDX-License-Identifier: GPL-3.0-or-later

pub mod rules;

use std::{cell::RefCell, ops::Deref};

use indexmap::IndexSet;
use smithay::{
    backend::renderer::utils::with_renderer_surface_state,
    desktop::{space::SpaceElement, Window, WindowSurface},
    output::Output,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{IsAlive, Logical, Point, Rectangle, Serial, Size},
    wayland::{
        compositor,
        seat::WaylandFocus,
        shell::xdg::{SurfaceCachedState, XdgToplevelSurfaceData},
    },
    xwayland::xwm::WmWindowType,
};
use tracing::{error, warn};

use crate::state::{Pinnacle, State, WithState};

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
    pub fn change_geometry(
        &self,
        new_loc: Option<Point<f64, Logical>>,
        new_size: Size<i32, Logical>,
    ) {
        let _span = tracy_client::span!("WindowElement::change_geometry");

        match self.0.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                toplevel.with_pending_state(|state| {
                    state.size = Some(new_size);
                });
            }
            WindowSurface::X11(surface) => {
                if !surface.is_override_redirect() {
                    // FIXME: rounded loc here
                    surface
                        .configure(Rectangle::new(
                            new_loc.unwrap_or_default().to_i32_round(), // FIXME: unwrap_or_default
                            new_size,
                        ))
                        .expect("failed to configure x11 win");
                }
            }
        }
    }

    /// Get this window's class (app id in Wayland but hey old habits die hard).
    pub fn class(&self) -> Option<String> {
        let _span = tracy_client::span!("WindowElement::class");

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
        let _span = tracy_client::span!("WindowElement::title");

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
        let _span = tracy_client::span!("WindowElement::close");

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
        let _span = tracy_client::span!("WindowElement::output");
        self.with_state(|st| st.tags.first().and_then(|tag| tag.output(pinnacle)))
    }

    /// Returns whether or not this window has an active tag.
    ///
    /// RefCell Safety: This calls `with_state` on `self`.
    pub fn is_on_active_tag(&self) -> bool {
        let _span = tracy_client::span!("WindowElement::is_on_active_tag");
        self.with_state(|state| state.tags.iter().any(|tag| tag.active()))
    }

    pub fn is_on_active_tag_on_output(&self, output: &Output) -> bool {
        let _span = tracy_client::span!("WindowElement::is_on_active_tag_on_output");

        let win_tags = self.with_state(|state| state.tags.clone());
        output.with_state(|state| {
            state
                .focused_tags()
                .cloned()
                .collect::<IndexSet<_>>()
                .intersection(&win_tags)
                .next()
                .is_some()
        })
    }

    pub fn is_x11_override_redirect(&self) -> bool {
        matches!(self.x11_surface(), Some(surface) if surface.is_override_redirect())
    }

    /// Marks the currently acked configure as committed.
    pub fn mark_serial_as_committed(&self) {
        let _span = tracy_client::span!("WindowElement::mark_serial_as_committed");

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
        let _span = tracy_client::span!("WindowElement: WithState::with_state");

        let state = self
            .user_data()
            .get_or_insert(|| RefCell::new(WindowElementState::new()));

        func(&state.borrow())
    }

    fn with_state_mut<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&mut Self::State) -> T,
    {
        let _span = tracy_client::span!("WindowElement: WithState::with_state_mut");

        let state = self
            .user_data()
            .get_or_insert(|| RefCell::new(WindowElementState::new()));

        func(&mut state.borrow_mut())
    }
}

impl Pinnacle {
    /// Returns the [Window] associated with a given [WlSurface].
    pub fn window_for_surface(&self, surface: &WlSurface) -> Option<WindowElement> {
        let _span = tracy_client::span!("Pinnacle::window_for_surface");

        self.windows
            .iter()
            .find(|&win| win.wl_surface().is_some_and(|surf| &*surf == surface))
            .cloned()
    }

    /// [`Self::window_for_surface`] but for windows that don't have a buffer.
    pub fn unmapped_window_for_surface(&self, surface: &WlSurface) -> Option<WindowElement> {
        let _span = tracy_client::span!("Pinnacle::unmapped_window_for_surface");

        self.unmapped_windows
            .iter()
            .find(|&win| win.wl_surface().is_some_and(|surf| &*surf == surface))
            .cloned()
    }

    /// Removes a window from the main window vec, z_index stack, and focus stacks.
    ///
    /// If `unmap` is true the window has become unmapped and will be pushed to `unmapped_windows`.
    pub fn remove_window(&mut self, window: &WindowElement, unmap: bool) {
        let _span = tracy_client::span!("Pinnacle::remove_window");

        self.windows.retain(|win| win != window);
        self.unmapped_windows.retain(|win| win != window);
        if unmap {
            self.unmapped_windows.push(window.clone());
        }

        self.z_index_stack.retain(|win| win != window);

        for output in self.outputs.keys() {
            output.with_state_mut(|state| state.focus_stack.stack.retain(|win| win != window));
        }

        self.space.unmap_elem(window);
    }

    /// Places a window on an output by setting the window's tags to the output's
    /// currently active tags.
    ///
    /// Additionally sets the window as the output's current keyboard-focused window as well as removing it
    /// from all other outputs' keyboard focus stack.
    pub fn place_window_on_output(&self, window: &WindowElement, output: &Output) {
        let _span = tracy_client::span!("Pinnacle::place_window_on_output");

        window.with_state_mut(|state| {
            state.tags = output.with_state(|state| {
                let output_tags = state.focused_tags().cloned().collect::<IndexSet<_>>();
                if !output_tags.is_empty() {
                    output_tags
                } else if let Some(first_tag) = state.tags.first() {
                    std::iter::once(first_tag.clone()).collect()
                } else {
                    IndexSet::new()
                }
            });

            tracing::debug!(
                "Placed window on {} with tags {:?}",
                output.name(),
                state.tags
            );
        });

        for op in self.outputs.keys() {
            op.with_state_mut(|state| state.focus_stack.stack.retain(|win| win != window));
        }

        output.with_state_mut(|state| {
            state.focus_stack.set_focus(window.clone());
        });
    }
}

impl State {
    /// Maps a window it it's floating, or requests a layout otherwise.
    pub fn map_new_window(&mut self, window: &WindowElement) {
        let _span = tracy_client::span!("State::map_new_window");

        self.pinnacle
            .raise_window(window.clone(), window.is_on_active_tag());

        if should_float(window) {
            window.with_state_mut(|state| {
                state.window_state.set_floating(true);
            });
        }

        self.pinnacle.update_window_state(window);
        if let Some(toplevel) = window.toplevel() {
            toplevel.send_pending_configure();
        }

        let Some(output) = window.output(&self.pinnacle) else {
            // FIXME: If the floating window has no tags for whatever reason, it will never map
            return;
        };

        output.with_state_mut(|state| state.focus_stack.set_focus(window.clone()));
        self.update_keyboard_focus(&output);

        if let Some(maybe_loc) = window.with_state(|state| {
            state
                .window_state
                .is_floating()
                .then_some(state.floating_loc)
        }) {
            let output_geo = self
                .pinnacle
                .space
                .output_geometry(&output)
                .unwrap_or_default();

            let loc = maybe_loc.map(|loc| loc.to_i32_round()).unwrap_or_else(|| {
                let size = window.geometry().size;
                let centered_loc = Point::from((
                    output_geo.loc.x + output_geo.size.w / 2 - size.w / 2,
                    output_geo.loc.y + output_geo.size.h / 2 - size.h / 2,
                ));
                centered_loc
            });

            window.with_state_mut(|state| state.floating_loc = Some(loc.to_f64()));

            self.pinnacle.space.map_element(window.clone(), loc, true);
        } else {
            self.capture_snapshots_on_output(&output, []);
            self.pinnacle.begin_layout_transaction(&output);
            self.pinnacle.request_layout(&output);
        }

        // It seems wlcs needs immediate frame sends for client tests to work
        #[cfg(feature = "testing")]
        window.send_frame(
            &output,
            self.pinnacle.clock.now(),
            Some(std::time::Duration::ZERO),
            |_, _| None,
        );

        self.schedule_render(&output);
    }
}

fn should_float(window: &WindowElement) -> bool {
    match window.underlying_surface() {
        WindowSurface::Wayland(toplevel) => {
            let has_parent = toplevel.parent().is_some();

            let (min_size, max_size) = compositor::with_states(toplevel.wl_surface(), |states| {
                let mut guard = states.cached_state.get::<SurfaceCachedState>();
                let state = guard.current();
                (state.min_size, state.max_size)
            });
            let requests_constrained_size = min_size.w > 0
                && min_size.h > 0
                && (min_size.w == max_size.w || min_size.h == max_size.h);

            let should_float = has_parent || requests_constrained_size;
            should_float
        }
        // Logic from `wants_floating` in sway/desktop/xwayland.c
        WindowSurface::X11(surface) => {
            let is_popup_by_type = surface.window_type().is_some_and(|typ| {
                matches!(
                    typ,
                    WmWindowType::Dialog
                        | WmWindowType::Utility
                        | WmWindowType::Toolbar
                        | WmWindowType::Splash
                )
            });

            let requests_constrained_size = surface.size_hints().is_some_and(|size_hints| {
                let Some((min_w, min_h)) = size_hints.min_size else {
                    return false;
                };
                let Some((max_w, max_h)) = size_hints.max_size else {
                    return false;
                };
                min_w > 0 && min_h > 0 && (min_w == max_w || min_h == max_h)
            });

            let should_float = surface.is_popup() || is_popup_by_type || requests_constrained_size;
            should_float
        }
    }
}

pub fn is_window_mapped(window: &WindowElement) -> bool {
    match window.underlying_surface() {
        WindowSurface::Wayland(toplevel) => {
            with_renderer_surface_state(toplevel.wl_surface(), |state| state.buffer().is_some())
                .unwrap_or_default()
        }
        WindowSurface::X11(surface) => surface.is_mapped(),
    }
}
