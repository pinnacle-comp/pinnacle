use pinnacle_api_defs::pinnacle::signal::v0alpha1::{
    WindowPointerEnterResponse, WindowPointerLeaveResponse,
};
use smithay::{
    desktop::{
        layer_map_for_output, utils::with_surfaces_surface_tree, LayerSurface, PopupKind,
        WindowSurface,
    },
    input::{
        pointer::{self, PointerTarget},
        touch::{self, TouchTarget},
        Seat,
    },
    reexports::wayland_server::{backend::ObjectId, protocol::wl_surface::WlSurface},
    utils::{IsAlive, Serial},
    wayland::seat::WaylandFocus,
    xwayland::X11Surface,
};

use crate::{
    state::{State, WithState},
    window::WindowElement,
};

use super::keyboard::KeyboardFocusTarget;

#[derive(Debug, Clone, PartialEq)]
pub enum PointerFocusTarget {
    WlSurface(WlSurface),
    X11Surface(X11Surface),
}

impl PointerFocusTarget {
    /// If the pointer focus's surface is owned by a window, get that window.
    pub fn window_for(&self, state: &State) -> Option<WindowElement> {
        match self {
            PointerFocusTarget::WlSurface(surf) => state
                .pinnacle
                .windows
                .iter()
                .find(|win| {
                    let Some(surface) = win.wl_surface() else {
                        return false;
                    };
                    let mut found = false;
                    with_surfaces_surface_tree(&surface, |surface, _| {
                        if surface == surf {
                            found = true;
                        }
                    });
                    found
                })
                .cloned(),
            PointerFocusTarget::X11Surface(surf) => state
                .pinnacle
                .windows
                .iter()
                .find(|win| win.x11_surface() == Some(surf))
                .cloned(),
        }
    }

    pub fn layer_for(&self, state: &State) -> Option<LayerSurface> {
        match self {
            PointerFocusTarget::WlSurface(surf) => {
                for output in state.pinnacle.space.outputs() {
                    let map = layer_map_for_output(output);
                    for layer in map.layers() {
                        let mut found = false;
                        with_surfaces_surface_tree(layer.wl_surface(), |surface, _| {
                            if surface == surf {
                                found = true;
                            }
                        });
                        if found {
                            return Some(layer.clone());
                        }
                    }
                }
                None
            }
            PointerFocusTarget::X11Surface(_) => None,
        }
    }

    pub fn popup_for(&self, state: &State) -> Option<PopupKind> {
        match self {
            PointerFocusTarget::WlSurface(surf) => state.pinnacle.popup_manager.find_popup(surf),
            PointerFocusTarget::X11Surface(_) => None,
        }
    }

    pub fn to_keyboard_focus_target(&self, state: &State) -> Option<KeyboardFocusTarget> {
        #[allow(clippy::manual_map)] // screw off clippy
        if let Some(window) = self.window_for(state) {
            Some(KeyboardFocusTarget::Window(window))
        } else if let Some(layer) = self.layer_for(state) {
            Some(KeyboardFocusTarget::LayerSurface(layer))
        } else if let Some(popup) = self.popup_for(state) {
            Some(KeyboardFocusTarget::Popup(popup))
        } else {
            None
        }
    }
}

impl IsAlive for PointerFocusTarget {
    fn alive(&self) -> bool {
        match self {
            PointerFocusTarget::WlSurface(surf) => surf.alive(),
            PointerFocusTarget::X11Surface(surf) => surf.alive(),
        }
    }
}

impl PointerTarget<State> for PointerFocusTarget {
    fn enter(&self, seat: &Seat<State>, data: &mut State, event: &pointer::MotionEvent) {
        match self {
            PointerFocusTarget::WlSurface(surf) => PointerTarget::enter(surf, seat, data, event),
            PointerFocusTarget::X11Surface(surf) => PointerTarget::enter(surf, seat, data, event),
        }

        if let Some(window) = self.window_for(data) {
            let window_id = Some(window.with_state(|state| state.id.0));

            data.pinnacle
                .signal_state
                .window_pointer_enter
                .signal(|buffer| buffer.push_back(WindowPointerEnterResponse { window_id }));
        }
    }

    fn motion(&self, seat: &Seat<State>, data: &mut State, event: &pointer::MotionEvent) {
        match self {
            PointerFocusTarget::WlSurface(surf) => PointerTarget::motion(surf, seat, data, event),
            PointerFocusTarget::X11Surface(surf) => PointerTarget::motion(surf, seat, data, event),
        }
    }

    fn relative_motion(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        event: &pointer::RelativeMotionEvent,
    ) {
        match self {
            PointerFocusTarget::WlSurface(surf) => {
                PointerTarget::relative_motion(surf, seat, data, event);
            }
            PointerFocusTarget::X11Surface(surf) => {
                PointerTarget::relative_motion(surf, seat, data, event);
            }
        }
    }

    fn button(&self, seat: &Seat<State>, data: &mut State, event: &pointer::ButtonEvent) {
        match self {
            PointerFocusTarget::WlSurface(surf) => PointerTarget::button(surf, seat, data, event),
            PointerFocusTarget::X11Surface(surf) => PointerTarget::button(surf, seat, data, event),
        }
    }

    fn axis(&self, seat: &Seat<State>, data: &mut State, frame: pointer::AxisFrame) {
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
        event: &pointer::GestureSwipeBeginEvent,
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
        event: &pointer::GestureSwipeUpdateEvent,
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
        event: &pointer::GestureSwipeEndEvent,
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
        event: &pointer::GesturePinchBeginEvent,
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
        event: &pointer::GesturePinchUpdateEvent,
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
        event: &pointer::GesturePinchEndEvent,
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
        event: &pointer::GestureHoldBeginEvent,
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

    fn gesture_hold_end(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        event: &pointer::GestureHoldEndEvent,
    ) {
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

            data.pinnacle
                .signal_state
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

impl WaylandFocus for PointerFocusTarget {
    fn wl_surface(&self) -> Option<WlSurface> {
        match self {
            PointerFocusTarget::WlSurface(surf) => Some(surf.clone()),
            PointerFocusTarget::X11Surface(surf) => surf.wl_surface(),
        }
    }

    fn same_client_as(&self, object_id: &ObjectId) -> bool {
        match self {
            PointerFocusTarget::WlSurface(surf) => surf.same_client_as(object_id),
            PointerFocusTarget::X11Surface(surf) => surf.same_client_as(object_id),
        }
    }
}

impl From<KeyboardFocusTarget> for PointerFocusTarget {
    fn from(target: KeyboardFocusTarget) -> Self {
        match target {
            KeyboardFocusTarget::Window(window) => match window.underlying_surface() {
                WindowSurface::Wayland(toplevel) => {
                    PointerFocusTarget::WlSurface(toplevel.wl_surface().clone())
                }
                WindowSurface::X11(surface) => PointerFocusTarget::X11Surface(surface.clone()),
            },
            KeyboardFocusTarget::Popup(popup) => {
                PointerFocusTarget::WlSurface(popup.wl_surface().clone())
            }
            KeyboardFocusTarget::LayerSurface(layer) => {
                PointerFocusTarget::WlSurface(layer.wl_surface().clone())
            }
        }
    }
}
