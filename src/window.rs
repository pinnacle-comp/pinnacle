// SPDX-License-Identifier: GPL-3.0-or-later

pub mod rules;

use std::{cell::RefCell, time::Duration};

use pinnacle_api_defs::pinnacle::signal::v0alpha1::{
    WindowPointerEnterResponse, WindowPointerLeaveResponse,
};
use smithay::{
    backend::input::KeyState,
    desktop::{
        utils::{
            send_dmabuf_feedback_surface_tree, send_frames_surface_tree,
            take_presentation_feedback_surface_tree, with_surfaces_surface_tree,
            OutputPresentationFeedback,
        },
        Window,
    },
    input::{
        keyboard::{KeyboardTarget, KeysymHandle, ModifiersState},
        pointer::{AxisFrame, MotionEvent, PointerTarget},
        Seat,
    },
    output::Output,
    reexports::{
        wayland_protocols::wp::presentation_time::server::wp_presentation_feedback,
        wayland_server::protocol::wl_surface::WlSurface,
    },
    space_elements,
    utils::{user_data::UserDataMap, Logical, Rectangle, Serial},
    wayland::{
        compositor::{self, SurfaceData},
        dmabuf::DmabufFeedback,
        seat::WaylandFocus,
        shell::xdg::XdgToplevelSurfaceData,
    },
    xwayland::X11Surface,
};

use crate::state::{State, WithState};

use self::window_state::WindowElementState;

pub mod window_state;

space_elements! {
    /// The different types of windows.
    #[derive(Debug, Clone, PartialEq)]
    pub WindowElement;
    /// This is a native Wayland window.
    Wayland = Window,
    /// This is an Xwayland window.
    X11 = X11Surface,
    /// This is an Xwayland override redirect window, which should not be messed with.
    X11OverrideRedirect = X11Surface,
}

