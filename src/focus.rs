// SPDX-License-Identifier: GPL-3.0-or-later

use smithay::{
    desktop::PopupKind,
    input::{
        keyboard::KeyboardTarget,
        pointer::{MotionEvent, PointerTarget},
        Seat,
    },
    output::Output,
    reexports::wayland_server::{protocol::wl_surface::WlSurface, Resource},
    utils::IsAlive,
    wayland::seat::WaylandFocus,
};

use crate::{backend::Backend, state::State, window::WindowElement};

#[derive(Default)]
pub struct FocusState {
    focus_stack: Vec<WindowElement>,
    pub focused_output: Option<Output>,
}

impl FocusState {
    pub fn new() -> Self {
        Default::default()
    }

    // TODO: how does this work with unmapped windows?
    /// Get the currently focused window. If there is none, the previous focus is returned.
    pub fn current_focus(&mut self) -> Option<WindowElement> {
        while let Some(window) = self.focus_stack.last() {
            if window.alive() {
                return Some(window.clone());
            }
            self.focus_stack.pop();
        }
        None
    }

    /// Set the currently focused window.
    pub fn set_focus(&mut self, window: WindowElement) {
        self.focus_stack.retain(|win| win != &window);
        self.focus_stack.push(window);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FocusTarget {
    Window(WindowElement),
    Popup(PopupKind),
    // TODO: LayerSurface
}

impl IsAlive for FocusTarget {
    fn alive(&self) -> bool {
        match self {
            FocusTarget::Window(window) => window.alive(),
            FocusTarget::Popup(popup) => popup.alive(),
        }
    }
}

impl From<FocusTarget> for WlSurface {
    fn from(value: FocusTarget) -> Self {
        value.wl_surface().expect("no wl_surface")
    }
}

impl<B: Backend> PointerTarget<State<B>> for FocusTarget {
    fn enter(&self, seat: &Seat<State<B>>, data: &mut State<B>, event: &MotionEvent) {
        match self {
            FocusTarget::Window(window) => PointerTarget::enter(window, seat, data, event),
            FocusTarget::Popup(popup) => {
                PointerTarget::enter(popup.wl_surface(), seat, data, event);
            }
        }
    }

    fn motion(&self, seat: &Seat<State<B>>, data: &mut State<B>, event: &MotionEvent) {
        match self {
            FocusTarget::Window(window) => PointerTarget::motion(window, seat, data, event),
            FocusTarget::Popup(popup) => {
                PointerTarget::motion(popup.wl_surface(), seat, data, event);
            }
        }
    }

    fn relative_motion(
        &self,
        seat: &Seat<State<B>>,
        data: &mut State<B>,
        event: &smithay::input::pointer::RelativeMotionEvent,
    ) {
        match self {
            FocusTarget::Window(window) => {
                PointerTarget::relative_motion(window, seat, data, event);
            }
            FocusTarget::Popup(popup) => {
                PointerTarget::relative_motion(popup.wl_surface(), seat, data, event);
            }
        }
    }

    fn button(
        &self,
        seat: &Seat<State<B>>,
        data: &mut State<B>,
        event: &smithay::input::pointer::ButtonEvent,
    ) {
        match self {
            FocusTarget::Window(window) => PointerTarget::button(window, seat, data, event),
            FocusTarget::Popup(popup) => {
                PointerTarget::button(popup.wl_surface(), seat, data, event);
            }
        }
    }

    fn axis(
        &self,
        seat: &Seat<State<B>>,
        data: &mut State<B>,
        frame: smithay::input::pointer::AxisFrame,
    ) {
        match self {
            FocusTarget::Window(window) => PointerTarget::axis(window, seat, data, frame),
            FocusTarget::Popup(popup) => PointerTarget::axis(popup.wl_surface(), seat, data, frame),
        }
    }

    fn leave(
        &self,
        seat: &Seat<State<B>>,
        data: &mut State<B>,
        serial: smithay::utils::Serial,
        time: u32,
    ) {
        match self {
            FocusTarget::Window(window) => PointerTarget::leave(window, seat, data, serial, time),
            FocusTarget::Popup(popup) => {
                PointerTarget::leave(popup.wl_surface(), seat, data, serial, time);
            }
        }
    }
}

impl<B: Backend> KeyboardTarget<State<B>> for FocusTarget {
    fn enter(
        &self,
        seat: &Seat<State<B>>,
        data: &mut State<B>,
        keys: Vec<smithay::input::keyboard::KeysymHandle<'_>>,
        serial: smithay::utils::Serial,
    ) {
        match self {
            FocusTarget::Window(window) => KeyboardTarget::enter(window, seat, data, keys, serial),
            FocusTarget::Popup(popup) => {
                KeyboardTarget::enter(popup.wl_surface(), seat, data, keys, serial);
            }
        }
    }

    fn leave(&self, seat: &Seat<State<B>>, data: &mut State<B>, serial: smithay::utils::Serial) {
        match self {
            FocusTarget::Window(window) => KeyboardTarget::leave(window, seat, data, serial),
            FocusTarget::Popup(popup) => {
                KeyboardTarget::leave(popup.wl_surface(), seat, data, serial);
            }
        }
    }

    fn key(
        &self,
        seat: &Seat<State<B>>,
        data: &mut State<B>,
        key: smithay::input::keyboard::KeysymHandle<'_>,
        state: smithay::backend::input::KeyState,
        serial: smithay::utils::Serial,
        time: u32,
    ) {
        match self {
            FocusTarget::Window(window) => {
                KeyboardTarget::key(window, seat, data, key, state, serial, time);
            }
            FocusTarget::Popup(popup) => {
                KeyboardTarget::key(popup.wl_surface(), seat, data, key, state, serial, time);
            }
        }
    }

    fn modifiers(
        &self,
        seat: &Seat<State<B>>,
        data: &mut State<B>,
        modifiers: smithay::input::keyboard::ModifiersState,
        serial: smithay::utils::Serial,
    ) {
        match self {
            FocusTarget::Window(window) => {
                KeyboardTarget::modifiers(window, seat, data, modifiers, serial);
            }
            FocusTarget::Popup(popup) => {
                KeyboardTarget::modifiers(popup.wl_surface(), seat, data, modifiers, serial);
            }
        }
    }
}

impl WaylandFocus for FocusTarget {
    fn wl_surface(&self) -> Option<WlSurface> {
        match self {
            FocusTarget::Window(window) => window.wl_surface(),
            FocusTarget::Popup(popup) => Some(popup.wl_surface().clone()),
        }
    }

    fn same_client_as(
        &self,
        object_id: &smithay::reexports::wayland_server::backend::ObjectId,
    ) -> bool {
        match self {
            FocusTarget::Window(WindowElement::Wayland(window)) => window.same_client_as(object_id),
            FocusTarget::Window(WindowElement::X11(surface)) => surface.same_client_as(object_id),
            FocusTarget::Popup(popup) => popup.wl_surface().id().same_client_as(object_id),
        }
    }
}

impl From<WindowElement> for FocusTarget {
    fn from(value: WindowElement) -> Self {
        FocusTarget::Window(value)
    }
}

impl From<PopupKind> for FocusTarget {
    fn from(value: PopupKind) -> Self {
        FocusTarget::Popup(value)
    }
}
