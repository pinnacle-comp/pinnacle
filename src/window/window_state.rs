// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::atomic::{AtomicU32, Ordering};

use smithay::{
    desktop::{space::SpaceElement, WindowSurface},
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel,
    utils::{Logical, Point, Serial, Size},
    wayland::compositor::HookId,
};

use crate::{
    layout::transaction::LayoutSnapshot,
    state::{Pinnacle, WithState},
    tag::Tag,
};

use super::{rules::DecorationMode, WindowElement};

/// A unique identifier for each window.
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WindowId(pub u32);

static WINDOW_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

impl WindowId {
    /// Get the next available window id. This always starts at 0.
    pub fn next() -> Self {
        Self(WINDOW_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Reset the static window id counter to 0.
    /// FIXME: remove statics
    pub fn reset() {
        WINDOW_ID_COUNTER.store(0, Ordering::Relaxed);
    }

    /// Get the window that has this WindowId.
    pub fn window(&self, pinnacle: &Pinnacle) -> Option<WindowElement> {
        pinnacle
            .windows
            .iter()
            .find(|win| win.with_state(|state| &state.id == self))
            .cloned()
    }
}

/// State of a [`WindowElement`]
#[derive(Debug)]
pub struct WindowElementState {
    /// The id of this window.
    pub id: WindowId,
    /// What tags the window is currently on.
    pub tags: Vec<Tag>,
    pub floating_or_tiled: FloatingOrTiled,
    pub fullscreen_or_maximized: FullscreenOrMaximized,
    pub target_loc: Option<Point<i32, Logical>>,
    pub minimized: bool,
    /// The most recent serial that has been committed.
    pub committed_serial: Option<Serial>,
    pub snapshot: Option<LayoutSnapshot>,
    pub snapshot_hook_id: Option<HookId>,
    pub decoration_mode: Option<DecorationMode>,
}

impl WindowElement {
    /// RefCell Safety: This method uses a [`RefCell`] on this window.
    pub fn toggle_floating(&self) {
        match self.with_state(|state| state.floating_or_tiled) {
            FloatingOrTiled::Floating { loc, size } => {
                self.with_state_mut(|state| {
                    state.floating_or_tiled = FloatingOrTiled::Tiled(Some((loc, size)))
                });
                self.set_tiled_states();
            }
            FloatingOrTiled::Tiled(prev_rect) => {
                // FIXME: is using window geometry here right?
                let (prev_loc, prev_size) = prev_rect.unwrap_or_else(|| {
                    let geo = self.geometry();
                    (geo.loc.to_f64(), geo.size)
                });

                self.with_state_mut(|state| {
                    state.floating_or_tiled = FloatingOrTiled::Floating {
                        loc: prev_loc,
                        size: prev_size,
                    };
                });

                // TODO: maybe move this into update_windows
                self.change_geometry(prev_loc, prev_size);
                self.set_floating_states();
            }
        }
    }

    /// RefCell Safety: This method uses a [`RefCell`] on this window.
    pub fn toggle_fullscreen(&self) {
        match self.with_state(|state| state.fullscreen_or_maximized) {
            FullscreenOrMaximized::Neither | FullscreenOrMaximized::Maximized => {
                self.with_state_mut(|state| {
                    state.fullscreen_or_maximized = FullscreenOrMaximized::Fullscreen;
                });

                match self.underlying_surface() {
                    WindowSurface::Wayland(toplevel) => {
                        toplevel.with_pending_state(|state| {
                            state.states.unset(xdg_toplevel::State::Maximized);
                            state.states.set(xdg_toplevel::State::Fullscreen);
                            state.states.set(xdg_toplevel::State::TiledTop);
                            state.states.set(xdg_toplevel::State::TiledLeft);
                            state.states.set(xdg_toplevel::State::TiledBottom);
                            state.states.set(xdg_toplevel::State::TiledRight);
                        });
                    }
                    WindowSurface::X11(surface) => {
                        if !surface.is_override_redirect() {
                            surface
                                .set_maximized(false)
                                .expect("failed to set x11 win to maximized");
                            surface
                                .set_fullscreen(true)
                                .expect("failed to set x11 win to not fullscreen");
                        }
                    }
                }
            }
            FullscreenOrMaximized::Fullscreen => {
                self.with_state_mut(|state| {
                    state.fullscreen_or_maximized = FullscreenOrMaximized::Neither;
                });

                match self.with_state(|state| state.floating_or_tiled) {
                    FloatingOrTiled::Floating { loc, size } => {
                        self.change_geometry(loc, size);
                        self.set_floating_states();
                    }
                    FloatingOrTiled::Tiled(_) => self.set_tiled_states(),
                }
            }
        }
    }

    /// RefCell Safety: This method uses a [`RefCell`] on this window.
    pub fn toggle_maximized(&self) {
        match self.with_state(|state| state.fullscreen_or_maximized) {
            FullscreenOrMaximized::Neither | FullscreenOrMaximized::Fullscreen => {
                self.with_state_mut(|state| {
                    state.fullscreen_or_maximized = FullscreenOrMaximized::Maximized;
                });

                match self.underlying_surface() {
                    WindowSurface::Wayland(toplevel) => {
                        toplevel.with_pending_state(|state| {
                            state.states.set(xdg_toplevel::State::Maximized);
                            state.states.unset(xdg_toplevel::State::Fullscreen);
                            state.states.set(xdg_toplevel::State::TiledTop);
                            state.states.set(xdg_toplevel::State::TiledLeft);
                            state.states.set(xdg_toplevel::State::TiledBottom);
                            state.states.set(xdg_toplevel::State::TiledRight);
                        });
                    }
                    WindowSurface::X11(surface) => {
                        if !surface.is_override_redirect() {
                            surface
                                .set_maximized(true)
                                .expect("failed to set x11 win to maximized");
                            surface
                                .set_fullscreen(false)
                                .expect("failed to set x11 win to not fullscreen");
                        }
                    }
                }
            }
            FullscreenOrMaximized::Maximized => {
                self.with_state_mut(|state| {
                    state.fullscreen_or_maximized = FullscreenOrMaximized::Neither;
                });

                match self.with_state(|state| state.floating_or_tiled) {
                    FloatingOrTiled::Floating { loc, size } => {
                        self.change_geometry(loc, size);
                        self.set_floating_states();
                    }
                    FloatingOrTiled::Tiled(_) => self.set_tiled_states(),
                }
            }
        }
    }

    /// Unsets maximized and fullscreen states for both wayland and xwayland windows
    /// and unsets tiled states for wayland windows.
    fn set_floating_states(&self) {
        match self.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                toplevel.with_pending_state(|state| {
                    state.states.unset(xdg_toplevel::State::Maximized);
                    state.states.unset(xdg_toplevel::State::Fullscreen);
                    state.states.unset(xdg_toplevel::State::TiledTop);
                    state.states.unset(xdg_toplevel::State::TiledLeft);
                    state.states.unset(xdg_toplevel::State::TiledBottom);
                    state.states.unset(xdg_toplevel::State::TiledRight);
                });
            }
            WindowSurface::X11(surface) => {
                if !surface.is_override_redirect() {
                    surface
                        .set_maximized(false)
                        .expect("failed to set x11 win to maximized");
                    surface
                        .set_fullscreen(false)
                        .expect("failed to set x11 win to not fullscreen");
                }
            }
        }
    }