impl WindowElement {
    pub fn with_surfaces<F>(&self, processor: F)
    where
        F: FnMut(&WlSurface, &SurfaceData) + Copy,
    {
        match self {
            WindowElement::Wayland(window) => window.with_surfaces(processor),
            WindowElement::X11(surface) | WindowElement::X11OverrideRedirect(surface) => {
                if let Some(surface) = surface.wl_surface() {
                    with_surfaces_surface_tree(&surface, processor);
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn send_frame<T, F>(
        &self,
        output: &Output,
        time: T,
        throttle: Option<Duration>,
        primary_scan_out_output: F,
    ) where
        T: Into<Duration>,
        F: FnMut(&WlSurface, &SurfaceData) -> Option<Output> + Copy,
    {
        match self {
            WindowElement::Wayland(window) => {
                window.send_frame(output, time, throttle, primary_scan_out_output)
            }
            WindowElement::X11(surface) | WindowElement::X11OverrideRedirect(surface) => {
                if let Some(surface) = surface.wl_surface() {
                    send_frames_surface_tree(
                        &surface,
                        output,
                        time,
                        throttle,
                        primary_scan_out_output,
                    );
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn send_dmabuf_feedback<'a, P, F>(
        &self,
        output: &Output,
        primary_scan_out_output: P,
        select_dmabuf_feedback: F,
    ) where
        P: FnMut(&WlSurface, &SurfaceData) -> Option<Output> + Copy,
        F: Fn(&WlSurface, &SurfaceData) -> &'a DmabufFeedback + Copy,
    {
        match self {
            WindowElement::Wayland(window) => {
                window.send_dmabuf_feedback(
                    output,
                    primary_scan_out_output,
                    select_dmabuf_feedback,
                );
            }
            WindowElement::X11(surface) | WindowElement::X11OverrideRedirect(surface) => {
                if let Some(surface) = surface.wl_surface() {
                    send_dmabuf_feedback_surface_tree(
                        &surface,
                        output,
                        primary_scan_out_output,
                        select_dmabuf_feedback,
                    );
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn take_presentation_feedback<F1, F2>(
        &self,
        output_feedback: &mut OutputPresentationFeedback,
        primary_scan_out_output: F1,
        presentation_feedback_flags: F2,
    ) where
        F1: FnMut(&WlSurface, &SurfaceData) -> Option<Output> + Copy,
        F2: FnMut(&WlSurface, &SurfaceData) -> wp_presentation_feedback::Kind + Copy,
    {
        match self {
            WindowElement::Wayland(window) => {
                window.take_presentation_feedback(
                    output_feedback,
                    primary_scan_out_output,
                    presentation_feedback_flags,
                );
            }
            WindowElement::X11(surface) | WindowElement::X11OverrideRedirect(surface) => {
                if let Some(surface) = surface.wl_surface() {
                    take_presentation_feedback_surface_tree(
                        &surface,
                        output_feedback,
                        primary_scan_out_output,
                        presentation_feedback_flags,
                    );
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn wl_surface(&self) -> Option<WlSurface> {
        match self {
            WindowElement::Wayland(window) => window.wl_surface(),
            WindowElement::X11(surface) | WindowElement::X11OverrideRedirect(surface) => {
                surface.wl_surface()
            }
            _ => unreachable!(),
        }
    }

    pub fn user_data(&self) -> &UserDataMap {
        match self {
            WindowElement::Wayland(window) => window.user_data(),
            WindowElement::X11(surface) | WindowElement::X11OverrideRedirect(surface) => {
                surface.user_data()
            }
            _ => unreachable!(),
        }
    }

    /// Send a geometry change without mapping windows or sending
    /// configures to Wayland windows.
    ///
    /// Xwayland windows will still receive a configure.
    ///
    /// RefCell Safety: This method uses a [`RefCell`] on this window.
    // TODO: ^ does that make things flicker?
    pub fn change_geometry(&self, new_geo: Rectangle<i32, Logical>) {
        match self {
            WindowElement::Wayland(window) => {
                window
                    .toplevel()
                    .expect("in wayland enum")
                    .with_pending_state(|state| {
                        state.size = Some(new_geo.size);
                    });
            }
            WindowElement::X11(surface) | WindowElement::X11OverrideRedirect(surface) => {
                // TODO: maybe move this check elsewhere idk
                if !surface.is_override_redirect() {
                    surface
                        .configure(new_geo)
                        .expect("failed to configure x11 win");
                }
            }
            _ => unreachable!(),
        }
        self.with_state(|state| {
            state.target_loc = Some(new_geo.loc);
        });
        // self.with_state(|state| {
        //     state.loc_request_state = LocationRequestState::Sent(new_geo.loc);
        // });
    }

    pub fn class(&self) -> Option<String> {
        match self {
            WindowElement::Wayland(window) => compositor::with_states(
                window.toplevel().expect("in wayland enum").wl_surface(),
                |states| {
                    states
                        .data_map
                        .get::<XdgToplevelSurfaceData>()
                        .expect("XdgToplevelSurfaceData wasn't in surface's data map")
                        .lock()
                        .expect("Failed to lock Mutex<XdgToplevelSurfaceData>")
                        .app_id
                        .clone()
                },
            ),
            WindowElement::X11(surface) | WindowElement::X11OverrideRedirect(surface) => {
                Some(surface.class())
            }
            _ => unreachable!(),
        }
    }

    pub fn title(&self) -> Option<String> {
        match self {
            WindowElement::Wayland(window) => compositor::with_states(
                window.toplevel().expect("in wayland enum").wl_surface(),
                |states| {
                    states
                        .data_map
                        .get::<XdgToplevelSurfaceData>()
                        .expect("XdgToplevelSurfaceData wasn't in surface's data map")
                        .lock()
                        .expect("Failed to lock Mutex<XdgToplevelSurfaceData>")
                        .title
                        .clone()
                },
            ),
            WindowElement::X11(surface) | WindowElement::X11OverrideRedirect(surface) => {
                Some(surface.title())
            }
            _ => unreachable!(),
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

    /// Returns `true` if the window element is [`Wayland`].
    ///
    /// [`Wayland`]: WindowElement::Wayland
    #[must_use]
    pub fn is_wayland(&self) -> bool {
        matches!(self, Self::Wayland(..))
    }

    /// Returns `true` if the window element is [`X11`].
    ///
    /// [`X11`]: WindowElement::X11
    #[must_use]
    pub fn is_x11(&self) -> bool {
        matches!(self, Self::X11(..))
    }

    /// Returns `true` if the window element is [`X11OverrideRedirect`].
    ///
    /// [`X11OverrideRedirect`]: WindowElement::X11OverrideRedirect
    #[must_use]
    pub fn is_x11_override_redirect(&self) -> bool {
        matches!(self, Self::X11OverrideRedirect(..))
    }
}

impl PointerTarget<State> for WindowElement {
    fn frame(&self, seat: &Seat<State>, state: &mut State) {
        match self {
            WindowElement::Wayland(window) => window
                .wl_surface()
                .expect("in wayland enum")
                .frame(seat, state),
            WindowElement::X11(surface) | WindowElement::X11OverrideRedirect(surface) => {
                surface.frame(seat, state)
            }
            _ => unreachable!(),
        }
    }

    fn enter(&self, seat: &Seat<State>, state: &mut State, event: &MotionEvent) {
        // TODO: ssd
        match self {
            WindowElement::Wayland(window) => PointerTarget::enter(
                &window.wl_surface().expect("in wayland enum"),
                seat,
                state,
                event,
            ),
            WindowElement::X11(surface) | WindowElement::X11OverrideRedirect(surface) => {
                PointerTarget::enter(surface, seat, state, event)
            }
            _ => unreachable!(),
        }

        let window_id = Some(self.with_state(|state| state.id.0));

        state
            .signal_state
            .window_pointer_enter
            .signal(|buffer| buffer.push_back(WindowPointerEnterResponse { window_id }));
    }

    fn motion(&self, seat: &Seat<State>, state: &mut State, event: &MotionEvent) {
        // TODO: ssd
        match self {
            WindowElement::Wayland(window) => PointerTarget::motion(
                &window.wl_surface().expect("in wayland enum"),
                seat,
                state,
                event,
            ),
            WindowElement::X11(surface) | WindowElement::X11OverrideRedirect(surface) => {
                PointerTarget::motion(surface, seat, state, event)
            }
            _ => unreachable!(),
        }
    }

    fn relative_motion(
        &self,
        seat: &Seat<State>,
        state: &mut State,
        event: &smithay::input::pointer::RelativeMotionEvent,
    ) {
        // TODO: ssd
        match self {
            WindowElement::Wayland(window) => {
                PointerTarget::relative_motion(
                    &window.wl_surface().expect("in wayland enum"),
                    seat,
                    state,
                    event,
                );
            }
            WindowElement::X11(surface) | WindowElement::X11OverrideRedirect(surface) => {
                PointerTarget::relative_motion(surface, seat, state, event);
            }
            _ => unreachable!(),
        }
    }

    fn button(
        &self,
        seat: &Seat<State>,
        state: &mut State,
        event: &smithay::input::pointer::ButtonEvent,
    ) {
        // TODO: ssd
        match self {
            WindowElement::Wayland(window) => PointerTarget::button(
                &window.wl_surface().expect("in wayland enum"),
                seat,
                state,
                event,
            ),
            WindowElement::X11(surface) | WindowElement::X11OverrideRedirect(surface) => {
                PointerTarget::button(surface, seat, state, event)
            }
            _ => unreachable!(),
        }
    }

    fn axis(&self, seat: &Seat<State>, state: &mut State, frame: AxisFrame) {
        // TODO: ssd
        match self {
            WindowElement::Wayland(window) => PointerTarget::axis(
                &window.wl_surface().expect("in wayland enum"),
                seat,
                state,
                frame,
            ),
            WindowElement::X11(surface) | WindowElement::X11OverrideRedirect(surface) => {
                PointerTarget::axis(surface, seat, state, frame)
            }
            _ => unreachable!(),
        }
    }

    fn leave(&self, seat: &Seat<State>, state: &mut State, serial: Serial, time: u32) {
        // TODO: ssd
        match self {
            WindowElement::Wayland(window) => {
                PointerTarget::leave(
                    &window.wl_surface().expect("in wayland enum"),
                    seat,
                    state,
                    serial,
                    time,
                );
            }
            WindowElement::X11(surface) | WindowElement::X11OverrideRedirect(surface) => {
                PointerTarget::leave(surface, seat, state, serial, time)
            }
            _ => unreachable!(),
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
        match self {
            WindowElement::Wayland(window) => {
                KeyboardTarget::enter(
                    &window.wl_surface().expect("in wayland enum"),
                    seat,
                    state,
                    keys,
                    serial,
                );
            }
            WindowElement::X11(surface) | WindowElement::X11OverrideRedirect(surface) => {
                KeyboardTarget::enter(surface, seat, state, keys, serial)
            }
            _ => unreachable!(),
        }
    }

    fn leave(&self, seat: &Seat<State>, state: &mut State, serial: Serial) {
        match self {
            WindowElement::Wayland(window) => KeyboardTarget::leave(
                &window.wl_surface().expect("in wayland enum"),
                seat,
                state,
                serial,
            ),
            WindowElement::X11(surface) | WindowElement::X11OverrideRedirect(surface) => {
                KeyboardTarget::leave(surface, seat, state, serial)
            }
            _ => unreachable!(),
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
            WindowElement::Wayland(window) => {
                KeyboardTarget::key(
                    &window.wl_surface().expect("in wayland enum"),
                    seat,
                    state,
                    key,
                    key_state,
                    serial,
                    time,
                );
            }
            WindowElement::X11(surface) | WindowElement::X11OverrideRedirect(surface) => {
                KeyboardTarget::key(surface, seat, state, key, key_state, serial, time);
            }
            _ => unreachable!(),
        }
    }

    fn modifiers(
        &self,
        seat: &Seat<State>,
        state: &mut State,
        modifiers: ModifiersState,
        serial: Serial,
    ) {
        match self {
            WindowElement::Wayland(window) => {
                KeyboardTarget::modifiers(
                    &window.wl_surface().expect("in wayland enum"),
                    seat,
                    state,
                    modifiers,
                    serial,
                );
            }
            WindowElement::X11(surface) | WindowElement::X11OverrideRedirect(surface) => {
                KeyboardTarget::modifiers(surface, seat, state, modifiers, serial);
            }
            _ => unreachable!(),
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
            .cloned()
            .or_else(|| {
                self.windows
                    .iter()
                    .find(|&win| win.wl_surface().is_some_and(|surf| &surf == surface))
                    .cloned()
            })
    }
}
