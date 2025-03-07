// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    mem,
    sync::atomic::{AtomicU32, Ordering},
};

use indexmap::IndexSet;
use smithay::{
    backend::renderer::element::Id,
    desktop::{layer_map_for_output, WindowSurface},
    reexports::wayland_protocols::xdg::{
        decoration::zv1::server::zxdg_toplevel_decoration_v1, shell::server::xdg_toplevel,
    },
    utils::{Logical, Point, Rectangle, Serial, Size},
    wayland::compositor::HookId,
};
use tracing::warn;

use crate::{
    layout::transaction::LayoutSnapshot,
    state::{Pinnacle, WithState},
    tag::Tag,
};

use super::{Unmapped, WindowElement};

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
    pub fn reset() {
        WINDOW_ID_COUNTER.store(0, Ordering::Relaxed);
    }

    /// Get the mapped window that has this WindowId.
    pub fn window(&self, pinnacle: &Pinnacle) -> Option<WindowElement> {
        let _span = tracy_client::span!("WindowId::window");

        pinnacle
            .windows
            .iter()
            .find(|win| win.with_state(|state| &state.id == self))
            .cloned()
    }

    pub fn unmapped_window<'a>(&self, pinnacle: &'a Pinnacle) -> Option<&'a Unmapped> {
        pinnacle
            .unmapped_windows
            .iter()
            .find(|unmapped| unmapped.window.with_state(|state| &state.id == self))
    }

    pub fn unmapped_window_mut<'a>(&self, pinnacle: &'a mut Pinnacle) -> Option<&'a mut Unmapped> {
        pinnacle
            .unmapped_windows
            .iter_mut()
            .find(|unmapped| unmapped.window.with_state(|state| &state.id == self))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayoutModeKind {
    Tiled,
    Floating,
    Maximized,
    Fullscreen,
}

impl LayoutModeKind {
    /// Returns `true` if the layout mode kind is [`Tiled`].
    ///
    /// [`Tiled`]: LayoutModeKind::Tiled
    #[must_use]
    fn is_tiled(&self) -> bool {
        matches!(self, Self::Tiled)
    }

    /// Returns `true` if the layout mode kind is [`Floating`].
    ///
    /// [`Floating`]: LayoutModeKind::Floating
    #[must_use]
    fn is_floating(&self) -> bool {
        matches!(self, Self::Floating)
    }

    /// Returns `true` if the layout mode kind is [`Maximized`].
    ///
    /// [`Maximized`]: LayoutModeKind::Maximized
    #[must_use]
    fn is_maximized(&self) -> bool {
        matches!(self, Self::Maximized)
    }

    /// Returns `true` if the layout mode kind is [`Fullscreen`].
    ///
    /// [`Fullscreen`]: LayoutModeKind::Fullscreen
    #[must_use]
    fn is_fullscreen(&self) -> bool {
        matches!(self, Self::Fullscreen)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutMode {
    current: LayoutModeKind,
    previous: LayoutModeKind,
}

impl LayoutMode {
    pub fn tiled() -> Self {
        Self {
            current: LayoutModeKind::Tiled,
            previous: LayoutModeKind::Floating,
        }
    }

    pub fn floating() -> Self {
        Self {
            current: LayoutModeKind::Floating,
            previous: LayoutModeKind::Tiled,
        }
    }

    pub fn fullscreen() -> Self {
        Self {
            current: LayoutModeKind::Fullscreen,
            previous: LayoutModeKind::Tiled,
        }
    }

    pub fn maximized() -> Self {
        Self {
            current: LayoutModeKind::Maximized,
            previous: LayoutModeKind::Tiled,
        }
    }

    pub fn current(&self) -> LayoutModeKind {
        self.current
    }

    pub fn is_tiled(&self) -> bool {
        self.current.is_tiled()
    }

    pub fn is_floating(&self) -> bool {
        self.current.is_floating()
    }

    pub fn is_fullscreen(&self) -> bool {
        self.current.is_fullscreen()
    }

    pub fn is_maximized(&self) -> bool {
        self.current.is_maximized()
    }

    pub fn set_floating(&mut self, floating: bool) {
        match floating {
            true => {
                if !self.is_floating() {
                    self.previous = self.current;
                    self.current = LayoutModeKind::Floating;
                }
            }
            false => {
                if !self.is_tiled() {
                    self.previous = self.current;
                    self.current = LayoutModeKind::Tiled;
                }
            }
        }
    }

    pub fn toggle_floating(&mut self) {
        self.set_floating(!self.is_floating());
    }

    pub fn set_maximized(&mut self, maximized: bool) {
        match maximized {
            true => {
                if !self.is_maximized() {
                    self.previous = self.current;
                    self.current = LayoutModeKind::Maximized;
                }
            }
            false => {
                if self.is_maximized() {
                    mem::swap(&mut self.current, &mut self.previous);
                }
            }
        }
    }

    pub fn toggle_maximized(&mut self) {
        self.set_maximized(!self.is_maximized());
    }

    pub fn set_fullscreen(&mut self, fullscreen: bool) {
        match fullscreen {
            true => {
                if !self.is_fullscreen() {
                    self.previous = self.current;
                    self.current = LayoutModeKind::Fullscreen;
                }
            }
            false => {
                if self.is_fullscreen() {
                    mem::swap(&mut self.current, &mut self.previous);
                }
            }
        }
    }

    pub fn toggle_fullscreen(&mut self) {
        self.set_fullscreen(!self.is_fullscreen());
    }
}

/// State of a [`WindowElement`]
#[derive(Debug)]
pub struct WindowElementState {
    /// The id of this window.
    pub id: WindowId,
    /// What tags the window is currently on.
    pub tags: IndexSet<Tag>,
    pub layout_mode: LayoutMode,
    pub minimized: bool,
    /// The most recent serial that has been committed.
    pub committed_serial: Option<Serial>,
    pub snapshot: Option<LayoutSnapshot>,
    pub snapshot_hook_id: Option<HookId>,
    pub decoration_mode: Option<zxdg_toplevel_decoration_v1::Mode>,
    pub floating_loc: Option<Point<f64, Logical>>,
    pub floating_size: Size<i32, Logical>,

    /// The id of a snapshot element if any.
    ///
    /// When updating the primary scanout output, Smithay looks at the ids of all elements drawn on
    /// screen. If it matches the ids of this window's elements, the primary output is updated.
    /// However, when a snapshot is rendering, the snapshot's element id is different from this
    /// window's ids. Therefore, we clone that snapshot's id into this field and use it to update
    /// the primary output when necessary.
    ///
    /// See [`Pinnacle::update_primary_scanout_output`] for more details.
    pub offscreen_elem_id: Option<Id>,
}

impl WindowElement {
    /// Unsets maximized and fullscreen states for both wayland and xwayland windows
    /// and unsets tiled states for wayland windows.
    pub(super) fn set_floating_states(&self) {
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
                    let _ = surface.set_maximized(false);
                    let _ = surface.set_fullscreen(false);
                }
            }
        }
    }

    /// Unsets maximized and fullscreen states for both wayland and xwayland windows
    /// and sets tiled states for wayland windows.
    pub fn set_tiled_states(&self) {
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
                    let _ = surface.set_maximized(false);
                    let _ = surface.set_fullscreen(false);
                }
            }
        }
    }

    pub(super) fn set_fullscreen_states(&self) {
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
                    let _ = surface.set_maximized(false);
                    let _ = surface.set_fullscreen(true);
                }
            }
        }
    }

    pub(super) fn set_maximized_states(&self) {
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
                    let _ = surface.set_maximized(true);
                    let _ = surface.set_fullscreen(false);
                }
            }
        }
    }
}

