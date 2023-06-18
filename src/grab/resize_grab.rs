use smithay::{
    desktop::Window,
    input::{
        pointer::{AxisFrame, ButtonEvent, GrabStartData, PointerGrab, PointerInnerHandle},
        SeatHandler,
    },
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel::{self, ResizeEdge},
        wayland_server::protocol::wl_surface::WlSurface,
    },
    utils::{IsAlive, Logical, Point, Rectangle, Size},
    wayland::{compositor, seat::WaylandFocus, shell::xdg::SurfaceCachedState},
};

use crate::{backend::Backend, state::State, window::SurfaceState};

pub struct ResizeSurfaceGrab<S: SeatHandler> {
    start_data: GrabStartData<S>,
    window: Window,

    edges: ResizeEdge,

    initial_window_rect: Rectangle<i32, Logical>,
    last_window_size: Size<i32, Logical>,

    button_used: u32,
}

impl<S: SeatHandler> ResizeSurfaceGrab<S> {
    pub fn start(
        start_data: GrabStartData<S>,
        window: Window,
        edges: ResizeEdge,
        initial_window_rect: Rectangle<i32, Logical>,
        button_used: u32,
    ) -> Self {
        ResizeSurfaceState::with_state(window.toplevel().wl_surface(), |state| {
            *state = ResizeSurfaceState::Resizing {
                edges,
                initial_window_rect,
            };
        });

        Self {
            start_data,
            window,
            edges,
            initial_window_rect,
            last_window_size: initial_window_rect.size,
            button_used,
        }
    }
}

impl<B: Backend> PointerGrab<State<B>> for ResizeSurfaceGrab<State<B>> {
    fn motion(
        &mut self,
        data: &mut State<B>,
        handle: &mut PointerInnerHandle<'_, State<B>>,
        _focus: Option<(<State<B> as SeatHandler>::PointerFocus, Point<i32, Logical>)>,
        event: &smithay::input::pointer::MotionEvent,
    ) {
        handle.motion(data, None, event);

        if !self.window.alive() {
            handle.unset_grab(data, event.serial, event.time);
            return;
        }

        let delta = (event.location - self.start_data.location).to_i32_round::<i32>();

        let mut new_window_width = self.initial_window_rect.size.w;
        let mut new_window_height = self.initial_window_rect.size.h;

        if let ResizeEdge::Left | ResizeEdge::TopLeft | ResizeEdge::BottomLeft = self.edges {
            new_window_width = self.initial_window_rect.size.w - delta.x;
        }
        if let ResizeEdge::Right | ResizeEdge::TopRight | ResizeEdge::BottomRight = self.edges {
            new_window_width = self.initial_window_rect.size.w + delta.x;
        }
        if let ResizeEdge::Top | ResizeEdge::TopRight | ResizeEdge::TopLeft = self.edges {
            new_window_height = self.initial_window_rect.size.h - delta.y;
        }
        if let ResizeEdge::Bottom | ResizeEdge::BottomRight | ResizeEdge::BottomLeft = self.edges {
            new_window_height = self.initial_window_rect.size.h + delta.y;
        }

        let (min_size, max_size) = match self.window.wl_surface() {
            Some(wl_surface) => compositor::with_states(&wl_surface, |states| {
                let data = states.cached_state.current::<SurfaceCachedState>();
                (data.min_size, data.max_size)
            }),
            None => ((0, 0).into(), (0, 0).into()),
        };

        let min_width = i32::min(1, min_size.w);
        let min_height = i32::min(1, min_size.h);

        let max_width = if max_size.w != 0 {
            max_size.w
        } else {
            i32::MAX
        };
        let max_height = if max_size.h != 0 {
            max_size.h
        } else {
            i32::MAX
        };

        self.last_window_size = Size::from((
            new_window_width.clamp(min_width, max_width),
            new_window_height.clamp(min_height, max_height),
        ));

        let toplevel_surface = self.window.toplevel();

        toplevel_surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Resizing);
            state.size = Some(self.last_window_size);
        });

        toplevel_surface.send_pending_configure();
    }

    fn relative_motion(
        &mut self,
        data: &mut State<B>,
        handle: &mut PointerInnerHandle<'_, State<B>>,
        focus: Option<(<State<B> as SeatHandler>::PointerFocus, Point<i32, Logical>)>,
        event: &smithay::input::pointer::RelativeMotionEvent,
    ) {
        handle.relative_motion(data, focus, event);
    }

    fn button(
        &mut self,
        data: &mut State<B>,
        handle: &mut PointerInnerHandle<'_, State<B>>,
        event: &ButtonEvent,
    ) {
        handle.button(data, event);

        if !handle.current_pressed().contains(&self.button_used) {
            handle.unset_grab(data, event.serial, event.time);

            let toplevel_surface = self.window.toplevel();
            toplevel_surface.with_pending_state(|state| {
                state.states.unset(xdg_toplevel::State::Resizing);
                state.size = Some(self.last_window_size);
            });

            toplevel_surface.send_pending_configure();

            ResizeSurfaceState::with_state(toplevel_surface.wl_surface(), |state| {
                *state = ResizeSurfaceState::WaitingForLastCommit {
                    edges: self.edges,
                    initial_window_rect: self.initial_window_rect,
                };
            });
        }
    }

    fn axis(
        &mut self,
        data: &mut State<B>,
        handle: &mut PointerInnerHandle<'_, State<B>>,
        details: AxisFrame,
    ) {
        handle.axis(data, details);
    }

    fn start_data(&self) -> &GrabStartData<State<B>> {
        &self.start_data
    }
}