    /// Unsets maximized and fullscreen states for both wayland and xwayland windows
    /// and sets tiled states for wayland windows.
    fn set_tiled_states(&self) {
        match self.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                toplevel.with_pending_state(|state| {
                    state.states.unset(xdg_toplevel::State::Maximized);
                    state.states.unset(xdg_toplevel::State::Fullscreen);
                    state.states.set(xdg_toplevel::State::TiledTop);
                    state.states.set(xdg_toplevel::State::TiledLeft);
                    state.states.set(xdg_toplevel::State::TiledBottom);
                    state.states.set(xdg_toplevel::State::TiledRight);
                });
            }
            WindowSurface::X11(surface) => {
                if !surface.is_override_redirect() {
                    surface
                        .set_maximized(false)
                        .expect("failed to set x11 win to maximized");
                    surface
                        .set_fullscreen(false)
                        .expect("failed to set x11 win to not fullscreen");
                }
            }
        }
    }
}

/// Whether a window is floating or tiled
#[derive(Debug, Clone, Copy)]
pub enum FloatingOrTiled {
    /// The window is floating with the specified geometry.
    Floating {
        loc: Point<f64, Logical>,
        size: Size<i32, Logical>,
    },
    /// The window is tiled.
    ///
    /// The previous geometry it had when it was floating is stored here.
    /// This is so when it becomes floating again, it returns to this geometry.
    Tiled(Option<(Point<f64, Logical>, Size<i32, Logical>)>),
}

impl FloatingOrTiled {
    /// Returns `true` if the floating or tiled is [`Floating`].
    ///
    /// [`Floating`]: FloatingOrTiled::Floating
    #[must_use]
    pub fn is_floating(&self) -> bool {
        matches!(self, Self::Floating { .. })
    }

    /// Returns `true` if the floating or tiled is [`Tiled`].
    ///
    /// [`Tiled`]: FloatingOrTiled::Tiled
    #[must_use]
    pub fn is_tiled(&self) -> bool {
        matches!(self, Self::Tiled(..))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FullscreenOrMaximized {
    Neither,
    Fullscreen,
    Maximized,
}

impl FullscreenOrMaximized {
    /// Returns `true` if the fullscreen or maximized is [`Neither`].
    ///
    /// [`Neither`]: FullscreenOrMaximized::Neither
    #[must_use]
    pub fn is_neither(&self) -> bool {
        matches!(self, Self::Neither)
    }

    /// Returns `true` if the fullscreen or maximized is [`Fullscreen`].
    ///
    /// [`Fullscreen`]: FullscreenOrMaximized::Fullscreen
    #[must_use]
    pub fn is_fullscreen(&self) -> bool {
        matches!(self, Self::Fullscreen)
    }

    /// Returns `true` if the fullscreen or maximized is [`Maximized`].
    ///
    /// [`Maximized`]: FullscreenOrMaximized::Maximized
    #[must_use]
    pub fn is_maximized(&self) -> bool {
        matches!(self, Self::Maximized)
    }
}

impl WindowElementState {
    pub fn new() -> Self {
        Self {
            id: WindowId::next(),
            // loc_request_state: LocationRequestState::Idle,
            tags: vec![],
            floating_or_tiled: FloatingOrTiled::Tiled(None),
            fullscreen_or_maximized: FullscreenOrMaximized::Neither,
            target_loc: None,
            minimized: false,
            committed_serial: None,
            snapshot: None,
            snapshot_hook_id: None,
            decoration_mode: None,
        }
    }
}
