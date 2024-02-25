// SPDX-License-Identifier: GPL-3.0-or-later

use pinnacle_api_defs::pinnacle::signal::v0alpha1::{
    WindowPointerEnterResponse, WindowPointerLeaveResponse,
};
use smithay::{
    backend::input::KeyState,
    desktop::{layer_map_for_output, LayerSurface, PopupKind, WindowSurface},
    input::{
        keyboard::{KeyboardTarget, KeysymHandle},
        pointer::{
            AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent,
            GesturePinchBeginEvent, GesturePinchEndEvent, GesturePinchUpdateEvent,
            GestureSwipeBeginEvent, GestureSwipeEndEvent, GestureSwipeUpdateEvent, MotionEvent,
            PointerTarget, RelativeMotionEvent,
        },
        touch::{self, TouchTarget},
        Seat,
    },
    output::Output,
    reexports::wayland_server::{backend::ObjectId, protocol::wl_surface::WlSurface},
    utils::{IsAlive, Serial, SERIAL_COUNTER},
    wayland::seat::WaylandFocus,
    xwayland::X11Surface,
};

use crate::{
    state::{State, WithState},
    window::WindowElement,
};

impl State {
    /// Get the currently focused window on `output`
    /// that isn't an override redirect window, if any.
    pub fn keyboard_focused_window(&self, output: &Output) -> Option<WindowElement> {
        // TODO: see if the below is necessary
        // output.with_state(|state| state.focus_stack.stack.retain(|win| win.alive()));

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
        let current_focus = self.keyboard_focused_window(output);

        if let Some(win) = &current_focus {
            assert!(!win.is_x11_override_redirect());

            if let Some(toplevel) = win.toplevel() {
                toplevel.send_configure();
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
pub enum KeyboardFocusTarget {
    Window(WindowElement),
    Popup(PopupKind),
    LayerSurface(LayerSurface),
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

impl TryFrom<KeyboardFocusTarget> for WlSurface {
    type Error = ();

    fn try_from(value: KeyboardFocusTarget) -> Result<Self, Self::Error> {
        value.wl_surface().ok_or(())
    }
}

impl KeyboardTarget<State> for KeyboardFocusTarget {
    fn enter(
        &self,
        seat: &Seat<State>,
        state: &mut State,
        keys: Vec<KeysymHandle<'_>>,
        serial: Serial,
    ) {
        match self {
            KeyboardFocusTarget::Window(window) => match window.underlying_surface() {
                WindowSurface::Wayland(toplevel) => {
                    KeyboardTarget::enter(toplevel.wl_surface(), seat, state, keys, serial);
                }
                WindowSurface::X11(surface) => {
                    KeyboardTarget::enter(surface, seat, state, keys, serial);
                }
            },
            KeyboardFocusTarget::Popup(popup) => {
                KeyboardTarget::enter(popup.wl_surface(), seat, state, keys, serial);
            }
            KeyboardFocusTarget::LayerSurface(layer) => {
                KeyboardTarget::enter(layer.wl_surface(), seat, state, keys, serial);
            }
        }
    }

    fn leave(&self, seat: &Seat<State>, state: &mut State, serial: Serial) {
        match self {
            KeyboardFocusTarget::Window(window) => match window.underlying_surface() {
                WindowSurface::Wayland(toplevel) => {
                    KeyboardTarget::leave(toplevel.wl_surface(), seat, state, serial);
                }
                WindowSurface::X11(surface) => {
                    KeyboardTarget::leave(surface, seat, state, serial);
                }
            },
            KeyboardFocusTarget::Popup(popup) => {
                KeyboardTarget::leave(popup.wl_surface(), seat, state, serial);
            }
            KeyboardFocusTarget::LayerSurface(layer) => {
                KeyboardTarget::leave(layer.wl_surface(), seat, state, serial)
            }
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
        match self {
            KeyboardFocusTarget::Window(window) => match window.underlying_surface() {
                WindowSurface::Wayland(toplevel) => KeyboardTarget::key(
                    toplevel.wl_surface(),
                    seat,
                    state,
                    key,
                    key_state,
                    serial,
                    time,
                ),
                WindowSurface::X11(surface) => {
                    KeyboardTarget::key(surface, seat, state, key, key_state, serial, time)
                }
            },
            KeyboardFocusTarget::Popup(popup) => {
                KeyboardTarget::key(
                    popup.wl_surface(),
                    seat,
                    state,
                    key,
                    key_state,
                    serial,
                    time,
                );
            }
            KeyboardFocusTarget::LayerSurface(layer) => {
                KeyboardTarget::key(
                    layer.wl_surface(),
                    seat,
                    state,
                    key,
                    key_state,
                    serial,
                    time,
                );
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
            KeyboardFocusTarget::Window(window) => match window.underlying_surface() {
                WindowSurface::Wayland(toplevel) => {
                    KeyboardTarget::modifiers(toplevel.wl_surface(), seat, data, modifiers, serial);
                }
                WindowSurface::X11(surface) => {
                    KeyboardTarget::modifiers(surface, seat, data, modifiers, serial);
                }
            },
            KeyboardFocusTarget::Popup(popup) => {
                KeyboardTarget::modifiers(popup.wl_surface(), seat, data, modifiers, serial);
            }
            KeyboardFocusTarget::LayerSurface(surface) => {
                KeyboardTarget::modifiers(surface.wl_surface(), seat, data, modifiers, serial);
            }
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

#[derive(Debug, Clone, PartialEq)]
pub enum PointerFocusTarget {
    WlSurface(WlSurface),
    X11Surface(X11Surface),
}

impl PointerFocusTarget {
    pub fn window_for(&self, state: &State) -> Option<WindowElement> {
        state
            .windows
            .iter()
            .find(|win| {
                win.wl_surface()
                    .is_some_and(|surf| Some(surf) == self.wl_surface())
            })
            .cloned()
    }

    pub fn layer_for(&self, state: &State) -> Option<LayerSurface> {
        state
            .space
            .outputs()
            .map(|op| layer_map_for_output(op))
            .flat_map(|map| map.layers().cloned().collect::<Vec<_>>())
            .find(|layer| Some(layer.wl_surface()) == self.wl_surface().as_ref())
    }
}

impl IsAlive for PointerFocusTarget {
    fn alive(&self) -> bool {
        match self {
            PointerFocusTarget::WlSurface(surface) => surface.alive(),
            PointerFocusTarget::X11Surface(surface) => surface.alive(),
        }
    }
}

impl TryFrom<PointerFocusTarget> for WlSurface {
    type Error = ();

    fn try_from(value: PointerFocusTarget) -> Result<Self, Self::Error> {
        value.wl_surface().ok_or(())
    }
}

// Yikes fallible `From`
impl From<KeyboardFocusTarget> for PointerFocusTarget {
    fn from(target: KeyboardFocusTarget) -> Self {
        Self::WlSurface(
            target
                .wl_surface()
                .expect("keyboard target had no wl surface"),
        )
    }
}

impl From<&WindowElement> for PointerFocusTarget {
    fn from(win: &WindowElement) -> Self {
        match win.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                PointerFocusTarget::WlSurface(toplevel.wl_surface().clone())
            }
            WindowSurface::X11(surface) => PointerFocusTarget::X11Surface(surface.clone()),
        }
    }
}

impl From<&LayerSurface> for PointerFocusTarget {
    fn from(layer: &LayerSurface) -> Self {
        PointerFocusTarget::WlSurface(layer.wl_surface().clone())
    }
}

impl WaylandFocus for PointerFocusTarget {
    fn wl_surface(&self) -> Option<WlSurface> {
        match self {
            PointerFocusTarget::WlSurface(surface) => surface.wl_surface(),
            PointerFocusTarget::X11Surface(surface) => surface.wl_surface(),
        }
    }

    fn same_client_as(&self, object_id: &ObjectId) -> bool {
        match self {
            PointerFocusTarget::WlSurface(surface) => surface.same_client_as(object_id),
            PointerFocusTarget::X11Surface(surface) => surface.same_client_as(object_id),
        }
    }
}

impl PointerTarget<State> for PointerFocusTarget {
    fn enter(&self, seat: &Seat<State>, data: &mut State, event: &MotionEvent) {
        match self {
            PointerFocusTarget::WlSurface(surf) => PointerTarget::enter(surf, seat, data, event),
            PointerFocusTarget::X11Surface(surf) => PointerTarget::enter(surf, seat, data, event),
        }

        // FIXME:
        // if let Some(window) = self.window_for(data) {
        //     let window_id = Some(window.with_state(|state| state.id.0));
        //
        //     data.signal_state
        //         .window_pointer_enter
        //         .signal(|buffer| buffer.push_back(WindowPointerEnterResponse { window_id }));
        // }
    }

    fn motion(&self, seat: &Seat<State>, data: &mut State, event: &MotionEvent) {
        match self {
            PointerFocusTarget::WlSurface(surf) => PointerTarget::motion(surf, seat, data, event),
            PointerFocusTarget::X11Surface(surf) => PointerTarget::motion(surf, seat, data, event),
        }
    }

    fn relative_motion(&self, seat: &Seat<State>, data: &mut State, event: &RelativeMotionEvent) {
        match self {
            PointerFocusTarget::WlSurface(surf) => {
                PointerTarget::relative_motion(surf, seat, data, event);
            }
            PointerFocusTarget::X11Surface(surf) => {
                PointerTarget::relative_motion(surf, seat, data, event);
            }
        }
    }

    fn button(&self, seat: &Seat<State>, data: &mut State, event: &ButtonEvent) {
        match self {
            PointerFocusTarget::WlSurface(surf) => PointerTarget::button(surf, seat, data, event),
            PointerFocusTarget::X11Surface(surf) => PointerTarget::button(surf, seat, data, event),
        }
    }

    fn axis(&self, seat: &Seat<State>, data: &mut State, frame: AxisFrame) {
        match self {
            PointerFocusTarget::WlSurface(surf) => PointerTarget::axis(surf, seat, data, frame),
            PointerFocusTarget::X11Surface(surf) => PointerTarget::axis(surf, seat, data, frame),
        }
    }

    fn frame(&self, seat: &Seat<State>, data: &mut State) {
        match self {
            PointerFocusTarget::WlSurface(surf) => PointerTarget::frame(surf, seat, data),
            PointerFocusTarget::X11Surface(surf) => PointerTarget::frame(surf, seat, data),
        }
    }

    fn gesture_swipe_begin(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        event: &GestureSwipeBeginEvent,
    ) {
        match self {
            PointerFocusTarget::WlSurface(surf) => {
                PointerTarget::gesture_swipe_begin(surf, seat, data, event);
            }
            PointerFocusTarget::X11Surface(surf) => {
                PointerTarget::gesture_swipe_begin(surf, seat, data, event);
            }
        }
    }

    fn gesture_swipe_update(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        event: &GestureSwipeUpdateEvent,
    ) {
        match self {
            PointerFocusTarget::WlSurface(surf) => {
                PointerTarget::gesture_swipe_update(surf, seat, data, event);
            }
            PointerFocusTarget::X11Surface(surf) => {
                PointerTarget::gesture_swipe_update(surf, seat, data, event);
            }
        }
    }

    fn gesture_swipe_end(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        event: &GestureSwipeEndEvent,
    ) {
        match self {
            PointerFocusTarget::WlSurface(surf) => {
                PointerTarget::gesture_swipe_end(surf, seat, data, event);
            }
            PointerFocusTarget::X11Surface(surf) => {
                PointerTarget::gesture_swipe_end(surf, seat, data, event);
            }
        }
    }

    fn gesture_pinch_begin(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        event: &GesturePinchBeginEvent,
    ) {
        match self {
            PointerFocusTarget::WlSurface(surf) => {
                PointerTarget::gesture_pinch_begin(surf, seat, data, event);
            }
            PointerFocusTarget::X11Surface(surf) => {
                PointerTarget::gesture_pinch_begin(surf, seat, data, event);
            }
        }
    }

    fn gesture_pinch_update(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        event: &GesturePinchUpdateEvent,
    ) {
        match self {
            PointerFocusTarget::WlSurface(surf) => {
                PointerTarget::gesture_pinch_update(surf, seat, data, event);
            }
            PointerFocusTarget::X11Surface(surf) => {
                PointerTarget::gesture_pinch_update(surf, seat, data, event);
            }
        }
    }

    fn gesture_pinch_end(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        event: &GesturePinchEndEvent,
    ) {
        match self {
            PointerFocusTarget::WlSurface(surf) => {
                PointerTarget::gesture_pinch_end(surf, seat, data, event);
            }
            PointerFocusTarget::X11Surface(surf) => {
                PointerTarget::gesture_pinch_end(surf, seat, data, event);
            }
        }
    }

    fn gesture_hold_begin(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        event: &GestureHoldBeginEvent,
    ) {
        match self {
            PointerFocusTarget::WlSurface(surf) => {
                PointerTarget::gesture_hold_begin(surf, seat, data, event);
            }
            PointerFocusTarget::X11Surface(surf) => {
                PointerTarget::gesture_hold_begin(surf, seat, data, event);
            }
        }
    }

    fn gesture_hold_end(&self, seat: &Seat<State>, data: &mut State, event: &GestureHoldEndEvent) {
        match self {
            PointerFocusTarget::WlSurface(surf) => {
                PointerTarget::gesture_hold_end(surf, seat, data, event);
            }
            PointerFocusTarget::X11Surface(surf) => {
                PointerTarget::gesture_hold_end(surf, seat, data, event);
            }
        }
    }

    fn leave(&self, seat: &Seat<State>, data: &mut State, serial: Serial, time: u32) {
        match self {
            PointerFocusTarget::WlSurface(surf) => {
                PointerTarget::leave(surf, seat, data, serial, time);
            }
            PointerFocusTarget::X11Surface(surf) => {
                PointerTarget::leave(surf, seat, data, serial, time);
            }
        }

        if let Some(window) = self.window_for(data) {
            let window_id = Some(window.with_state(|state| state.id.0));

            data.signal_state
                .window_pointer_leave
                .signal(|buffer| buffer.push_back(WindowPointerLeaveResponse { window_id }));
        }
    }
}

impl TouchTarget<State> for PointerFocusTarget {
    fn down(&self, seat: &Seat<State>, data: &mut State, event: &touch::DownEvent, seq: Serial) {
        match self {
            PointerFocusTarget::WlSurface(surf) => TouchTarget::down(surf, seat, data, event, seq),
            PointerFocusTarget::X11Surface(surf) => TouchTarget::down(surf, seat, data, event, seq),
        }
    }

    fn up(&self, seat: &Seat<State>, data: &mut State, event: &touch::UpEvent, seq: Serial) {
        match self {
            PointerFocusTarget::WlSurface(surf) => TouchTarget::up(surf, seat, data, event, seq),
            PointerFocusTarget::X11Surface(surf) => TouchTarget::up(surf, seat, data, event, seq),
        }
    }

    fn motion(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        event: &touch::MotionEvent,
        seq: Serial,
    ) {
        match self {
            PointerFocusTarget::WlSurface(surf) => {
                TouchTarget::motion(surf, seat, data, event, seq);
            }
            PointerFocusTarget::X11Surface(surf) => {
                TouchTarget::motion(surf, seat, data, event, seq);
            }
        }
    }

    fn frame(&self, seat: &Seat<State>, data: &mut State, seq: Serial) {
        match self {
            PointerFocusTarget::WlSurface(surf) => TouchTarget::frame(surf, seat, data, seq),
            PointerFocusTarget::X11Surface(surf) => TouchTarget::frame(surf, seat, data, seq),
        }
    }

    fn cancel(&self, seat: &Seat<State>, data: &mut State, seq: Serial) {
        match self {
            PointerFocusTarget::WlSurface(surf) => TouchTarget::cancel(surf, seat, data, seq),
            PointerFocusTarget::X11Surface(surf) => TouchTarget::cancel(surf, seat, data, seq),
        }
    }

    fn shape(&self, seat: &Seat<State>, data: &mut State, event: &touch::ShapeEvent, seq: Serial) {
        match self {
            PointerFocusTarget::WlSurface(surf) => TouchTarget::shape(surf, seat, data, event, seq),
            PointerFocusTarget::X11Surface(surf) => {
                TouchTarget::shape(surf, seat, data, event, seq);
            }
        }
    }

    fn orientation(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        event: &touch::OrientationEvent,
        seq: Serial,
    ) {
        match self {
            PointerFocusTarget::WlSurface(surf) => {
                TouchTarget::orientation(surf, seat, data, event, seq);
            }
            PointerFocusTarget::X11Surface(surf) => {
                TouchTarget::orientation(surf, seat, data, event, seq);
            }
        }
    }
}
