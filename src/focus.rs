// SPDX-License-Identifier: GPL-3.0-or-later

use smithay::{
    desktop::{LayerSurface, PopupKind},
    input::{
        keyboard::KeyboardTarget,
        pointer::{MotionEvent, PointerTarget},
        Seat,
    },
    output::Output,
    reexports::wayland_server::{protocol::wl_surface::WlSurface, Resource},
    utils::{IsAlive, SERIAL_COUNTER},
    wayland::seat::WaylandFocus,
};

use crate::{
    state::{State, WithState},
    window::WindowElement,
};

impl State {
    /// Get the currently focused window on `output`
    /// that isn't an override redirect window, if any.
    pub fn focused_window(&mut self, output: &Output) -> Option<WindowElement> {
        output.with_state(|state| state.focus_stack.stack.retain(|win| win.alive()));

        let windows = output.with_state(|state| {
            state
                .focus_stack
                .stack
                .iter()
                .rev()
                .filter(|win| {
                    let win_tags = win.with_state(|state| state.tags.clone());
                    let output_tags = state.focused_tags().cloned().collect::<Vec<_>>();

                    win_tags
                        .iter()
                        .any(|win_tag| output_tags.iter().any(|op_tag| win_tag == op_tag))
                })
                .cloned()
                .collect::<Vec<_>>()
        });

        windows
            .into_iter()
            .find(|win| !win.is_x11_override_redirect())
    }

    /// Update the keyboard focus.
    pub fn update_focus(&mut self, output: &Output) {
        let current_focus = self.focused_window(output);

        if let Some(win) = &current_focus {
            assert!(!win.is_x11_override_redirect());

            if let WindowElement::Wayland(w) = win {
                w.toplevel().send_configure();
            }
        }

        self.seat.get_keyboard().expect("no keyboard").set_focus(
            self,
            current_focus.map(|win| win.into()),
            SERIAL_COUNTER.next_serial(),
        );
    }

    pub fn fixup_focus(&mut self) {
        for win in self.z_index_stack.stack.iter() {
            self.space.raise_element(win, false);
        }
    }
}

/// A vector of windows, with the last one being the one in focus and the first
/// being the one at the bottom of the focus stack.
#[derive(Debug)]
pub struct FocusStack<T> {
    pub stack: Vec<T>,
    focused: bool,
}

impl<T> Default for FocusStack<T> {
    fn default() -> Self {
        Self {
            stack: Default::default(),
            focused: Default::default(),
        }
    }
}

impl<T: PartialEq> FocusStack<T> {
    /// Set `focus` to be focused.
    ///
    /// If it's already in the stack, it will be removed then pushed.
    /// If it isn't, it will just be pushed.
    pub fn set_focus(&mut self, focus: T) {
        self.stack.retain(|foc| foc != &focus);
        self.stack.push(focus);
        self.focused = true;
    }

    pub fn unset_focus(&mut self) {
        self.focused = false;
    }

    pub fn current_focus(&self) -> Option<&T> {
        self.focused.then(|| self.stack.last())?
    }
}

/// Different focusable objects.
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
    fn frame(&self, seat: &Seat<State>, data: &mut State) {
        match self {
            FocusTarget::Window(window) => window.frame(seat, data),
            FocusTarget::Popup(popup) => popup.wl_surface().frame(seat, data),
            FocusTarget::LayerSurface(surf) => surf.frame(seat, data),
        }
    }

    fn enter(&self, seat: &Seat<State>, data: &mut State, event: &MotionEvent) {
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
            FocusTarget::Window(
                WindowElement::X11(surface) | WindowElement::X11OverrideRedirect(surface),
            ) => surface.same_client_as(object_id),
            FocusTarget::Popup(popup) => popup.wl_surface().id().same_client_as(object_id),
            FocusTarget::LayerSurface(surf) => surf.wl_surface().id().same_client_as(object_id),
            _ => unreachable!(),
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
