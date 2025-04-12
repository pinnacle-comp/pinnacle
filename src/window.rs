// SPDX-License-Identifier: GPL-3.0-or-later

pub mod rules;

use std::{cell::RefCell, ops::Deref};

use indexmap::IndexSet;
use rules::WindowRules;
use smithay::{
    desktop::{layer_map_for_output, space::SpaceElement, Window, WindowSurface},
    output::Output,
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_positioner::{
            Anchor, ConstraintAdjustment, Gravity,
        },
        wayland_server::protocol::wl_surface::WlSurface,
    },
    utils::{IsAlive, Logical, Point, Rectangle, Serial},
    wayland::{
        compositor,
        seat::WaylandFocus,
        shell::xdg::{PositionerState, SurfaceCachedState, XdgToplevelSurfaceData},
        xdg_activation::XdgActivationTokenData,
    },
    xwayland::xwm::WmWindowType,
};
use tracing::{error, warn};
use window_state::LayoutModeKind;

use crate::{
    state::{Pinnacle, State, WithState},
    tag::Tag,
};

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

    pub fn set_tags_to_output(&self, output: &Output) {
        self.with_state_mut(|state| {
            set_tags_to_output(&mut state.tags, output);
        });
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
    pub fn window_for_surface(&self, surface: &WlSurface) -> Option<&WindowElement> {
        let _span = tracy_client::span!("Pinnacle::window_for_surface");

        self.windows
            .iter()
            .find(|&win| win.wl_surface().is_some_and(|surf| &*surf == surface))
    }

    pub fn unmapped_window_for_surface(&self, surface: &WlSurface) -> Option<&Unmapped> {
        self.unmapped_windows.iter().find(|win| {
            win.window
                .wl_surface()
                .is_some_and(|surf| &*surf == surface)
        })
    }

    pub fn unmapped_window_for_surface_mut(
        &mut self,
        surface: &WlSurface,
    ) -> Option<&mut Unmapped> {
        self.unmapped_windows.iter_mut().find(|win| {
            win.window
                .wl_surface()
                .is_some_and(|surf| &*surf == surface)
        })
    }

    /// Removes a window from the main window vec, z_index stack, and focus stacks.
    ///
    /// If `unmap` is true the window has become unmapped and will be pushed to `unmapped_windows`.
    pub fn remove_window(&mut self, window: &WindowElement, unmap: bool) {
        let _span = tracy_client::span!("Pinnacle::remove_window");

        self.windows.retain(|win| win != window);
        self.unmapped_windows.retain(|win| win.window != window);
        if unmap {
            self.unmapped_windows.push(Unmapped {
                window: window.clone(),
                activation_token_data: None,
                window_rules: WindowRules::default(),
                awaiting_tags: false,
            });
        }

        self.z_index_stack.retain(|win| win != window);

        for output in self.outputs.keys() {
            output.with_state_mut(|state| state.focus_stack.remove(window));
        }

        self.space.unmap_elem(window);
    }

    /// Returns the parent or parent-equivalent window, if any.
    pub fn parent_window_for(&self, window: &WindowElement) -> Option<&WindowElement> {
        match window.underlying_surface() {
            WindowSurface::Wayland(toplevel) => toplevel
                .parent()
                .and_then(|parent| self.window_for_surface(&parent)),
            WindowSurface::X11(surface) => {
                let transient_for_id = surface.is_transient_for()?;
                self.windows.iter().find(|win| {
                    if let Some(surf) = win.x11_surface() {
                        surf.window_id() == transient_for_id
                    } else {
                        false
                    }
                })
            }
        }
    }
}

fn set_tags_to_output(tags: &mut IndexSet<Tag>, output: &Output) {
    *tags = output.with_state(|state| {
        let output_tags = state.focused_tags().cloned().collect::<IndexSet<_>>();
        if !output_tags.is_empty() {
            output_tags
        } else if let Some(first_tag) = state.tags.first() {
            std::iter::once(first_tag.clone()).collect()
        } else {
            IndexSet::new()
        }
    });
}

