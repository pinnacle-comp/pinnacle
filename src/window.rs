// SPDX-License-Identifier: GPL-3.0-or-later

pub mod rules;

use std::{cell::RefCell, ops::Deref};

use pinnacle_api_defs::pinnacle::signal::v0alpha1::{
    WindowPointerEnterResponse, WindowPointerLeaveResponse,
};
use smithay::{
    backend::input::KeyState,
    desktop::{space::SpaceElement, Window, WindowSurface},
    input::{
        keyboard::{KeyboardTarget, KeysymHandle, ModifiersState},
        pointer::{AxisFrame, MotionEvent, PointerTarget},
        Seat,
    },
    output::Output,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{IsAlive, Logical, Point, Rectangle, Serial},
    wayland::{compositor, seat::WaylandFocus, shell::xdg::XdgToplevelSurfaceData},
};

use crate::state::{State, WithState};

use self::window_state::WindowElementState;

pub mod window_state;

#[derive(Debug, Clone, PartialEq)]
pub struct WindowElement(Window);

impl Deref for WindowElement {
    type Target = Window;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl WindowElement {
    pub fn new(window: Window) -> Self {
        Self(window)
    }

    /// Send a geometry change without mapping windows or sending
    /// configures to Wayland windows.
    ///
    /// Xwayland windows will still receive a configure.
    ///
    /// RefCell Safety: This method uses a [`RefCell`] on this window.
    // TODO: ^ does that make things flicker?
    pub fn change_geometry(&self, new_geo: Rectangle<i32, Logical>) {
        match self.0.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                toplevel.with_pending_state(|state| {
                    state.size = Some(new_geo.size);
                });
            }
            WindowSurface::X11(surface) => {
                // TODO: maybe move this check elsewhere idk
                if !surface.is_override_redirect() {
                    surface
                        .configure(new_geo)
                        .expect("failed to configure x11 win");
                }
            }
        }
        self.with_state(|state| {
            state.target_loc = Some(new_geo.loc);
        });
    }

    pub fn class(&self) -> Option<String> {
        match self.0.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                compositor::with_states(toplevel.wl_surface(), |states| {
                    states
                        .data_map
                        .get::<XdgToplevelSurfaceData>()
                        .expect("XdgToplevelSurfaceData wasn't in surface's data map")
                        .lock()
                        .expect("Failed to lock Mutex<XdgToplevelSurfaceData>")
                        .app_id
                        .clone()
                })
            }
            WindowSurface::X11(surface) => Some(surface.class()),
        }
    }

    pub fn title(&self) -> Option<String> {
        match self.0.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                compositor::with_states(toplevel.wl_surface(), |states| {
                    states
                        .data_map
                        .get::<XdgToplevelSurfaceData>()
                        .expect("XdgToplevelSurfaceData wasn't in surface's data map")
                        .lock()
                        .expect("Failed to lock Mutex<XdgToplevelSurfaceData>")
                        .title
                        .clone()
                })
            }
            WindowSurface::X11(surface) => Some(surface.title()),
        }
    }

    /// Get the output this window is on.
    ///
    /// This method gets the first tag the window has and returns its output.
    ///
    /// RefCell Safety: This method uses a [`RefCell`] on this window and every mapped output.
    pub fn output(&self, state: &State) -> Option<Output> {
        self.with_state(|st| st.tags.first().and_then(|tag| tag.output(state)))
    }

    /// Returns whether or not this window has an active tag.
    ///
    /// RefCell Safety: This uses RefCells on both `self` and everything in `outputs`.
    pub fn is_on_active_tag<'a>(&self, outputs: impl IntoIterator<Item = &'a Output>) -> bool {
        let tags = outputs
            .into_iter()
            .flat_map(|op| op.with_state(|state| state.focused_tags().cloned().collect::<Vec<_>>()))
            .collect::<Vec<_>>();

        self.with_state(|state| {
            state
                .tags
                .iter()
                .any(|tag| tags.iter().any(|tag2| tag == tag2))
        })
    }

    /// Place this window on the given output, giving it the output's focused tags.
    ///
    /// RefCell Safety: Uses refcells on both the window and the output.
    pub fn place_on_output(&self, output: &Output) {
        self.with_state(|state| {
            state.tags = output.with_state(|state| {
                let output_tags = state.focused_tags().cloned().collect::<Vec<_>>();
                if !output_tags.is_empty() {
                    output_tags
                } else if let Some(first_tag) = state.tags.first() {
                    vec![first_tag.clone()]
                } else {
                    vec![]
                }
            });

            tracing::debug!(
                "Placed window on {} with tags {:?}",
                output.name(),
                state.tags
            );
        });
    }

    pub fn is_x11_override_redirect(&self) -> bool {
        matches!(self.x11_surface(), Some(surface) if surface.is_override_redirect())
    }
}

impl SpaceElement for WindowElement {
    fn bbox(&self) -> Rectangle<i32, Logical> {
        self.0.bbox()
    }

    fn is_in_input_region(&self, point: &Point<f64, Logical>) -> bool {
        self.0.is_in_input_region(point)
    }

    fn set_activate(&self, activated: bool) {
        self.0.set_activate(activated)
    }

    fn output_enter(&self, output: &Output, overlap: Rectangle<i32, Logical>) {
        self.0.output_enter(output, overlap)
    }

    fn output_leave(&self, output: &Output) {
        self.0.output_leave(output)
    }

    fn geometry(&self) -> Rectangle<i32, Logical> {
        self.0.geometry()
    }

    fn z_index(&self) -> u8 {
        self.0.z_index()
    }

