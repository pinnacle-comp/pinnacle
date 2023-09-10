// SPDX-License-Identifier: GPL-3.0-or-later

use smithay::{
    desktop::{LayerSurface, PopupKind, Space},
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

use crate::{
    state::{State, WithState},
    window::WindowElement,
};

#[derive(Default)]
pub struct FocusState {
    pub focus_stack: Vec<WindowElement>,
    pub focused_output: Option<Output>,
}

impl State {
    /// Get the currently focused window on `output`, if any.
    pub fn current_focus(&mut self, output: &Output) -> Option<WindowElement> {
        self.focus_state.focus_stack.retain(|win| win.alive());

        let mut windows = self.focus_state.focus_stack.iter().rev().filter(|win| {
            let win_tags = win.with_state(|state| state.tags.clone());
            let output_tags =
                output.with_state(|state| state.focused_tags().cloned().collect::<Vec<_>>());

            win_tags
                .iter()
                .any(|win_tag| output_tags.iter().any(|op_tag| win_tag == op_tag))
        });

        windows.next().cloned()
    }
}

impl FocusState {
    pub fn new() -> Self {
        Default::default()
    }

    /// Set the currently focused window.
    pub fn set_focus(&mut self, window: WindowElement) {
        self.focus_stack.retain(|win| win != &window);
        self.focus_stack.push(window);
    }

    /// Fix focus layering for all windows in the `focus_stack`.
    ///
    /// This will call `space.raise_element` on all windows from back
    /// to front to correct their z locations.
    pub fn fix_up_focus(&self, space: &mut Space<WindowElement>) {
        for win in self.focus_stack.iter() {
            space.raise_element(win, false);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FocusTarget {
    Window(WindowElement),
    Popup(PopupKind),
    LayerSurface(LayerSurface),
}

impl IsAlive for FocusTarget {
    fn alive(&self) -> bool {
        match self {
            FocusTarget::Window(window) => window.alive(),
            FocusTarget::Popup(popup) => popup.alive(),
            FocusTarget::LayerSurface(surf) => surf.alive(),
        }
    }
}

impl TryFrom<FocusTarget> for WlSurface {
    type Error = ();

    fn try_from(value: FocusTarget) -> Result<Self, Self::Error> {
        value.wl_surface().ok_or(())
    }
}

impl PointerTarget<State> for FocusTarget {
    fn enter(&self, seat: &Seat<State>, data: &mut State, event: &MotionEvent) {
        // tracing::debug!("Pointer enter on {self:?}");
        match self {
            FocusTarget::Window(window) => PointerTarget::enter(window, seat, data, event),
            FocusTarget::Popup(popup) => {
                PointerTarget::enter(popup.wl_surface(), seat, data, event);
            }
            FocusTarget::LayerSurface(surf) => PointerTarget::enter(surf, seat, data, event),
        }
    }

    fn motion(&self, seat: &Seat<State>, data: &mut State, event: &MotionEvent) {
        match self {
            FocusTarget::Window(window) => PointerTarget::motion(window, seat, data, event),
            FocusTarget::Popup(popup) => {
                PointerTarget::motion(popup.wl_surface(), seat, data, event);
            }
            FocusTarget::LayerSurface(surf) => PointerTarget::motion(surf, seat, data, event),
        }
    }

    fn relative_motion(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        event: &smithay::input::pointer::RelativeMotionEvent,
    ) {
        match self {
            FocusTarget::Window(window) => {
                PointerTarget::relative_motion(window, seat, data, event);
            }
            FocusTarget::Popup(popup) => {
                PointerTarget::relative_motion(popup.wl_surface(), seat, data, event);
            }
            FocusTarget::LayerSurface(surf) => {
                PointerTarget::relative_motion(surf, seat, data, event);
            }
        }
    }

    fn button(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        event: &smithay::input::pointer::ButtonEvent,
    ) {
        match self {
            FocusTarget::Window(window) => PointerTarget::button(window, seat, data, event),
            FocusTarget::Popup(popup) => {
                PointerTarget::button(popup.wl_surface(), seat, data, event);
            }
            FocusTarget::LayerSurface(surf) => PointerTarget::button(surf, seat, data, event),
        }
    }

    fn axis(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        frame: smithay::input::pointer::AxisFrame,
    ) {
        match self {
            FocusTarget::Window(window) => PointerTarget::axis(window, seat, data, frame),
            FocusTarget::Popup(popup) => PointerTarget::axis(popup.wl_surface(), seat, data, frame),
            FocusTarget::LayerSurface(surf) => PointerTarget::axis(surf, seat, data, frame),
        }
    }

    fn leave(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        serial: smithay::utils::Serial,
        time: u32,
    ) {
        // tracing::debug!("Pointer leave on {self:?}");
        match self {
            FocusTarget::Window(window) => PointerTarget::leave(window, seat, data, serial, time),
            FocusTarget::Popup(popup) => {
                PointerTarget::leave(popup.wl_surface(), seat, data, serial, time);
            }
            FocusTarget::LayerSurface(surf) => PointerTarget::leave(surf, seat, data, serial, time),
        }
    }

    fn gesture_swipe_begin(
        &self,
        _seat: &Seat<State>,
        _data: &mut State,
        _event: &smithay::input::pointer::GestureSwipeBeginEvent,
    ) {
        todo!()
    }

    fn gesture_swipe_update(
        &self,
        _seat: &Seat<State>,
        _data: &mut State,
        _event: &smithay::input::pointer::GestureSwipeUpdateEvent,
    ) {
        todo!()
    }

    fn gesture_swipe_end(
        &self,
        _seat: &Seat<State>,
        _data: &mut State,
        _event: &smithay::input::pointer::GestureSwipeEndEvent,
    ) {
        todo!()
    }

    fn gesture_pinch_begin(
        &self,
        _seat: &Seat<State>,
        _data: &mut State,
        _event: &smithay::input::pointer::GesturePinchBeginEvent,
    ) {
        todo!()
    }

    fn gesture_pinch_update(
        &self,
        _seat: &Seat<State>,
        _data: &mut State,
        _event: &smithay::input::pointer::GesturePinchUpdateEvent,
    ) {
        todo!()
    }

    fn gesture_pinch_end(
        &self,
        _seat: &Seat<State>,
        _data: &mut State,
        _event: &smithay::input::pointer::GesturePinchEndEvent,
    ) {
        todo!()
    }

    fn gesture_hold_begin(
        &self,
        _seat: &Seat<State>,
        _data: &mut State,
        _event: &smithay::input::pointer::GestureHoldBeginEvent,
    ) {
        todo!()
    }

    fn gesture_hold_end(
        &self,
        _seat: &Seat<State>,
        _data: &mut State,
        _event: &smithay::input::pointer::GestureHoldEndEvent,
    ) {
        todo!()
    }
}

impl KeyboardTarget<State> for FocusTarget {
    fn enter(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        keys: Vec<smithay::input::keyboard::KeysymHandle<'_>>,
        serial: smithay::utils::Serial,
    ) {
        match self {
            FocusTarget::Window(window) => KeyboardTarget::enter(window, seat, data, keys, serial),
            FocusTarget::Popup(popup) => {
                KeyboardTarget::enter(popup.wl_surface(), seat, data, keys, serial);
            }
            FocusTarget::LayerSurface(surf) => {
                KeyboardTarget::enter(surf, seat, data, keys, serial);
            }
        }
    }

    fn leave(&self, seat: &Seat<State>, data: &mut State, serial: smithay::utils::Serial) {
        match self {
            FocusTarget::Window(window) => KeyboardTarget::leave(window, seat, data, serial),
            FocusTarget::Popup(popup) => {
                KeyboardTarget::leave(popup.wl_surface(), seat, data, serial);
            }
            FocusTarget::LayerSurface(surf) => KeyboardTarget::leave(surf, seat, data, serial),
        }
    }

    fn key(
        &self,
        seat: &Seat<State>,
        data: &mut State,
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
            FocusTarget::LayerSurface(surf) => {
                KeyboardTarget::key(surf, seat, data, key, state, serial, time);
            }
        }
    }

    fn modifiers(
        &self,
        seat: &Seat<State>,
        data: &mut State,
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
            FocusTarget::LayerSurface(surf) => {
                KeyboardTarget::modifiers(surf, seat, data, modifiers, serial);
            }
        }
    }
}

impl WaylandFocus for FocusTarget {
    fn wl_surface(&self) -> Option<WlSurface> {
        match self {
            FocusTarget::Window(window) => window.wl_surface(),
            FocusTarget::Popup(popup) => Some(popup.wl_surface().clone()),
            FocusTarget::LayerSurface(surf) => Some(surf.wl_surface().clone()),
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
            FocusTarget::LayerSurface(surf) => surf.wl_surface().id().same_client_as(object_id),
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

impl From<LayerSurface> for FocusTarget {
    fn from(value: LayerSurface) -> Self {
        FocusTarget::LayerSurface(value)
    }
}
