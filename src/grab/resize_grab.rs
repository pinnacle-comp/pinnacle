// SPDX-License-Identifier: GPL-3.0-or-later

use smithay::{
    desktop::{space::SpaceElement, WindowSurface},
    input::{
        pointer::{
            AxisFrame, ButtonEvent, CursorIcon, CursorImageStatus, Focus, GestureHoldBeginEvent,
            GestureHoldEndEvent, GesturePinchBeginEvent, GesturePinchEndEvent,
            GesturePinchUpdateEvent, GestureSwipeBeginEvent, GestureSwipeEndEvent,
            GestureSwipeUpdateEvent, GrabStartData, PointerGrab, PointerInnerHandle,
        },
        Seat, SeatHandler,
    },
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::protocol::wl_surface::WlSurface,
    },
    utils::{IsAlive, Logical, Point, Rectangle, Size},
    wayland::{compositor, shell::xdg::SurfaceCachedState},
    xwayland,
};
use tracing::warn;

use crate::{
    state::{State, WithState},
    util::transaction::TransactionBuilder,
    window::WindowElement,
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

impl ResizeEdge {
    fn cursor_icon(&self) -> CursorIcon {
        match self.0 {
            xdg_toplevel::ResizeEdge::None => CursorIcon::Default, // TODO: possibly different icon here?
            xdg_toplevel::ResizeEdge::Top => CursorIcon::NResize,
            xdg_toplevel::ResizeEdge::Bottom => CursorIcon::SResize,
            xdg_toplevel::ResizeEdge::Left => CursorIcon::WResize,
            xdg_toplevel::ResizeEdge::TopLeft => CursorIcon::NwResize,
            xdg_toplevel::ResizeEdge::BottomLeft => CursorIcon::SwResize,
            xdg_toplevel::ResizeEdge::Right => CursorIcon::EResize,
            xdg_toplevel::ResizeEdge::TopRight => CursorIcon::NeResize,
            xdg_toplevel::ResizeEdge::BottomRight => CursorIcon::SeResize,
            _ => CursorIcon::Default,
        }
    }
}

pub struct ResizeSurfaceGrab {
    start_data: GrabStartData<State>,
    window: WindowElement,
    edges: ResizeEdge,
    initial_window_geo: Rectangle<i32, Logical>,
    last_window_size: Size<i32, Logical>,
    button_used: u32,
}

impl ResizeSurfaceGrab {
    pub fn start(
        start_data: GrabStartData<State>,
        window: WindowElement,
        edges: ResizeEdge,
        initial_window_geo: Rectangle<i32, Logical>,
        button_used: u32,
    ) -> Option<Self> {
        Some(Self {
            start_data,
            window,
            edges,
            initial_window_geo,
            last_window_size: initial_window_geo.size,
            button_used,
        })
    }

    fn ungrab(&mut self) {
        if !self.window.alive() {
            return;
        }

        if let Some(toplevel) = self.window.toplevel() {
            toplevel.with_pending_state(|state| {
                state.states.unset(xdg_toplevel::State::Resizing);
            });

            toplevel.send_pending_configure();
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

        if data.pinnacle.layout_state.pending_resize {
            return;
        }

        // TODO: if-let chains in 1.88
        let output = self.window.output(&data.pinnacle);

        if !self.window.alive() || output.is_none() {
            data.pinnacle
                .cursor_state
                .set_cursor_image(CursorImageStatus::default_named());
            handle.unset_grab(self, data, event.serial, event.time, true);
            return;
        }

        let Some(output) = output else {
            unreachable!();
        };

        let delta = (event.location - self.start_data.location).to_i32_round::<i32>();

        let mut new_window_width = self.initial_window_geo.size.w;
        let mut new_window_height = self.initial_window_geo.size.h;

        if let xdg_toplevel::ResizeEdge::Left
        | xdg_toplevel::ResizeEdge::TopLeft
        | xdg_toplevel::ResizeEdge::BottomLeft = self.edges.0
        {
            new_window_width = self.initial_window_geo.size.w - delta.x;
        }
        if let xdg_toplevel::ResizeEdge::Right
        | xdg_toplevel::ResizeEdge::TopRight
        | xdg_toplevel::ResizeEdge::BottomRight = self.edges.0
        {
            new_window_width = self.initial_window_geo.size.w + delta.x;
        }
        if let xdg_toplevel::ResizeEdge::Top
        | xdg_toplevel::ResizeEdge::TopRight
        | xdg_toplevel::ResizeEdge::TopLeft = self.edges.0
        {
            new_window_height = self.initial_window_geo.size.h - delta.y;
        }
        if let xdg_toplevel::ResizeEdge::Bottom
        | xdg_toplevel::ResizeEdge::BottomRight
        | xdg_toplevel::ResizeEdge::BottomLeft = self.edges.0
        {
            new_window_height = self.initial_window_geo.size.h + delta.y;
        }

        let (min_size, max_size) = match self.window.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                compositor::with_states(toplevel.wl_surface(), |states| {
                    let mut guard = states.cached_state.get::<SurfaceCachedState>();
                    let data = guard.current();
                    (data.min_size, data.max_size)
                })
            }
            WindowSurface::X11(surface) => (
                surface.min_size().unwrap_or_default(),
                surface.max_size().unwrap_or_default(),
            ),
        };

        let min_width = i32::max(1, min_size.w);
        let min_height = i32::max(1, min_size.h);

        let max_width = if max_size.w != 0 { max_size.w } else { i32::MAX };
        let max_height = if max_size.h != 0 { max_size.h } else { i32::MAX };

        self.last_window_size = Size::from((
            new_window_width.clamp(min_width, max_width),
            new_window_height.clamp(min_height, max_height),
        ));

        self.window
            .with_state_mut(|state| state.floating_size = self.last_window_size);

        let serial = match self.window.underlying_surface() {
            WindowSurface::Wayland(toplevel) => {
                toplevel.with_pending_state(|state| {
                    state.states.set(xdg_toplevel::State::Resizing);
                    state.size = Some(self.last_window_size);
                });

                toplevel.send_pending_configure()
            }
            WindowSurface::X11(surface) => {
                if !surface.is_override_redirect() {
                    let loc = self.initial_window_geo.loc + delta;
                    let _ = surface.configure(Rectangle::new(loc, self.last_window_size));
                }

                None
            }
        };

        data.pinnacle.layout_state.pending_resize = true;

        let mut transaction_builder = TransactionBuilder::new(false, true);
        transaction_builder.add(
            &self.window,
            // We need the updated window size to correctly map the window, so set
            // the target_loc to the bottom right corner then subtract the
            // updated size when updating the layout.
            self.initial_window_geo.loc + self.initial_window_geo.size,
            serial,
            &data.pinnacle.loop_handle,
        );
        data.pinnacle
            .layout_state
            .pending_transactions
            .entry(output.downgrade())
            .or_default()
            .push(transaction_builder.into_pending(Vec::new()));
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
            data.pinnacle
                .cursor_state
                .set_cursor_image(CursorImageStatus::default_named());
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
            let Some(window) = self.pinnacle.window_for_surface(surface).cloned() else {
                tracing::error!("Surface had no window, cancelling resize request");
                return;
            };

            if window.with_state(|state| !state.layout_mode.is_floating()) {
                return;
            }

            let Some(initial_window_loc) = self.pinnacle.space.element_location(&window) else {
                return;
            };
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
                Rectangle::new(initial_window_loc, initial_window_size),
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

        let Some(window) = self.pinnacle.window_for_surface(surface).cloned() else {
            tracing::error!("Surface had no window, cancelling resize request");
            return;
        };

        if window.with_state(|state| !state.layout_mode.is_floating()) {
            return;
        }

        let Some(initial_window_loc) = self.pinnacle.space.element_location(&window) else {
            warn!("Resize request on unmapped surface");
            return;
        };
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
            focus: None,
            button: button_used,
            location: pointer.current_location(),
        };

        let grab = ResizeSurfaceGrab::start(
            start_data,
            window,
            edges,
            Rectangle::new(initial_window_loc, initial_window_size),
            button_used,
        );

        if let Some(grab) = grab {
            pointer.set_grab(self, grab, serial, Focus::Clear);

            self.pinnacle
                .cursor_state
                .set_cursor_image(CursorImageStatus::Named(edges.cursor_icon()));
        }
    }
}
