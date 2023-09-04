// SPDX-License-Identifier: GPL-3.0-or-later

use std::{cell::RefCell, sync::atomic::AtomicU32, time::Duration};

use smithay::{
    backend::input::KeyState,
    desktop::{
        space::SpaceElement,
        utils::{
            send_dmabuf_feedback_surface_tree, send_frames_surface_tree,
            take_presentation_feedback_surface_tree, under_from_surface_tree,
            with_surfaces_surface_tree, OutputPresentationFeedback,
        },
        Space, Window, WindowSurfaceType,
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
    utils::{user_data::UserDataMap, IsAlive, Logical, Point, Rectangle, Serial, Size},
    wayland::{
        compositor::{Blocker, BlockerState, SurfaceData},
        dmabuf::DmabufFeedback,
        seat::WaylandFocus,
    },
    xwayland::X11Surface,
};

use crate::state::{State, WithState};

use self::window_state::{LocationRequestState, WindowElementState};

pub mod window_state;

#[derive(Debug, Clone, PartialEq)]
pub enum WindowElement {
    Wayland(Window),
    X11(X11Surface),
}

impl WindowElement {
    pub fn surface_under(
        &self,
        location: Point<f64, Logical>,
        window_type: WindowSurfaceType,
    ) -> Option<(WlSurface, Point<i32, Logical>)> {
        match self {
            WindowElement::Wayland(window) => window.surface_under(location, window_type),
            WindowElement::X11(surface) => surface.wl_surface().and_then(|wl_surf| {
                under_from_surface_tree(&wl_surf, location, (0, 0), window_type)
            }),
        }
    }

    pub fn with_surfaces<F>(&self, processor: F)
    where
        F: FnMut(&WlSurface, &SurfaceData) + Copy,
    {
        match self {
            WindowElement::Wayland(window) => window.with_surfaces(processor),
            WindowElement::X11(surface) => {
                if let Some(surface) = surface.wl_surface() {
                    with_surfaces_surface_tree(&surface, processor);
                }
            }
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
            WindowElement::X11(surface) => {
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
            WindowElement::X11(surface) => {
                if let Some(surface) = surface.wl_surface() {
                    send_dmabuf_feedback_surface_tree(
                        &surface,
                        output,
                        primary_scan_out_output,
                        select_dmabuf_feedback,
                    );
                }
            }
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
            WindowElement::X11(surface) => {
                if let Some(surface) = surface.wl_surface() {
                    take_presentation_feedback_surface_tree(
                        &surface,
                        output_feedback,
                        primary_scan_out_output,
                        presentation_feedback_flags,
                    );
                }
            }
        }
    }

    pub fn wl_surface(&self) -> Option<WlSurface> {
        match self {
            WindowElement::Wayland(window) => window.wl_surface(),
            WindowElement::X11(surface) => surface.wl_surface(),
        }
    }

    pub fn user_data(&self) -> &UserDataMap {
        match self {
            WindowElement::Wayland(window) => window.user_data(),
            WindowElement::X11(surface) => surface.user_data(),
        }
    }

    /// Send a geometry change without mapping windows or sending
    /// configures to Wayland windows.
    ///
    /// Xwayland windows will still receive a configure.
    ///
    /// This method uses a [`RefCell`].
    // TODO: ^ does that make things flicker?
    pub fn change_geometry(&self, new_geo: Rectangle<i32, Logical>) {
        match self {
            WindowElement::Wayland(window) => {
                window.toplevel().with_pending_state(|state| {
                    state.size = Some(new_geo.size);
                });
            }
            WindowElement::X11(surface) => {
                surface
                    .configure(new_geo)
                    .expect("failed to configure x11 win");
            }
        }
        self.with_state(|state| {
            state.loc_request_state = LocationRequestState::Sent(new_geo.loc);
        });
    }

    /// Request a size and loc change.
    pub fn request_size_change(
        &self,
        space: &mut Space<WindowElement>,
        new_loc: Point<i32, Logical>,
        new_size: Size<i32, Logical>,
    ) {
        match self {
            WindowElement::Wayland(window) => {
                window.toplevel().with_pending_state(|state| {
                    state.size = Some(new_size);
                });
                self.with_state(|state| {
                    state.loc_request_state =
                        LocationRequestState::Requested(window.toplevel().send_configure(), new_loc)
                });
            }
            WindowElement::X11(surface) => {
                tracing::debug!("sending size change to x11 win");
                surface
                    .configure(Rectangle::from_loc_and_size(new_loc, new_size))
                    .expect("failed to configure x11 win");
                // self.with_state(|state| {
                //     state.resize_state = WindowResizeState::Acknowledged(new_loc);
                // });
                if !surface.is_override_redirect() {
                    surface
                        .set_mapped(true)
                        .expect("failed to set x11 win to mapped");
                }
                space.map_element(self.clone(), new_loc, false);
                // if let Some(focused_output) = state.focus_state.focused_output.clone() {
                //     self.send_frame(
                //         &focused_output,
                //         state.clock.now(),
                //         Some(Duration::ZERO),
                //         surface_primary_scanout_output,
                //     );
                // }
            }
        }
    }

    /// Get the output this window is on.
    ///
    /// This method gets the first tag the window has and returns its output.
    pub fn output(&self, state: &State) -> Option<Output> {
        self.with_state(|st| st.tags.first().and_then(|tag| tag.output(state)))
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
}

impl IsAlive for WindowElement {
    fn alive(&self) -> bool {
        match self {
            WindowElement::Wayland(window) => window.alive(),
            WindowElement::X11(surface) => surface.alive(),
        }
    }
}

impl PointerTarget<State> for WindowElement {
    fn enter(&self, seat: &Seat<State>, data: &mut State, event: &MotionEvent) {
        // TODO: ssd
        match self {
            WindowElement::Wayland(window) => PointerTarget::enter(window, seat, data, event),
            WindowElement::X11(surface) => PointerTarget::enter(surface, seat, data, event),
        }
    }

    fn motion(&self, seat: &Seat<State>, data: &mut State, event: &MotionEvent) {
        // TODO: ssd
        match self {
            WindowElement::Wayland(window) => PointerTarget::motion(window, seat, data, event),
            WindowElement::X11(surface) => PointerTarget::motion(surface, seat, data, event),
        }
    }

    fn relative_motion(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        event: &smithay::input::pointer::RelativeMotionEvent,
    ) {
        // TODO: ssd
        match self {
            WindowElement::Wayland(window) => {
                PointerTarget::relative_motion(window, seat, data, event);
            }
            WindowElement::X11(surface) => {
                PointerTarget::relative_motion(surface, seat, data, event);
            }
        }
    }

    fn button(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        event: &smithay::input::pointer::ButtonEvent,
    ) {
        // TODO: ssd
        match self {
            WindowElement::Wayland(window) => PointerTarget::button(window, seat, data, event),
            WindowElement::X11(surface) => PointerTarget::button(surface, seat, data, event),
        }
    }

    fn axis(&self, seat: &Seat<State>, data: &mut State, frame: AxisFrame) {
        // TODO: ssd
        match self {
            WindowElement::Wayland(window) => PointerTarget::axis(window, seat, data, frame),
            WindowElement::X11(surface) => PointerTarget::axis(surface, seat, data, frame),
        }
    }

    fn leave(&self, seat: &Seat<State>, data: &mut State, serial: Serial, time: u32) {
        // TODO: ssd
        match self {
            WindowElement::Wayland(window) => {
                PointerTarget::leave(window, seat, data, serial, time);
            }
            WindowElement::X11(surface) => PointerTarget::leave(surface, seat, data, serial, time),
        }
    }
}

impl KeyboardTarget<State> for WindowElement {
    fn enter(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        keys: Vec<KeysymHandle<'_>>,
        serial: Serial,
    ) {
        match self {
            WindowElement::Wayland(window) => {
                KeyboardTarget::enter(window, seat, data, keys, serial);
            }
            WindowElement::X11(surface) => KeyboardTarget::enter(surface, seat, data, keys, serial),
        }
    }

    fn leave(&self, seat: &Seat<State>, data: &mut State, serial: Serial) {
        match self {
            WindowElement::Wayland(window) => KeyboardTarget::leave(window, seat, data, serial),
            WindowElement::X11(surface) => KeyboardTarget::leave(surface, seat, data, serial),
        }
    }

    fn key(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        key: KeysymHandle<'_>,
        state: KeyState,
        serial: Serial,
        time: u32,
    ) {
        match self {
            WindowElement::Wayland(window) => {
                KeyboardTarget::key(window, seat, data, key, state, serial, time);
            }
            WindowElement::X11(surface) => {
                KeyboardTarget::key(surface, seat, data, key, state, serial, time);
            }
        }
    }

    fn modifiers(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        modifiers: ModifiersState,
        serial: Serial,
    ) {
        match self {
            WindowElement::Wayland(window) => {
                KeyboardTarget::modifiers(window, seat, data, modifiers, serial);
            }
            WindowElement::X11(surface) => {
                KeyboardTarget::modifiers(surface, seat, data, modifiers, serial);
            }
        }
    }
}

impl SpaceElement for WindowElement {
    fn geometry(&self) -> Rectangle<i32, Logical> {
        // TODO: ssd
        match self {
            WindowElement::Wayland(window) => SpaceElement::geometry(window),
            WindowElement::X11(surface) => SpaceElement::geometry(surface),
        }
    }

    fn bbox(&self) -> Rectangle<i32, Logical> {
        // TODO: ssd
        match self {
            WindowElement::Wayland(window) => SpaceElement::bbox(window),
            WindowElement::X11(surface) => SpaceElement::bbox(surface),
        }
    }

    fn is_in_input_region(&self, point: &Point<f64, Logical>) -> bool {
        // TODO: ssd
        match self {
            WindowElement::Wayland(window) => SpaceElement::is_in_input_region(window, point),
            WindowElement::X11(surface) => SpaceElement::is_in_input_region(surface, point),
        }
    }

    fn z_index(&self) -> u8 {
        match self {
            WindowElement::Wayland(window) => SpaceElement::z_index(window),
            WindowElement::X11(surface) => SpaceElement::z_index(surface),
        }
    }

    fn set_activate(&self, activated: bool) {
        match self {
            WindowElement::Wayland(window) => SpaceElement::set_activate(window, activated),
            WindowElement::X11(surface) => SpaceElement::set_activate(surface, activated),
        }
    }

    fn output_enter(&self, output: &Output, overlap: Rectangle<i32, Logical>) {
        match self {
            WindowElement::Wayland(window) => SpaceElement::output_enter(window, output, overlap),
            WindowElement::X11(surface) => SpaceElement::output_enter(surface, output, overlap),
        }
    }

    fn output_leave(&self, output: &Output) {
        match self {
            WindowElement::Wayland(window) => SpaceElement::output_leave(window, output),
            WindowElement::X11(surface) => SpaceElement::output_leave(surface, output),
        }
    }

    fn refresh(&self) {
        match self {
            WindowElement::Wayland(window) => SpaceElement::refresh(window),
            WindowElement::X11(surface) => SpaceElement::refresh(surface),
        }
    }
}

impl WithState for WindowElement {
    type State = WindowElementState;

    fn with_state<F, T>(&self, mut func: F) -> T
    where
        F: FnMut(&mut Self::State) -> T,
    {
        self.user_data()
            .insert_if_missing(RefCell::<Self::State>::default);

        let state = self
            .user_data()
            .get::<RefCell<Self::State>>()
            .expect("RefCell not in data map");

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

pub struct WindowBlocker;
pub static BLOCKER_COUNTER: AtomicU32 = AtomicU32::new(0);

impl Blocker for WindowBlocker {
    fn state(&self) -> BlockerState {
        if BLOCKER_COUNTER.load(std::sync::atomic::Ordering::SeqCst) > 0 {
            BlockerState::Pending
        } else {
            BlockerState::Released
        }
    }
}