impl Pinnacle {
    /// Updates toplevel/x11surface state for a window's layout mode.
    ///
    /// You may need to call `send_configure`/`send_pending_configure` after this
    /// for toplevels.
    pub fn configure_window_if_nontiled(&self, window: &WindowElement) {
        let _span = tracy_client::span!("Pinnacle::configure_window_if_nontiled");

        if window.is_x11_override_redirect() {
            return;
        }

        let layout_mode = window.with_state(|state| state.layout_mode);

        let Some(output) = window.output(self) else {
            warn!("Tried to update layout mode of window with no tags");
            return;
        };

        let Some(output_geo) = self.space.output_geometry(&output) else {
            warn!("Tried to update layout mode of window on an unmapped output");
            return;
        };

        match layout_mode.current() {
            LayoutModeKind::Tiled => (),
            LayoutModeKind::Floating => {
                window.set_floating_states();

                let (size, loc) =
                    window.with_state(|state| (state.floating_size, state.floating_loc));

                match window.underlying_surface() {
                    WindowSurface::Wayland(toplevel) => {
                        toplevel.with_pending_state(|state| {
                            state.size = Some(size);
                        });
                    }
                    WindowSurface::X11(surface) => {
                        if size.is_empty() {
                            // https://www.x.org/releases/X11R7.6/doc/man/man3/XConfigureWindow.3.xhtml
                            // Setting a zero size seems to be a nono
                            return;
                        }
                        let loc = loc.unwrap_or_else(|| surface.geometry().loc.to_f64());
                        if let Err(err) =
                            surface.configure(Some(Rectangle::new(loc.to_i32_round(), size)))
                        {
                            warn!("Failed to configure xwayland window: {err}");
                        }
                    }
                }
            }
            LayoutModeKind::Maximized { .. } => {
                let non_exclusive_geo = {
                    let map = layer_map_for_output(&output);
                    map.non_exclusive_zone()
                };
                let loc = output_geo.loc + non_exclusive_geo.loc;

                window.set_maximized_states();

                match window.underlying_surface() {
                    WindowSurface::Wayland(toplevel) => {
                        toplevel.with_pending_state(|state| {
                            state.size = Some(non_exclusive_geo.size);
                        });
                    }
                    WindowSurface::X11(surface) => {
                        if let Err(err) =
                            surface.configure(Some(Rectangle::new(loc, non_exclusive_geo.size)))
                        {
                            warn!("Failed to configure xwayland window: {err}");
                        }
                    }
                }
            }
            LayoutModeKind::Fullscreen { .. } => {
                window.set_fullscreen_states();

                match window.underlying_surface() {
                    WindowSurface::Wayland(toplevel) => {
                        toplevel.with_pending_state(|state| {
                            state.size = Some(output_geo.size);
                        });
                    }
                    WindowSurface::X11(surface) => {
                        if let Err(err) = surface.configure(Some(output_geo)) {
                            warn!("Failed to configure xwayland window: {err}");
                        }
                    }
                }
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
            tags: Default::default(),
            layout_mode: LayoutMode::tiled(),
            floating_loc: None,
            floating_size: Default::default(),
            minimized: false,
            committed_serial: None,
            snapshot: None,
            snapshot_hook_id: None,
            decoration_mode: None,
            offscreen_elem_id: None,
        }
    }
}
