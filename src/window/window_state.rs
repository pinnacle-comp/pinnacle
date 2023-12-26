// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::atomic::{AtomicU32, Ordering};

use smithay::{
    desktop::space::SpaceElement,
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel,
    utils::{Logical, Point, Rectangle, Serial},
};

use crate::{
    state::{State, WithState},
    tag::Tag,
};

use super::WindowElement;

/// A unique identifier for each window.
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WindowId {
    /// A config API returned an invalid window. It should be using this variant.
    None,
    /// A valid window id.
    #[serde(untagged)]
    Some(u32),
}

static WINDOW_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

impl WindowId {
    /// Get the next available window id. This always starts at 0.
    pub fn next() -> Self {
        Self::Some(WINDOW_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the window that has this WindowId.
    pub fn window(&self, state: &State) -> Option<WindowElement> {
        state
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
    /// The window's resize state. See [WindowResizeState] for more.
    pub loc_request_state: LocationRequestState,
    /// What tags the window is currently on.
    pub tags: Vec<Tag>,
    pub floating_or_tiled: FloatingOrTiled,
    pub fullscreen_or_maximized: FullscreenOrMaximized,
}

/// The state of a window's resize operation.
#[derive(Debug, Default, Clone)]
pub enum LocationRequestState {
    /// The window doesn't need to be moved.
    #[default]
    Idle,
    Sent(Point<i32, Logical>),
    /// The window has received a configure request with a new size. The desired location and the
    /// configure request's serial should be provided here.
    Requested(Serial, Point<i32, Logical>),
    /// The client has received the configure request and has successfully changed its size. It's
    /// now safe to move the window in [`CompositorHandler.commit()`] without flickering.
    ///
    /// [`CompositorHandler.commit()`]: smithay::wayland::compositor::CompositorHandler#tymethod.commit
    Acknowledged(Point<i32, Logical>),
}

impl WindowElement {
    /// RefCell Safety: This method uses a [`RefCell`] on this window.
    pub fn toggle_floating(&self) {
        match self.with_state(|state| state.floating_or_tiled) {
            FloatingOrTiled::Floating(current_rect) => {
                self.with_state(|state| {
                    state.floating_or_tiled = FloatingOrTiled::Tiled(Some(current_rect))
                });
                self.set_tiled_states();
            }
            FloatingOrTiled::Tiled(prev_rect) => {
                let prev_rect = prev_rect.unwrap_or_else(|| self.geometry());

                self.with_state(|state| {
                    state.floating_or_tiled = FloatingOrTiled::Floating(prev_rect);
                });

                // TODO: maybe move this into update_windows
                self.change_geometry(prev_rect);
                self.set_floating_states();
            }
        }
    }

    /// RefCell Safety: This method uses a [`RefCell`] on this window.
    pub fn toggle_fullscreen(&self) {
        match self.with_state(|state| state.fullscreen_or_maximized) {
            FullscreenOrMaximized::Neither | FullscreenOrMaximized::Maximized => {
                self.with_state(|state| {
                    state.fullscreen_or_maximized = FullscreenOrMaximized::Fullscreen;
                });

                match self {
                    WindowElement::Wayland(window) => {
                        window.toplevel().with_pending_state(|state| {
                            state.states.unset(xdg_toplevel::State::Maximized);
                            state.states.set(xdg_toplevel::State::Fullscreen);
                            state.states.set(xdg_toplevel::State::TiledTop);
                            state.states.set(xdg_toplevel::State::TiledLeft);
                            state.states.set(xdg_toplevel::State::TiledBottom);
                            state.states.set(xdg_toplevel::State::TiledRight);
                        });
                    }
                    WindowElement::X11(surface) => {
                        surface
                            .set_maximized(false)
                            .expect("failed to set x11 win to maximized");
                        surface
                            .set_fullscreen(true)
                            .expect("failed to set x11 win to not fullscreen");
                    }
                    WindowElement::X11OverrideRedirect(_) => (),
                    _ => unreachable!(),
                }
            }
            FullscreenOrMaximized::Fullscreen => {
                self.with_state(|state| {
                    state.fullscreen_or_maximized = FullscreenOrMaximized::Neither;
                });

                match self.with_state(|state| state.floating_or_tiled) {
                    FloatingOrTiled::Floating(current_rect) => {
                        self.change_geometry(current_rect);
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
                self.with_state(|state| {
                    state.fullscreen_or_maximized = FullscreenOrMaximized::Maximized;
                });

                match self {
                    WindowElement::Wayland(window) => {
                        window.toplevel().with_pending_state(|state| {
                            state.states.set(xdg_toplevel::State::Maximized);
                            state.states.unset(xdg_toplevel::State::Fullscreen);
                            state.states.set(xdg_toplevel::State::TiledTop);
                            state.states.set(xdg_toplevel::State::TiledLeft);
                            state.states.set(xdg_toplevel::State::TiledBottom);
                            state.states.set(xdg_toplevel::State::TiledRight);
                        });
                    }
                    WindowElement::X11(surface) => {
                        surface
                            .set_maximized(true)
                            .expect("failed to set x11 win to maximized");
                        surface
                            .set_fullscreen(false)
                            .expect("failed to set x11 win to not fullscreen");
                    }
                    WindowElement::X11OverrideRedirect(_) => (),
                    _ => unreachable!(),
                }
            }
            FullscreenOrMaximized::Maximized => {
                self.with_state(|state| {
                    state.fullscreen_or_maximized = FullscreenOrMaximized::Neither;
                });

                match self.with_state(|state| state.floating_or_tiled) {
                    FloatingOrTiled::Floating(current_rect) => {
                        self.change_geometry(current_rect);
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
        match self {
            WindowElement::Wayland(window) => {
                window.toplevel().with_pending_state(|state| {
                    state.states.unset(xdg_toplevel::State::Maximized);
                    state.states.unset(xdg_toplevel::State::Fullscreen);
                    state.states.unset(xdg_toplevel::State::TiledTop);
                    state.states.unset(xdg_toplevel::State::TiledLeft);
                    state.states.unset(xdg_toplevel::State::TiledBottom);
                    state.states.unset(xdg_toplevel::State::TiledRight);
                });
            }
            WindowElement::X11(surface) => {
                surface
                    .set_maximized(false)
                    .expect("failed to set x11 win to maximized");
                surface
                    .set_fullscreen(false)
                    .expect("failed to set x11 win to not fullscreen");
            }
            WindowElement::X11OverrideRedirect(_) => (),
            _ => unreachable!(),
        }
    }

    /// Unsets maximized and fullscreen states for both wayland and xwayland windows
    /// and sets tiled states for wayland windows.
    fn set_tiled_states(&self) {
        match self {
            WindowElement::Wayland(window) => {
                window.toplevel().with_pending_state(|state| {
                    state.states.unset(xdg_toplevel::State::Maximized);
                    state.states.unset(xdg_toplevel::State::Fullscreen);
                    state.states.set(xdg_toplevel::State::TiledTop);
                    state.states.set(xdg_toplevel::State::TiledLeft);
                    state.states.set(xdg_toplevel::State::TiledBottom);
                    state.states.set(xdg_toplevel::State::TiledRight);
                });
            }
            WindowElement::X11(surface) => {
                surface
                    .set_maximized(false)
                    .expect("failed to set x11 win to maximized");
                surface
                    .set_fullscreen(false)
                    .expect("failed to set x11 win to not fullscreen");
            }
            WindowElement::X11OverrideRedirect(_) => (),
            _ => unreachable!(),
        }
    }
}

/// Whether a window is floating or tiled
#[derive(Debug, Clone, Copy)]
pub enum FloatingOrTiled {
    /// The window is floating with the specified geometry.
    Floating(Rectangle<i32, Logical>),
    /// The window is tiled.
    ///
    /// The previous geometry it had when it was floating is stored here.
    /// This is so when it becomes floating again, it returns to this geometry.
    Tiled(Option<Rectangle<i32, Logical>>),
}

impl FloatingOrTiled {
    /// Returns `true` if the floating or tiled is [`Floating`].
    ///
    /// [`Floating`]: FloatingOrTiled::Floating
    #[must_use]
    pub fn is_floating(&self) -> bool {
        matches!(self, Self::Floating(..))
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
            loc_request_state: LocationRequestState::Idle,
            tags: vec![],
            floating_or_tiled: FloatingOrTiled::Tiled(None),
            fullscreen_or_maximized: FullscreenOrMaximized::Neither,
        }
    }
}