    fn refresh(&self) {
        self.0.refresh();
    }
}

impl IsAlive for WindowElement {
    fn alive(&self) -> bool {
        self.0.alive()
    }
}

impl PointerTarget<State> for WindowElement {
    fn frame(&self, seat: &Seat<State>, state: &mut State) {
        match self.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                PointerTarget::frame(toplevel.wl_surface(), seat, state);
            }
            WindowSurface::X11(surface) => PointerTarget::frame(surface, seat, state),
        }
    }

    fn enter(&self, seat: &Seat<State>, state: &mut State, event: &MotionEvent) {
        match self.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                PointerTarget::enter(toplevel.wl_surface(), seat, state, event);
            }
            WindowSurface::X11(surface) => PointerTarget::enter(surface, seat, state, event),
        }

        let window_id = Some(self.with_state(|state| state.id.0));

        state
            .signal_state
            .window_pointer_enter
            .signal(|buffer| buffer.push_back(WindowPointerEnterResponse { window_id }));
    }

    fn motion(&self, seat: &Seat<State>, state: &mut State, event: &MotionEvent) {
        match self.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                PointerTarget::motion(toplevel.wl_surface(), seat, state, event);
            }
            WindowSurface::X11(surface) => PointerTarget::motion(surface, seat, state, event),
        }
    }

    fn relative_motion(
        &self,
        seat: &Seat<State>,
        state: &mut State,
        event: &smithay::input::pointer::RelativeMotionEvent,
    ) {
        match self.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                PointerTarget::relative_motion(toplevel.wl_surface(), seat, state, event);
            }
            WindowSurface::X11(surface) => {
                PointerTarget::relative_motion(surface, seat, state, event);
            }
        }
    }

    fn button(
        &self,
        seat: &Seat<State>,
        state: &mut State,
        event: &smithay::input::pointer::ButtonEvent,
    ) {
        match self.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                PointerTarget::button(toplevel.wl_surface(), seat, state, event);
            }
            WindowSurface::X11(surface) => PointerTarget::button(surface, seat, state, event),
        }
    }

    fn axis(&self, seat: &Seat<State>, state: &mut State, frame: AxisFrame) {
        match self.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                PointerTarget::axis(toplevel.wl_surface(), seat, state, frame);
            }
            WindowSurface::X11(surface) => PointerTarget::axis(surface, seat, state, frame),
        }
    }

    fn leave(&self, seat: &Seat<State>, state: &mut State, serial: Serial, time: u32) {
        match self.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                PointerTarget::leave(toplevel.wl_surface(), seat, state, serial, time);
            }
            WindowSurface::X11(surface) => PointerTarget::leave(surface, seat, state, serial, time),
        }

        let window_id = Some(self.with_state(|state| state.id.0));

        state
            .signal_state
            .window_pointer_leave
            .signal(|buffer| buffer.push_back(WindowPointerLeaveResponse { window_id }));
    }

    fn gesture_swipe_begin(
        &self,
        _seat: &Seat<State>,
        _state: &mut State,
        _event: &smithay::input::pointer::GestureSwipeBeginEvent,
    ) {
        todo!()
    }

    fn gesture_swipe_update(
        &self,
        _seat: &Seat<State>,
        _state: &mut State,
        _event: &smithay::input::pointer::GestureSwipeUpdateEvent,
    ) {
        todo!()
    }

    fn gesture_swipe_end(
        &self,
        _seat: &Seat<State>,
        _state: &mut State,
        _event: &smithay::input::pointer::GestureSwipeEndEvent,
    ) {
        todo!()
    }

    fn gesture_pinch_begin(
        &self,
        _seat: &Seat<State>,
        _state: &mut State,
        _event: &smithay::input::pointer::GesturePinchBeginEvent,
    ) {
        todo!()
    }

    fn gesture_pinch_update(
        &self,
        _seat: &Seat<State>,
        _state: &mut State,
        _event: &smithay::input::pointer::GesturePinchUpdateEvent,
    ) {
        todo!()
    }

    fn gesture_pinch_end(
        &self,
        _seat: &Seat<State>,
        _state: &mut State,
        _event: &smithay::input::pointer::GesturePinchEndEvent,
    ) {
        todo!()
    }

    fn gesture_hold_begin(
        &self,
        _seat: &Seat<State>,
        _state: &mut State,
        _event: &smithay::input::pointer::GestureHoldBeginEvent,
    ) {
        todo!()
    }

    fn gesture_hold_end(
        &self,
        _seat: &Seat<State>,
        _state: &mut State,
        _event: &smithay::input::pointer::GestureHoldEndEvent,
    ) {
        todo!()
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

impl WithState for WindowElement {
    type State = WindowElementState;

    fn with_state<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&mut Self::State) -> T,
    {
        let state = self
            .user_data()
            .get_or_insert(|| RefCell::new(WindowElementState::new()));

        func(&mut state.borrow_mut())
    }
}

impl State {
    /// Returns the [Window] associated with a given [WlSurface].
    pub fn window_for_surface(&self, surface: &WlSurface) -> Option<WindowElement> {
        self.space
            .elements()
            .find(|window| window.wl_surface().map(|s| s == *surface).unwrap_or(false))
            .or_else(|| {
                self.windows
                    .iter()
                    .find(|&win| win.wl_surface().is_some_and(|surf| &surf == surface))
            })
            .cloned()
    }

    pub fn new_window_for_surface(&self, surface: &WlSurface) -> Option<WindowElement> {
        self.new_windows
            .iter()
            .find(|&win| win.wl_surface().is_some_and(|surf| &surf == surface))
            .cloned()
    }
}
