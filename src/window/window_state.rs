// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::atomic::{AtomicU32, Ordering};

use smithay::{
    desktop::{space::SpaceElement, WindowSurface},
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel,
    utils::{Logical, Point, Serial, Size},
    wayland::compositor::HookId,
};
use tracing::warn;

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
    pub floating_loc: Option<Point<f64, Logical>>,
    pub floating_size: Option<Size<i32, Logical>>,
}

impl WindowElement {
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

impl Pinnacle {
    pub fn set_window_floating(&self, window: &WindowElement, floating: bool) {
        // If the window is fullscreen or maximized, simply mark it as floating or tiled
        // and don't set floating or tiled states to prevent stuff like decorations
        // appearing in fullscreen mode.
        if window.with_state(|state| !state.fullscreen_or_maximized.is_neither()) {
            window.with_state_mut(|state| {
                state.floating_or_tiled = match floating {
                    true => FloatingOrTiled::Floating,
                    false => FloatingOrTiled::Tiled,
                }
            });
            return;
        }

        if floating {
            let size = window
                .with_state(|state| state.floating_size)
                .unwrap_or_else(|| window.geometry().size);
            let loc = window
                .with_state(|state| state.floating_loc)
                .or_else(|| self.space.element_location(window).map(|loc| loc.to_f64()))
                .or_else(|| {
                    self.focused_output().map(|op| {
                        let op_geo = self
                            .space
                            .output_geometry(op)
                            .expect("focused output wasn't mapped");

                        let x = op_geo.loc.x + op_geo.size.w / 2 - (size.w / 2);
                        let y = op_geo.loc.y + op_geo.size.h / 2 - (size.h / 2);

                        (x as f64, y as f64).into()
                    })
                })
                .unwrap_or_default();

            window.with_state_mut(|state| {
                state.floating_size = Some(size);
                state.floating_loc = Some(loc);
                state.floating_or_tiled = FloatingOrTiled::Floating;
            });

            window.change_geometry(loc, size);
            window.set_floating_states();
        } else {
            let geo = self.space.element_geometry(window);

            window.with_state_mut(|state| {
                if let Some(geo) = geo {
                    state.floating_size.replace(geo.size);
                    state.floating_loc.replace(geo.loc.to_f64()); // FIXME: i32 -> f64
                }
                state.floating_or_tiled = FloatingOrTiled::Tiled;
            });
            window.set_tiled_states();
        }
    }

    pub fn set_window_maximized(&self, window: &WindowElement, maximized: bool) {
        if maximized {
            // We only want to update the stored floating geometry when exiting floating mode.
            if window.with_state(|state| {
                state.floating_or_tiled.is_floating() && state.fullscreen_or_maximized.is_neither()
            }) {
                let geo = self.space.element_geometry(window);

                if let Some(geo) = geo {
                    window.with_state_mut(|state| {
                        state.floating_size.replace(geo.size);
                        state.floating_loc.replace(geo.loc.to_f64()); // FIXME: i32 -> f64
                    });
                }
            }

            window.with_state_mut(|state| {
                state.fullscreen_or_maximized = FullscreenOrMaximized::Maximized;
            });

            match window.underlying_surface() {
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
                        if let Err(err) = surface.set_maximized(true) {
                            warn!("Failed to set xwayland window to maximized: {err}");
                        }
                        if let Err(err) = surface.set_fullscreen(false) {
                            warn!("Failed to unset xwayland window fullscreen: {err}");
                        }
                    }
                }
            }
        } else {
            window.with_state_mut(|state| {
                state.fullscreen_or_maximized = FullscreenOrMaximized::Neither;
            });

            match window.with_state(|state| state.floating_or_tiled) {
                FloatingOrTiled::Floating => self.set_window_floating(window, true),
                FloatingOrTiled::Tiled => window.set_tiled_states(),
            }
        }
    }

    pub fn set_window_fullscreen(&self, window: &WindowElement, fullscreen: bool) {
        if fullscreen {
            // We only want to update the stored floating geometry when exiting floating mode.
            if window.with_state(|state| {
                state.floating_or_tiled.is_floating() && state.fullscreen_or_maximized.is_neither()
            }) {
                let geo = self.space.element_geometry(window);

                if let Some(geo) = geo {
                    window.with_state_mut(|state| {
                        state.floating_size.replace(geo.size);
                        state.floating_loc.replace(geo.loc.to_f64()); // FIXME: i32 -> f64
                    });
                }
            }

            window.with_state_mut(|state| {
                state.fullscreen_or_maximized = FullscreenOrMaximized::Fullscreen;
            });

            match window.underlying_surface() {
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
                        if let Err(err) = surface.set_maximized(false) {
                            warn!("Failed to unset xwayland window maximized: {err}");
                        }
                        if let Err(err) = surface.set_fullscreen(true) {
                            warn!("Failed to set xwayland window to fullscreen: {err}");
                        }
                    }
                }
            }
        } else {
            window.with_state_mut(|state| {
                state.fullscreen_or_maximized = FullscreenOrMaximized::Neither;
            });

            match window.with_state(|state| state.floating_or_tiled) {
                FloatingOrTiled::Floating => self.set_window_floating(window, true),
                FloatingOrTiled::Tiled => window.set_tiled_states(),
            }
        }
    }
}

/// Whether a window is floating or tiled
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatingOrTiled {
    /// The window is floating.
    Floating,
    /// The window is tiled.
    Tiled,
}

impl FloatingOrTiled {
    /// Returns `true` if the floating or tiled is [`Floating`].
    ///
    /// [`Floating`]: FloatingOrTiled::Floating
    #[must_use]
    pub fn is_floating(&self) -> bool {
        matches!(self, Self::Floating)
    }

    /// Returns `true` if the floating or tiled is [`Tiled`].
    ///
    /// [`Tiled`]: FloatingOrTiled::Tiled
    #[must_use]
    pub fn is_tiled(&self) -> bool {
        matches!(self, Self::Tiled)
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
            tags: vec![],
            floating_or_tiled: FloatingOrTiled::Tiled,
            floating_loc: None,
            floating_size: None,
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
