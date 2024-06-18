// SPDX-License-Identifier: GPL-3.0-or-later

use smithay::{
    desktop::{space::SpaceElement, WindowSurface},
    input::{
        pointer::{
            AxisFrame, ButtonEvent, Focus, GestureHoldBeginEvent, GestureHoldEndEvent,
            GesturePinchBeginEvent, GesturePinchEndEvent, GesturePinchUpdateEvent,
            GestureSwipeBeginEvent, GestureSwipeEndEvent, GestureSwipeUpdateEvent, GrabStartData,
            PointerGrab, PointerInnerHandle,
        },
        Seat, SeatHandler,
    },
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::protocol::wl_surface::WlSurface,
    },
    utils::{IsAlive, Logical, Point, Rectangle, Size},
    wayland::{compositor, seat::WaylandFocus, shell::xdg::SurfaceCachedState},
    xwayland,
};

use crate::{
    state::{Pinnacle, State, WithState},
    window::{window_state::FloatingOrTiled, WindowElement},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResizeEdge(pub xdg_toplevel::ResizeEdge);

impl From<xwayland::xwm::ResizeEdge> for ResizeEdge {
    fn from(value: xwayland::xwm::ResizeEdge) -> Self {
        match value {
            xwayland::xwm::ResizeEdge::Bottom => Self(xdg_toplevel::ResizeEdge::Bottom),
            xwayland::xwm::ResizeEdge::BottomLeft => Self(xdg_toplevel::ResizeEdge::BottomLeft),
            xwayland::xwm::ResizeEdge::BottomRight => Self(xdg_toplevel::ResizeEdge::BottomRight),
            xwayland::xwm::ResizeEdge::Left => Self(xdg_toplevel::ResizeEdge::Left),
            xwayland::xwm::ResizeEdge::Right => Self(xdg_toplevel::ResizeEdge::Right),
            xwayland::xwm::ResizeEdge::Top => Self(xdg_toplevel::ResizeEdge::Top),
            xwayland::xwm::ResizeEdge::TopLeft => Self(xdg_toplevel::ResizeEdge::TopLeft),
            xwayland::xwm::ResizeEdge::TopRight => Self(xdg_toplevel::ResizeEdge::TopRight),
        }
    }
}

impl From<xdg_toplevel::ResizeEdge> for ResizeEdge {
    fn from(value: xdg_toplevel::ResizeEdge) -> Self {
        Self(value)
    }
}

pub struct ResizeSurfaceGrab {
    start_data: GrabStartData<State>,
    window: WindowElement,

    edges: ResizeEdge,

    initial_window_loc: Point<f64, Logical>,
    initial_window_size: Size<i32, Logical>,

    last_window_size: Size<i32, Logical>,

    button_used: u32,
}

impl ResizeSurfaceGrab {
    pub fn start(
        start_data: GrabStartData<State>,
        window: WindowElement,
        edges: ResizeEdge,
        initial_window_loc: Point<f64, Logical>,
        initial_window_size: Size<i32, Logical>,
        button_used: u32,
    ) -> Option<Self> {
        window.wl_surface()?.with_state_mut(|state| {
            state.resize_state = ResizeSurfaceState::Resizing {
                edges,
                initial_window_loc,
                initial_window_size,
            };
        });

        Some(Self {
            start_data,
            window,
            edges,
            initial_window_loc,
            initial_window_size,
            last_window_size: initial_window_size,
            button_used,
        })
    }

    fn ungrab(&mut self) {
        if !self.window.alive() {
            return;
        }

        match self.window.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                toplevel.with_pending_state(|state| {
                    state.states.unset(xdg_toplevel::State::Resizing);
                    state.size = Some(self.last_window_size);
                });

                toplevel.send_pending_configure();

                toplevel.wl_surface().with_state_mut(|state| {
                    // TODO: validate resize state
                    state.resize_state = ResizeSurfaceState::WaitingForLastCommit {
                        edges: self.edges,
                        initial_window_loc: self.initial_window_loc,
                        initial_window_size: self.initial_window_size,
                    };
                });
            }
            WindowSurface::X11(surface) => {
                if surface.is_override_redirect() {
                    return;
                }
                let Some(surface) = surface.wl_surface() else { return };
                surface.with_state_mut(|state| {
                    state.resize_state = ResizeSurfaceState::WaitingForLastCommit {
                        edges: self.edges,
                        initial_window_loc: self.initial_window_loc,
                        initial_window_size: self.initial_window_size,
                    };
                });
            }
        }
    }
}