impl State {
    /// Maps an unmapped window, inserting it into the main window vec.
    ///
    /// If it's floating, this will map the window onto the space.
    /// Otherwise, it requests a layout.
    pub fn map_new_window(&mut self, unmapped: Unmapped) {
        let _span = tracy_client::span!("State::map_new_window");

        let Unmapped {
            window,
            activation_token_data: _, // TODO:
            window_rules,
            awaiting_tags,
        } = unmapped;

        assert!(!awaiting_tags, "tried to map new window without tags");

        self.pinnacle.windows.push(window.clone());

        self.pinnacle
            .raise_window(window.clone(), window.is_on_active_tag());

        if window_rules.layout_mode.is_none() && should_float(&window) {
            window.with_state_mut(|state| {
                state.layout_mode.set_floating(true);
            });
        }

        let Some(output) = window.output(&self.pinnacle) else {
            // FIXME: If the window has no tags for whatever reason, it will never map
            return;
        };

        let Some(output_geo) = self.pinnacle.space.output_geometry(&output) else {
            return;
        };

        let mut working_output_geo = layer_map_for_output(&output).non_exclusive_zone();
        working_output_geo.loc += output_geo.loc;

        let center_rect = self
            .pinnacle
            .parent_window_for(&window)
            .and_then(|parent| self.pinnacle.space.element_geometry(parent))
            .unwrap_or(working_output_geo);

        if window.with_state(|state| state.layout_mode.is_floating()) {
            let size = window.geometry().size;

            let loc = window
                .with_state(|state| state.floating_loc)
                .unwrap_or_else(|| {
                    // Attempt to center the window within its parent.
                    // If it has no parent, center it within the non-exclusive zone of its output.
                    //
                    // We use a positioner to slide the window so that it isn't off screen.

                    let positioner = PositionerState {
                        rect_size: size,
                        anchor_rect: center_rect,
                        anchor_edges: Anchor::None,
                        gravity: Gravity::None,
                        constraint_adjustment: ConstraintAdjustment::SlideX
                            | ConstraintAdjustment::SlideY,
                        offset: (0, 0).into(),
                        ..Default::default()
                    };

                    positioner
                        .get_unconstrained_geometry(working_output_geo)
                        .to_f64()
                        .loc
                });

            window.with_state_mut(|state| {
                state.floating_loc = Some(loc);
            });

            if let Some(surface) = window.x11_surface() {
                let _ = surface.configure(Some(Rectangle::new(loc.to_i32_round(), size)));
            }
        }

        // TODO: xdg activation

        if window_rules.focused != Some(false) {
            output.with_state_mut(|state| state.focus_stack.set_focus(window.clone()));
            self.update_keyboard_focus(&output);
        } else {
            output.with_state_mut(|state| state.focus_stack.add_focus(window.clone()));
        }

        if !window.is_on_active_tag() {
            return;
        }

        match window.with_state(|state| state.layout_mode.current()) {
            LayoutModeKind::Tiled => {
                self.capture_snapshots_on_output(&output, []);
                self.pinnacle.begin_layout_transaction(&output);
                self.pinnacle.request_layout(&output);
            }
            LayoutModeKind::Floating => {
                let loc = window.with_state(|state| state.floating_loc.expect("initialized above"));
                self.pinnacle
                    .space
                    .map_element(window.clone(), loc.to_i32_round(), true);
            }
            LayoutModeKind::Maximized => {
                let non_exclusive_geo = {
                    let map = layer_map_for_output(&output);
                    map.non_exclusive_zone()
                };
                let loc = output_geo.loc + non_exclusive_geo.loc;
                self.pinnacle.space.map_element(window, loc, true);
            }
            LayoutModeKind::Fullscreen => {
                self.pinnacle
                    .space
                    .map_element(window, output_geo.loc, true);
            }
        }

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

            let is_transient = surface.is_transient_for().is_some();

            let requests_constrained_size = surface.size_hints().is_some_and(|size_hints| {
                let Some((min_w, min_h)) = size_hints.min_size else {
                    return false;
                };
                let Some((max_w, max_h)) = size_hints.max_size else {
                    return false;
                };
                min_w > 0 && min_h > 0 && (min_w == max_w || min_h == max_h)
            });

            let should_float =
                surface.is_popup() || is_popup_by_type || requests_constrained_size || is_transient;
            should_float
        }
    }
}

#[derive(Debug, Clone)]
pub struct Unmapped {
    pub window: WindowElement,
    pub activation_token_data: Option<XdgActivationTokenData>,
    pub window_rules: WindowRules,
    pub awaiting_tags: bool,
}
