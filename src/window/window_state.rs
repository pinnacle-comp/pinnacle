// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::atomic::{AtomicU32, Ordering};

use indexmap::IndexSet;
use smithay::{
    desktop::{WindowSurface, layer_map_for_output},
    reexports::wayland_protocols::xdg::{
        decoration::zv1::server::zxdg_toplevel_decoration_v1, shell::server::xdg_toplevel,
    },
    utils::{Logical, Point, Rectangle, Serial, Size},
    wayland::{compositor::HookId, foreign_toplevel_list::ForeignToplevelHandle},
};
use tracing::warn;

use crate::{
    render::util::snapshot::WindowSnapshot,
    state::{Pinnacle, WithState},
    tag::Tag,
    util::transaction::Transaction,
};

use super::{Unmapped, WindowElement};

/// A unique identifier for each window.
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub struct WindowId(pub u32);

static WINDOW_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

impl WindowId {
    /// Gets the next available window id. This always starts at 0.
    pub fn next() -> Self {
        Self(WINDOW_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Reset the static window id counter to 0.
    pub fn reset() {
        WINDOW_ID_COUNTER.store(0, Ordering::Relaxed);
    }

    /// Gets the mapped window that has this WindowId.
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
    Spilled,
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

    /// Returns `true` if the LayoutModeKind is [`Spilled`]
    ///
    /// [`Spilled`]: LayoutModeKind::Spilled
    #[must_use]
    fn is_spilled(&self) -> bool {
        matches!(self, Self::Spilled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutMode {
    /// The base layout mode. This is either floating or tiled.
    base_mode: FloatingOrTiled,
    /// A semantically "elevated" layout mode that applies over the base mode.
    elevated_mode: Option<FullscreenOrMaximized>,
    /// An elevated layout mode requested external to the config, e.g. from a client or
    /// wlr-foreign-toplevel-management.
    pub client_requested_mode: Option<FullscreenOrMaximized>,
}

impl LayoutMode {
    /// Creates a new layout mode that is tiled.
    pub fn new_tiled() -> Self {
        Self {
            base_mode: FloatingOrTiled::Tiled,
            elevated_mode: None,
            client_requested_mode: None,
        }
    }

    /// Creates a new layout mode that is floating.
    pub fn new_floating() -> Self {
        Self {
            base_mode: FloatingOrTiled::Floating,
            elevated_mode: None,
            client_requested_mode: None,
        }
    }

    /// Creates a new layout mode that is fullscreen with a base mode of tiled.
    pub fn new_fullscreen() -> Self {
        Self {
            base_mode: FloatingOrTiled::Tiled,
            elevated_mode: Some(FullscreenOrMaximized::Fullscreen),
            client_requested_mode: None,
        }
    }

    /// Creates a new layout mode that is fullscreen with a base mode of tiled.
    /// This mode should be created in response to a client requested mode.
    pub fn new_fullscreen_external() -> Self {
        Self {
            base_mode: FloatingOrTiled::Tiled,
            elevated_mode: None,
            client_requested_mode: Some(FullscreenOrMaximized::Fullscreen),
        }
    }

    /// Creates a new layout mode that is maximized with a base mode of tiled.
    pub fn new_maximized() -> Self {
        Self {
            base_mode: FloatingOrTiled::Tiled,
            elevated_mode: Some(FullscreenOrMaximized::Maximized),
            client_requested_mode: None,
        }
    }

    /// Creates a new layout mode that is maximized with a base mode of tiled.
    /// This mode should be created in response to a client requested mode.
    pub fn new_maximized_external() -> Self {
        Self {
            base_mode: FloatingOrTiled::Tiled,
            elevated_mode: None,
            client_requested_mode: Some(FullscreenOrMaximized::Maximized),
        }
    }

    /// Returns the current layout mode.
    pub fn current(&self) -> LayoutModeKind {
        self.client_requested_mode
            .or(self.elevated_mode)
            .map(|mode| match mode {
                FullscreenOrMaximized::Fullscreen => LayoutModeKind::Fullscreen,
                FullscreenOrMaximized::Maximized => LayoutModeKind::Maximized,
            })
            .unwrap_or_else(|| match self.base_mode {
                FloatingOrTiled::Floating => LayoutModeKind::Floating,
                FloatingOrTiled::Tiled => LayoutModeKind::Tiled,
                FloatingOrTiled::Spilled => LayoutModeKind::Spilled,
            })
    }

    pub fn is_tiled(&self) -> bool {
        self.current().is_tiled()
    }

    pub fn is_floating(&self) -> bool {
        self.current().is_floating()
    }

    pub fn is_spilled(&self) -> bool {
        self.current().is_spilled()
    }

    pub fn is_fullscreen(&self) -> bool {
        self.current().is_fullscreen()
    }

    pub fn is_maximized(&self) -> bool {
        self.current().is_maximized()
    }

    pub fn set_floating(&mut self, floating: bool) {
        match floating {
            true => {
                if !self.is_floating() {
                    self.client_requested_mode = None;
                    self.elevated_mode = None;
                    self.base_mode = FloatingOrTiled::Floating;
                }
            }
            false => {
                if self.is_floating() {
                    self.client_requested_mode = None;
                    self.elevated_mode = None;
                    self.base_mode = FloatingOrTiled::Tiled;
                }
            }
        }
    }

    pub fn set_spilled(&mut self, spilled: bool) {
        match spilled {
            true => {
                if !self.is_spilled() {
                    self.client_requested_mode = None;
                    self.elevated_mode = None;
                    self.base_mode = FloatingOrTiled::Spilled;
                }
            }
            false => {
                if self.is_spilled() {
                    self.client_requested_mode = None;
                    self.elevated_mode = None;
                    self.base_mode = FloatingOrTiled::Tiled
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
                    self.client_requested_mode = None;
                    self.elevated_mode = Some(FullscreenOrMaximized::Maximized);
                }
            }
            false => {
                if self.is_maximized() {
                    if self.client_requested_mode == Some(FullscreenOrMaximized::Maximized) {
                        self.client_requested_mode = None;
                    } else {
                        self.elevated_mode = None;
                    }
                }
            }
        }
    }

    /// Sets maximized state. Use this in response to a client requested maximized mode.
    pub fn set_client_maximized(&mut self, maximized: bool) {
        match maximized {
            true => {
                if !self.is_maximized() {
                    self.client_requested_mode = Some(FullscreenOrMaximized::Maximized);
                }
            }
            false => {
                let took = self
                    .client_requested_mode
                    .take_if(|mode| mode.is_maximized())
                    .is_some();

                if !took {
                    self.elevated_mode.take_if(|mode| mode.is_maximized());
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
                    self.client_requested_mode = None;
                    self.elevated_mode = Some(FullscreenOrMaximized::Fullscreen);
                }
            }
            false => {
                if self.is_fullscreen() {
                    if self.client_requested_mode == Some(FullscreenOrMaximized::Fullscreen) {
                        self.client_requested_mode = None;
                    } else {
                        self.elevated_mode = None;
                    }
                }
            }
        }
    }

    pub fn toggle_fullscreen(&mut self) {
        self.set_fullscreen(!self.is_fullscreen());
    }

    /// Sets fullscreen state. Use this in response to a client requested fullscreen mode.
    pub fn set_client_fullscreen(&mut self, fullscreen: bool) {
        match fullscreen {
            true => {
                if !self.is_fullscreen() {
                    self.client_requested_mode = Some(FullscreenOrMaximized::Fullscreen);
                }
            }
            false => {
                let took = self
                    .client_requested_mode
                    .take_if(|mode| mode.is_fullscreen())
                    .is_some();

                if !took {
                    self.elevated_mode.take_if(|mode| mode.is_fullscreen());
                }
            }
        }
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
    pub decoration_mode: Option<zxdg_toplevel_decoration_v1::Mode>,
    pub floating_x: Option<i32>,
    pub floating_y: Option<i32>,
    pub floating_size: Size<i32, Logical>,

    pub pending_transactions: Vec<(Serial, Transaction)>,

    pub layout_node: Option<taffy::NodeId>,

    // FIXME: Turn `WindowElement` into `Mapped`
    // and move these fields into that
    pub snapshot: Option<WindowSnapshot>,
    pub mapped_hook_id: Option<HookId>,
    pub foreign_toplevel_list_handle: Option<ForeignToplevelHandle>,
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
            LayoutModeKind::Floating | LayoutModeKind::Spilled => {
                window.set_floating_states();

                let (size, loc) =
                    window.with_state(|state| (state.floating_size, state.floating_loc()));

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
                        let loc = loc.unwrap_or_else(|| surface.geometry().loc);
                        if let Err(err) = surface.configure(Some(Rectangle::new(loc, size))) {
                            warn!("Failed to configure xwayland window: {err}");
                        }
                    }
                }
            }
            LayoutModeKind::Maximized => {
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
            LayoutModeKind::Fullscreen => {
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
    Spilled,
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

    #[must_use]
    pub fn is_spilled(&self) -> bool {
        matches!(self, Self::Tiled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FullscreenOrMaximized {
    Fullscreen,
    Maximized,
}

impl FullscreenOrMaximized {
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
            layout_mode: LayoutMode::new_tiled(),
            floating_x: Default::default(),
            floating_y: Default::default(),
            floating_size: Default::default(),
            minimized: false,
            snapshot: None,
            mapped_hook_id: None,
            decoration_mode: None,
            pending_transactions: Default::default(),
            layout_node: None,
            foreign_toplevel_list_handle: None,
        }
    }

    pub fn floating_loc(&self) -> Option<Point<i32, Logical>> {
        if let (Some(x), Some(y)) = (self.floating_x, self.floating_y) {
            Some(Point::from((x, y)))
        } else {
            None
        }
    }

    pub fn set_floating_loc(&mut self, loc: impl Into<Option<Point<i32, Logical>>>) {
        let loc: Option<Point<_, _>> = loc.into();
        self.floating_x = loc.map(|loc| loc.x);
        self.floating_y = loc.map(|loc| loc.y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_mode_changes_correctly_user_only() {
        let mut layout_mode = LayoutMode::new_tiled();
        assert!(layout_mode.is_tiled());

        // toggle_floating
        layout_mode.toggle_floating();
        assert!(layout_mode.is_floating());
        layout_mode.toggle_floating();
        assert!(layout_mode.is_tiled());

        // set_floating
        layout_mode.set_floating(false);
        assert!(layout_mode.is_tiled());
        layout_mode.set_floating(true);
        assert!(layout_mode.is_floating());

        // toggle_maximized
        layout_mode.toggle_maximized();
        assert!(layout_mode.is_maximized());
        layout_mode.toggle_maximized();
        assert!(layout_mode.is_floating());

        // Make base mode tiled
        layout_mode.set_floating(false);
        assert!(layout_mode.is_tiled());

        // set_maximized
        layout_mode.set_maximized(true);
        assert!(layout_mode.is_maximized());
        layout_mode.set_maximized(false);
        assert!(layout_mode.is_tiled());
        layout_mode.set_maximized(false);
        assert!(layout_mode.is_tiled());

        // toggle_fullscreen
        layout_mode.toggle_fullscreen();
        assert!(layout_mode.is_fullscreen());
        layout_mode.toggle_fullscreen();
        assert!(layout_mode.is_tiled());

        // set_fullscreen
        layout_mode.set_fullscreen(false);
        assert!(layout_mode.is_tiled());
        layout_mode.set_fullscreen(true);
        assert!(layout_mode.is_fullscreen());

        // maximized to fullscreen
        layout_mode.toggle_maximized();
        assert!(layout_mode.is_maximized());
        layout_mode.toggle_fullscreen();
        assert!(layout_mode.is_fullscreen());
    }

    #[test]
    fn layout_mode_changes_correctly_when_client_sets_maximized_when_already_maximized() {
        let mut layout_mode = LayoutMode::new_tiled();
        assert!(layout_mode.is_tiled());

        layout_mode.set_maximized(true);
        assert!(layout_mode.is_maximized());
        layout_mode.set_client_maximized(true);
        assert!(layout_mode.is_maximized());
        assert!(layout_mode.client_requested_mode.is_none());
    }

    #[test]
    fn layout_mode_changes_correctly_when_client_sets_maximized_when_not_already_maximized() {
        let mut layout_mode = LayoutMode::new_tiled();
        assert!(layout_mode.is_tiled());

        layout_mode.set_fullscreen(true);
        assert!(layout_mode.is_fullscreen());

        layout_mode.set_client_maximized(true);
        assert!(layout_mode.is_maximized());
        assert_eq!(
            layout_mode.client_requested_mode,
            Some(FullscreenOrMaximized::Maximized)
        );

        layout_mode.set_client_maximized(false);
        assert!(layout_mode.is_fullscreen());
        assert_eq!(layout_mode.client_requested_mode, None);
    }

    #[test]
    fn layout_mode_does_not_change_when_client_requests_to_unset_different_mode() {
        let mut layout_mode = LayoutMode::new_tiled();
        assert!(layout_mode.is_tiled());

        layout_mode.set_fullscreen(true);
        assert!(layout_mode.is_fullscreen());

        layout_mode.set_client_maximized(true);
        assert!(layout_mode.is_maximized());
        assert_eq!(
            layout_mode.client_requested_mode,
            Some(FullscreenOrMaximized::Maximized)
        );

        layout_mode.set_client_fullscreen(false);
        assert!(layout_mode.is_maximized());
        assert_eq!(
            layout_mode.client_requested_mode,
            Some(FullscreenOrMaximized::Maximized)
        );
    }
}
