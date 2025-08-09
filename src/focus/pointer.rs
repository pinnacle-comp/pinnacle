use std::borrow::Cow;

use smithay::{
    desktop::{
        LayerSurface, PopupKind, WindowSurface, WindowSurfaceType, layer_map_for_output,
        utils::with_surfaces_surface_tree,
    },
    input::{
        Seat, SeatHandler,
        pointer::{self, PointerTarget},
        touch::{self, TouchTarget},
    },
    output::WeakOutput,
    reexports::wayland_server::{backend::ObjectId, protocol::wl_surface::WlSurface},
    utils::{IsAlive, Logical, Point, Serial},
    wayland::{seat::WaylandFocus, session_lock::LockSurface},
    xwayland::X11Surface,
};

use crate::{
    api::signal::Signal as _,
    state::{Pinnacle, State, WithState},
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
    pub fn window_for(&self, pinnacle: &Pinnacle) -> Option<WindowElement> {
        match self {
            PointerFocusTarget::WlSurface(surf) => pinnacle
                .windows
                .iter()
                .find(|win| {
                    let mut found = false;

                    if let Some(surface) = win.wl_surface() {
                        with_surfaces_surface_tree(&surface, |surface, _| {
                            if surface == surf {
                                found = true;
                            }
                        });
                    }

                    #[cfg(feature = "snowcap")]
                    if !found {
                        win.with_state(|state| {
                            for deco in state.decoration_surfaces.iter() {
                                with_surfaces_surface_tree(deco.wl_surface(), |surface, _| {
                                    if surface == surf {
                                        found = true;
                                    }
                                });

                                if found {
                                    break;
                                }
                            }
                        });
                    }

                    found
                })
                .cloned(),
            PointerFocusTarget::X11Surface(surf) => pinnacle
                .windows
                .iter()
                .find(|win| win.x11_surface() == Some(surf))
                .cloned(),
        }
    }

    pub fn layer_for(&self, pinnacle: &Pinnacle) -> Option<LayerSurface> {
        match self {
            PointerFocusTarget::WlSurface(surf) => pinnacle.space.outputs().find_map(|op| {
                let map = layer_map_for_output(op);
                map.layer_for_surface(surf, WindowSurfaceType::ALL).cloned()
            }),
            PointerFocusTarget::X11Surface(_) => None,
        }
    }

    pub fn popup_for(&self, pinnacle: &Pinnacle) -> Option<PopupKind> {
        match self {
            PointerFocusTarget::WlSurface(surf) => pinnacle.popup_manager.find_popup(surf),
            PointerFocusTarget::X11Surface(_) => None,
        }
    }

    pub fn lock_surface_for(&self, pinnacle: &Pinnacle) -> Option<LockSurface> {
        match self {
            PointerFocusTarget::WlSurface(surf) => pinnacle.space.outputs().find_map(|op| {
                op.with_state(|state| match state.lock_surface.as_ref() {
                    Some(lock_surface) if lock_surface.wl_surface() == surf => {
                        Some(lock_surface.clone())
                    }
                    _ => None,
                })
            }),
            PointerFocusTarget::X11Surface(_) => None,
        }
    }

    pub fn to_keyboard_focus_target(&self, pinnacle: &Pinnacle) -> Option<KeyboardFocusTarget> {
        #[allow(clippy::manual_map)] // screw off clippy
        if let Some(window) = self.window_for(pinnacle) {
            Some(KeyboardFocusTarget::Window(window))
        } else if let Some(layer) = self.layer_for(pinnacle) {
            Some(KeyboardFocusTarget::LayerSurface(layer))
        } else if let Some(popup) = self.popup_for(pinnacle) {
            Some(KeyboardFocusTarget::Popup(popup))
        } else if let Some(lock_surface) = self.lock_surface_for(pinnacle) {
            Some(KeyboardFocusTarget::LockSurface(lock_surface))
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
    fn wl_surface(&self) -> Option<Cow<'_, WlSurface>> {
        match self {
            PointerFocusTarget::WlSurface(surf) => Some(Cow::Borrowed(surf)),
            PointerFocusTarget::X11Surface(surf) => surf.wl_surface().map(Cow::Owned),
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
            KeyboardFocusTarget::LockSurface(lock) => {
                PointerFocusTarget::WlSurface(lock.wl_surface().clone())
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
/// Content under the pointer location.
pub struct PointerContents {
    /// The current pointer focus under the pointer.
    pub focus_under: Option<(<State as SeatHandler>::PointerFocus, Point<f64, Logical>)>,
    /// The output under the pointer.
    pub output_under: Option<WeakOutput>,
}

impl Pinnacle {
    pub fn set_pointer_contents(&mut self, new_contents: PointerContents) {
        let old_contents = std::mem::take(&mut self.pointer_contents);

        let old_focused_win = old_contents
            .focus_under
            .and_then(|(foc, _)| foc.window_for(self));
        let new_focused_win = new_contents
            .focus_under
            .as_ref()
            .and_then(|(foc, _)| foc.window_for(self));

        if old_focused_win != new_focused_win {
            if let Some(old) = old_focused_win {
                self.signal_state.window_pointer_leave.signal(&old);
            }
            if let Some(new) = new_focused_win {
                self.signal_state.window_pointer_enter.signal(&new);
            }
        }

        let old_op = old_contents.output_under.and_then(|op| op.upgrade());
        let new_op = new_contents
            .output_under
            .as_ref()
            .and_then(|op| op.upgrade());

        if old_op != new_op {
            if let Some(old) = old_op {
                self.signal_state.output_pointer_leave.signal(&old);
            }
            if let Some(new) = new_op {
                self.signal_state.output_pointer_enter.signal(&new);
            }
        }

        self.pointer_contents = new_contents;
    }
}
