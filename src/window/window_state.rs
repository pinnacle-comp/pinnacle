// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::atomic::{AtomicU32, Ordering};

use indexmap::IndexSet;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowState {
    Tiled,
    Floating,
    Maximized { previous_state: FloatingOrTiled },
    Fullscreen { previous_state: FloatingOrTiled },
}

impl WindowState {
    pub fn set_floating(&mut self, floating: bool) {
        if floating {
            *self = WindowState::Floating;
        } else {
            *self = WindowState::Tiled;
        }
    }

    pub fn toggle_floating(&mut self) {
        *self = match self {
            WindowState::Tiled => WindowState::Floating,
            WindowState::Floating => WindowState::Tiled,
            WindowState::Maximized { previous_state }
            | WindowState::Fullscreen { previous_state } => match previous_state {
                FloatingOrTiled::Floating => WindowState::Tiled,
                FloatingOrTiled::Tiled => WindowState::Floating,
            },
        }
    }

    pub fn set_maximized(&mut self, maximized: bool) {
        if maximized {
            *self = match self {
                WindowState::Tiled => WindowState::Maximized {
                    previous_state: FloatingOrTiled::Tiled,
                },
                WindowState::Floating => WindowState::Maximized {
                    previous_state: FloatingOrTiled::Floating,
                },
                ref it @ WindowState::Maximized { .. } => **it,
                WindowState::Fullscreen { previous_state } => WindowState::Maximized {
                    previous_state: *previous_state,
                },
            }
        } else if let WindowState::Maximized { previous_state } = self {
            *self = match previous_state {
                FloatingOrTiled::Floating => WindowState::Floating,
                FloatingOrTiled::Tiled => WindowState::Tiled,
            }
        }
    }

    pub fn toggle_maximized(&mut self) {
        *self = match self {
            WindowState::Tiled => WindowState::Maximized {
                previous_state: FloatingOrTiled::Tiled,
            },
            WindowState::Floating => WindowState::Maximized {
                previous_state: FloatingOrTiled::Floating,
            },
            WindowState::Maximized { previous_state } => match previous_state {
                FloatingOrTiled::Floating => WindowState::Floating,
                FloatingOrTiled::Tiled => WindowState::Tiled,
            },
            WindowState::Fullscreen { previous_state } => WindowState::Maximized {
                previous_state: *previous_state,
            },
        }
    }

    pub fn set_fullscreen(&mut self, fullscreen: bool) {
        if fullscreen {
            *self = match self {
                WindowState::Tiled => WindowState::Fullscreen {
                    previous_state: FloatingOrTiled::Tiled,
                },
                WindowState::Floating => WindowState::Fullscreen {
                    previous_state: FloatingOrTiled::Floating,
                },
                ref it @ WindowState::Fullscreen { .. } => **it,
                WindowState::Maximized { previous_state } => WindowState::Fullscreen {
                    previous_state: *previous_state,
                },
            }
        } else if let WindowState::Fullscreen { previous_state } = self {
            *self = match previous_state {
                FloatingOrTiled::Floating => WindowState::Floating,
                FloatingOrTiled::Tiled => WindowState::Tiled,
            }
        }
    }

    pub fn toggle_fullscreen(&mut self) {
        *self = match self {
            WindowState::Tiled => WindowState::Fullscreen {
                previous_state: FloatingOrTiled::Tiled,
            },
            WindowState::Floating => WindowState::Fullscreen {
                previous_state: FloatingOrTiled::Floating,
            },
            WindowState::Fullscreen { previous_state } => match previous_state {
                FloatingOrTiled::Floating => WindowState::Floating,
                FloatingOrTiled::Tiled => WindowState::Tiled,
            },
            WindowState::Maximized { previous_state } => WindowState::Fullscreen {
                previous_state: *previous_state,
            },
        }
    }

    /// Returns `true` if the window state is [`Tiled`].
    ///
    /// [`Tiled`]: WindowState::Tiled
    #[must_use]
    pub fn is_tiled(&self) -> bool {
        matches!(self, Self::Tiled)
    }

    /// Returns `true` if the window state is [`Floating`].
    ///
    /// [`Floating`]: WindowState::Floating
    #[must_use]
    pub fn is_floating(&self) -> bool {
        matches!(self, Self::Floating)
    }

    /// Returns `true` if the window state is [`Maximized`].
    ///
    /// [`Maximized`]: WindowState::Maximized
    #[must_use]
    pub fn is_maximized(&self) -> bool {
        matches!(self, Self::Maximized { .. })
    }

    /// Returns `true` if the window state is [`Fullscreen`].
    ///
    /// [`Fullscreen`]: WindowState::Fullscreen
    #[must_use]
    pub fn is_fullscreen(&self) -> bool {
        matches!(self, Self::Fullscreen { .. })
    }
}

/// State of a [`WindowElement`]
#[derive(Debug)]
pub struct WindowElementState {
    /// The id of this window.
    pub id: WindowId,
    /// What tags the window is currently on.
    pub tags: IndexSet<Tag>,
    pub window_state: WindowState,
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
    /// Update a window's state from its [`WindowState`] (shocking).
    pub fn update_window_state(&self, window: &WindowElement) {
        let window_state = window.with_state(|state| state.window_state);

        match window_state {
            WindowState::Tiled => {
                window.set_tiled_states();
            }
            WindowState::Floating => {
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
                });

                window.change_geometry(loc, size);
                window.set_floating_states();
            }
            WindowState::Maximized { .. } => match window.underlying_surface() {
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
            },
            WindowState::Fullscreen { .. } => match window.underlying_surface() {
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
            },
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
            tags: Default::default(),
            window_state: WindowState::Tiled,
            floating_loc: None,
            floating_size: None,
            target_loc: None,
            minimized: false,
            committed_serial: None,
            snapshot: None,
            snapshot_hook_id: None,
            decoration_mode: None,
        }
    }
}
