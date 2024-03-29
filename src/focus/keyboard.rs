use smithay::{
    backend::input::KeyState,
    desktop::{LayerSurface, PopupKind, WindowSurface},
    input::{
        keyboard::{KeyboardTarget, KeysymHandle, ModifiersState},
        Seat,
    },
    reexports::wayland_server::{protocol::wl_surface::WlSurface, Resource},
    utils::{IsAlive, Serial},
    wayland::seat::WaylandFocus,
};

use crate::{state::State, window::WindowElement};

/// Keyboard focusable objects
#[derive(Debug, Clone, PartialEq)]
pub enum KeyboardFocusTarget {
    Window(WindowElement),
    Popup(PopupKind),
    LayerSurface(LayerSurface),
}

impl KeyboardTarget<State> for KeyboardFocusTarget {
    fn enter(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        keys: Vec<KeysymHandle<'_>>,
        serial: Serial,
    ) {
        match self {
            KeyboardFocusTarget::Window(window) => {
                KeyboardTarget::enter(window, seat, data, keys, serial)
            }
            KeyboardFocusTarget::Popup(popup) => {
                KeyboardTarget::enter(popup.wl_surface(), seat, data, keys, serial);
            }
            KeyboardFocusTarget::LayerSurface(surf) => {
                KeyboardTarget::enter(surf.wl_surface(), seat, data, keys, serial);
            }
        }
    }

    fn leave(&self, seat: &Seat<State>, data: &mut State, serial: Serial) {
        match self {
            KeyboardFocusTarget::Window(window) => {
                KeyboardTarget::leave(window, seat, data, serial)
            }
            KeyboardFocusTarget::Popup(popup) => {
                KeyboardTarget::leave(popup.wl_surface(), seat, data, serial);
            }
            KeyboardFocusTarget::LayerSurface(surf) => {
                KeyboardTarget::leave(surf.wl_surface(), seat, data, serial)
            }
        }
    }

    fn key(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        key: KeysymHandle<'_>,
        state: smithay::backend::input::KeyState,
        serial: Serial,
        time: u32,
    ) {
        match self {
            KeyboardFocusTarget::Window(window) => {
                KeyboardTarget::key(window, seat, data, key, state, serial, time);
            }
            KeyboardFocusTarget::Popup(popup) => {
                KeyboardTarget::key(popup.wl_surface(), seat, data, key, state, serial, time);
            }
            KeyboardFocusTarget::LayerSurface(surf) => {
                KeyboardTarget::key(surf.wl_surface(), seat, data, key, state, serial, time);
            }
        }
    }

    fn modifiers(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        modifiers: smithay::input::keyboard::ModifiersState,
        serial: Serial,
    ) {
        match self {
            KeyboardFocusTarget::Window(window) => {
                KeyboardTarget::modifiers(window, seat, data, modifiers, serial);
            }
            KeyboardFocusTarget::Popup(popup) => {
                KeyboardTarget::modifiers(popup.wl_surface(), seat, data, modifiers, serial);
            }
            KeyboardFocusTarget::LayerSurface(surf) => {
                KeyboardTarget::modifiers(surf.wl_surface(), seat, data, modifiers, serial);
            }
        }
    }
}

impl IsAlive for KeyboardFocusTarget {
    fn alive(&self) -> bool {
        match self {
            KeyboardFocusTarget::Window(window) => window.alive(),
            KeyboardFocusTarget::Popup(popup) => popup.alive(),
            KeyboardFocusTarget::LayerSurface(surf) => surf.alive(),
        }
    }
}

impl WaylandFocus for KeyboardFocusTarget {
    fn wl_surface(&self) -> Option<WlSurface> {
        match self {
            KeyboardFocusTarget::Window(window) => window.wl_surface(),
            KeyboardFocusTarget::Popup(popup) => Some(popup.wl_surface().clone()),
            KeyboardFocusTarget::LayerSurface(surf) => Some(surf.wl_surface().clone()),
        }
    }

    fn same_client_as(
        &self,
        object_id: &smithay::reexports::wayland_server::backend::ObjectId,
    ) -> bool {
        match self {
            KeyboardFocusTarget::Window(window) => window.same_client_as(object_id),
            KeyboardFocusTarget::Popup(popup) => popup.wl_surface().id().same_client_as(object_id),
            KeyboardFocusTarget::LayerSurface(surf) => {
                surf.wl_surface().id().same_client_as(object_id)
            }
        }
    }
}

impl TryFrom<KeyboardFocusTarget> for WlSurface {
    type Error = ();

    fn try_from(value: KeyboardFocusTarget) -> Result<Self, Self::Error> {
        value.wl_surface().ok_or(())
    }
}

impl From<WindowElement> for KeyboardFocusTarget {
    fn from(value: WindowElement) -> Self {
        KeyboardFocusTarget::Window(value)
    }
}

impl From<PopupKind> for KeyboardFocusTarget {
    fn from(value: PopupKind) -> Self {
        KeyboardFocusTarget::Popup(value)
    }
}

impl From<LayerSurface> for KeyboardFocusTarget {
    fn from(value: LayerSurface) -> Self {
        KeyboardFocusTarget::LayerSurface(value)
    }
}

impl KeyboardTarget<State> for WindowElement {
    fn enter(
        &self,
        seat: &Seat<State>,
        state: &mut State,
        keys: Vec<KeysymHandle<'_>>,
        serial: Serial,
    ) {
        match self.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                KeyboardTarget::enter(toplevel.wl_surface(), seat, state, keys, serial);
            }
            WindowSurface::X11(surface) => {
                KeyboardTarget::enter(surface, seat, state, keys, serial);
            }
        }
    }

    fn leave(&self, seat: &Seat<State>, state: &mut State, serial: Serial) {
        match self.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                KeyboardTarget::leave(toplevel.wl_surface(), seat, state, serial);
            }
            WindowSurface::X11(surface) => KeyboardTarget::leave(surface, seat, state, serial),
        }
    }

    fn key(
        &self,
        seat: &Seat<State>,
        state: &mut State,
        key: KeysymHandle<'_>,
        key_state: KeyState,
        serial: Serial,
        time: u32,
    ) {
        match self.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                KeyboardTarget::key(
                    toplevel.wl_surface(),
                    seat,
                    state,
                    key,
                    key_state,
                    serial,
                    time,
                );
            }
            WindowSurface::X11(surface) => {
                KeyboardTarget::key(surface, seat, state, key, key_state, serial, time);
            }
        }
    }

    fn modifiers(
        &self,
        seat: &Seat<State>,
        state: &mut State,
        modifiers: ModifiersState,
        serial: Serial,
    ) {
        match self.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                KeyboardTarget::modifiers(toplevel.wl_surface(), seat, state, modifiers, serial);
            }
            WindowSurface::X11(surface) => {
                KeyboardTarget::modifiers(surface, seat, state, modifiers, serial);
            }
        }
    }
}