impl PointerGrab<State> for ResizeSurfaceGrab {
    fn frame(&mut self, data: &mut State, handle: &mut PointerInnerHandle<'_, State>) {
        handle.frame(data);
    }

    fn motion(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        _focus: Option<(<State as SeatHandler>::PointerFocus, Point<f64, Logical>)>,
        event: &smithay::input::pointer::MotionEvent,
    ) {
        handle.motion(data, None, event);

        if !self.window.alive() {
            handle.unset_grab(self, data, event.serial, event.time, true);
            return;
        }

        let delta = (event.location - self.start_data.location).to_i32_round::<i32>();

        let mut new_window_width = self.initial_window_size.w;
        let mut new_window_height = self.initial_window_size.h;

        if let xdg_toplevel::ResizeEdge::Left
        | xdg_toplevel::ResizeEdge::TopLeft
        | xdg_toplevel::ResizeEdge::BottomLeft = self.edges.0
        {
            new_window_width = self.initial_window_size.w - delta.x;
        }
        if let xdg_toplevel::ResizeEdge::Right
        | xdg_toplevel::ResizeEdge::TopRight
        | xdg_toplevel::ResizeEdge::BottomRight = self.edges.0
        {
            new_window_width = self.initial_window_size.w + delta.x;
        }
        if let xdg_toplevel::ResizeEdge::Top
        | xdg_toplevel::ResizeEdge::TopRight
        | xdg_toplevel::ResizeEdge::TopLeft = self.edges.0
        {
            new_window_height = self.initial_window_size.h - delta.y;
        }
        if let xdg_toplevel::ResizeEdge::Bottom
        | xdg_toplevel::ResizeEdge::BottomRight
        | xdg_toplevel::ResizeEdge::BottomLeft = self.edges.0
        {
            new_window_height = self.initial_window_size.h + delta.y;
        }

        let (min_size, max_size) = match self.window.wl_surface() {
            Some(wl_surface) => compositor::with_states(&wl_surface, |states| {
                let data = states.cached_state.current::<SurfaceCachedState>();
                (data.min_size, data.max_size)
            }),
            None => (Size::default(), Size::default()),
        };

        let min_width = i32::max(1, min_size.w);
        let min_height = i32::max(1, min_size.h);

        let max_width = if max_size.w != 0 { max_size.w } else { i32::MAX };
        let max_height = if max_size.h != 0 { max_size.h } else { i32::MAX };

        self.last_window_size = Size::from((
            new_window_width.clamp(min_width, max_width),
            new_window_height.clamp(min_height, max_height),
        ));

        match self.window.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                toplevel.with_pending_state(|state| {
                    state.states.set(xdg_toplevel::State::Resizing);
                    state.size = Some(self.last_window_size);
                });

                toplevel.send_pending_configure();
            }
            WindowSurface::X11(surface) => {
                if !surface.is_override_redirect() {
                    let loc = data
                        .pinnacle
                        .space
                        .element_location(&self.window)
                        .expect("failed to get x11 win loc");
                    surface
                        .configure(Rectangle::from_loc_and_size(loc, self.last_window_size))
                        .expect("failed to configure x11 win");
                }
            }
        }
    }

    fn relative_motion(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        focus: Option<(<State as SeatHandler>::PointerFocus, Point<f64, Logical>)>,
        event: &smithay::input::pointer::RelativeMotionEvent,
    ) {
        handle.relative_motion(data, focus, event);
    }

    fn button(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &ButtonEvent,
    ) {
        handle.button(data, event);

        if !handle.current_pressed().contains(&self.button_used) {
            handle.unset_grab(self, data, event.serial, event.time, true);
        }
    }

    fn axis(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        details: AxisFrame,
    ) {
        handle.axis(data, details);
    }

    fn start_data(&self) -> &GrabStartData<State> {
        &self.start_data
    }

    fn unset(&mut self, _data: &mut State) {
        self.ungrab();
    }

    fn gesture_swipe_begin(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GestureSwipeBeginEvent,
    ) {
        handle.gesture_swipe_begin(data, event);
    }

    fn gesture_swipe_update(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GestureSwipeUpdateEvent,
    ) {
        handle.gesture_swipe_update(data, event);
    }

    fn gesture_swipe_end(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GestureSwipeEndEvent,
    ) {
        handle.gesture_swipe_end(data, event);
    }

    fn gesture_pinch_begin(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GesturePinchBeginEvent,
    ) {
        handle.gesture_pinch_begin(data, event);
    }

    fn gesture_pinch_update(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GesturePinchUpdateEvent,
    ) {
        handle.gesture_pinch_update(data, event);
    }

    fn gesture_pinch_end(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GesturePinchEndEvent,
    ) {
        handle.gesture_pinch_end(data, event);
    }

    fn gesture_hold_begin(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GestureHoldBeginEvent,
    ) {
        handle.gesture_hold_begin(data, event);
    }

    fn gesture_hold_end(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GestureHoldEndEvent,
    ) {
        handle.gesture_hold_end(data, event);
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum ResizeSurfaceState {
    #[default]
    Idle,
    Resizing {
        edges: ResizeEdge,
        initial_window_loc: Point<f64, Logical>,
        initial_window_size: Size<i32, Logical>,
    },
    WaitingForLastCommit {
        edges: ResizeEdge,
        initial_window_loc: Point<f64, Logical>,
        initial_window_size: Size<i32, Logical>,
    },
}

impl ResizeSurfaceState {
    #[allow(clippy::type_complexity)] // FIXME:
    fn on_commit(&mut self) -> Option<(ResizeEdge, Point<f64, Logical>, Size<i32, Logical>)> {
        match *self {
            Self::Idle => None,
            Self::Resizing {
                edges,
                initial_window_loc,
                initial_window_size,
            } => Some((edges, initial_window_loc, initial_window_size)),
            Self::WaitingForLastCommit {
                edges,
                initial_window_loc,
                initial_window_size,
            } => {
                *self = Self::Idle;
                Some((edges, initial_window_loc, initial_window_size))
            }
        }
    }
}

impl Pinnacle {
    pub fn move_surface_if_resized(&mut self, surface: &WlSurface) {
        let Some(window) = self.window_for_surface(surface) else {
            return;
        };

        // FIXME: i32 -> f64
        let Some(mut window_loc) = self.space.element_location(&window).map(|loc| loc.to_f64())
        else {
            return;
        };
        let geometry = window.geometry();

        let new_loc: Option<(Option<f64>, Option<f64>)> = surface.with_state_mut(|state| {
            state.resize_state.on_commit().map(
                |(edges, initial_window_loc, initial_window_size)| {
                    let mut new_x = None;
                    let mut new_y = None;
                    if let xdg_toplevel::ResizeEdge::Left
                    | xdg_toplevel::ResizeEdge::TopLeft
                    | xdg_toplevel::ResizeEdge::BottomLeft = edges.0
                    {
                        new_x = Some(
                            initial_window_loc.x + (initial_window_size.w - geometry.size.w) as f64,
                        );
                    }
                    if let xdg_toplevel::ResizeEdge::Top
                    | xdg_toplevel::ResizeEdge::TopLeft
                    | xdg_toplevel::ResizeEdge::TopRight = edges.0
                    {
                        new_y = Some(
                            initial_window_loc.y + (initial_window_size.h - geometry.size.h) as f64,
                        );
                    }

                    (new_x, new_y)
                },
            )
        });

        if window.with_state(|state| state.floating_or_tiled.is_tiled()) {
            return;
        }

        let Some(new_loc) = new_loc else { return };

        if let Some(new_x) = new_loc.0 {
            window_loc.x = new_x;
        }
        if let Some(new_y) = new_loc.1 {
            window_loc.y = new_y;
        }

        let size = self
            .space
            .element_geometry(&window)
            .expect("called element_geometry on unmapped window")
            .size;

        window.with_state_mut(|state| {
            if state.floating_or_tiled.is_floating() {
                state.floating_or_tiled = FloatingOrTiled::Floating {
                    loc: window_loc,
                    size,
                };
            }
        });

        if new_loc.0.is_some() || new_loc.1.is_some() {
            // FIXME: space maps with i32 not f64
            self.space
                .map_element(window.clone(), window_loc.to_i32_round(), false);

            if let Some(surface) = window.x11_surface() {
                if !surface.is_override_redirect() {
                    let geo = surface.geometry();
                    // FIXME: rounding
                    let new_geo = Rectangle::from_loc_and_size(window_loc.to_i32_round(), geo.size);
                    surface
                        .configure(new_geo)
                        .expect("failed to configure x11 win");
                }
            }
        }
    }
}

impl State {
    /// The application requests a resize e.g. when you drag the edges of a window.
    pub fn resize_request_client(
        &mut self,
        surface: &WlSurface,
        seat: &Seat<State>,
        serial: smithay::utils::Serial,
        edges: self::ResizeEdge,
        button_used: u32,
    ) {
        let pointer = seat.get_pointer().expect("seat had no pointer");

        if let Some(start_data) = crate::grab::pointer_grab_start_data(&pointer, surface, serial) {
            let Some(window) = self.pinnacle.window_for_surface(surface) else {
                tracing::error!("Surface had no window, cancelling resize request");
                return;
            };

            // TODO: check for fullscreen/maximized (probably shouldn't matter)
            if window.with_state(|state| state.floating_or_tiled.is_tiled()) {
                return;
            }

            // FIXME: space stores loc as i32
            let initial_window_loc = self
                .pinnacle
                .space
                .element_location(&window)
                .expect("resize request called on unmapped window")
                .to_f64();
            let initial_window_size = window.geometry().size;

            if let Some(window) = self.pinnacle.window_for_surface(surface) {
                if let Some(toplevel) = window.toplevel() {
                    toplevel.with_pending_state(|state| {
                        state.states.set(xdg_toplevel::State::Resizing);
                    });

                    toplevel.send_pending_configure();
                }
            }

            let grab = ResizeSurfaceGrab::start(
                start_data,
                window,
                edges,
                initial_window_loc,
                initial_window_size,
                button_used,
            );

            if let Some(grab) = grab {
                pointer.set_grab(self, grab, serial, Focus::Clear);
            }
        }
    }

    /// The compositor requested a resize e.g. you hold the mod key and right-click drag.
    pub fn resize_request_server(
        &mut self,
        surface: &WlSurface,
        seat: &Seat<State>,
        serial: smithay::utils::Serial,
        edges: self::ResizeEdge,
        button_used: u32,
    ) {
        let pointer = seat.get_pointer().expect("seat had no pointer");

        let Some(window) = self.pinnacle.window_for_surface(surface) else {
            tracing::error!("Surface had no window, cancelling resize request");
            return;
        };

        if window.with_state(|state| state.floating_or_tiled.is_tiled()) {
            return;
        }

        // FIXME: i32 -> f64
        let initial_window_loc = self
            .pinnacle
            .space
            .element_location(&window)
            .expect("resize request called on unmapped window")
            .to_f64();
        let initial_window_size = window.geometry().size;

        if let Some(window) = self.pinnacle.window_for_surface(surface) {
            if let Some(toplevel) = window.toplevel() {
                toplevel.with_pending_state(|state| {
                    state.states.set(xdg_toplevel::State::Resizing);
                });

                toplevel.send_pending_configure();
            }
        }

        let start_data = smithay::input::pointer::GrabStartData {
            focus: pointer
                .current_focus()
                .map(|focus| (focus, initial_window_loc)),
            button: button_used,
            location: pointer.current_location(),
        };

        let grab = ResizeSurfaceGrab::start(
            start_data,
            window,
            edges,
            initial_window_loc,
            initial_window_size,
            button_used,
        );

        if let Some(grab) = grab {
            pointer.set_grab(self, grab, serial, Focus::Clear);
        }
    }
}