#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
enum ResizeSurfaceState {
    #[default]
    Idle,
    Resizing {
        edges: ResizeEdge,
        initial_window_rect: Rectangle<i32, Logical>,
    },
    WaitingForLastCommit {
        edges: ResizeEdge,
        initial_window_rect: Rectangle<i32, Logical>,
    },
}

impl ResizeSurfaceState {
    fn commit(&mut self) -> Option<(ResizeEdge, Rectangle<i32, Logical>)> {
        match *self {
            Self::Idle => None,
            Self::Resizing {
                edges,
                initial_window_rect,
            } => Some((edges, initial_window_rect)),
            Self::WaitingForLastCommit {
                edges,
                initial_window_rect,
            } => {
                *self = Self::Idle;
                Some((edges, initial_window_rect))
            }
        }
    }
}

impl SurfaceState for ResizeSurfaceState {}

pub fn handle_commit<B: Backend>(state: &mut State<B>, surface: &WlSurface) -> Option<()> {
    let window = state.window_for_surface(surface)?;
    let mut window_loc = state.space.element_location(&window)?;
    let geometry = window.geometry();

    let new_loc: Point<Option<i32>, Logical> = ResizeSurfaceState::with_state(surface, |state| {
        state
            .commit()
            .map(|(edges, initial_window_rect)| {
                let mut new_x: Option<i32> = None;
                let mut new_y: Option<i32> = None;
                if let ResizeEdge::Left | ResizeEdge::TopLeft | ResizeEdge::BottomLeft = edges {
                    new_x = Some(
                        initial_window_rect.loc.x + (initial_window_rect.size.w - geometry.size.w),
                    );
                }
                if let ResizeEdge::Top | ResizeEdge::TopLeft | ResizeEdge::TopRight = edges {
                    new_y = Some(
                        initial_window_rect.loc.y + (initial_window_rect.size.h - geometry.size.h),
                    );
                }

                (new_x, new_y)
            })
            .unwrap_or_default()
            .into()
    });

    if let Some(new_x) = new_loc.x {
        window_loc.x = new_x;
    }
    if let Some(new_y) = new_loc.y {
        window_loc.y = new_y;
    }

    if new_loc.x.is_some() || new_loc.y.is_some() {
        state.space.map_element(window, window_loc, false);
    }

    Some(())
}
