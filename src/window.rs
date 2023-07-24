// SPDX-License-Identifier: GPL-3.0-or-later

use std::{cell::RefCell, sync::atomic::AtomicU32, time::Duration};

use smithay::{
    desktop::{
        utils::{
            send_dmabuf_feedback_surface_tree, send_frames_surface_tree,
            take_presentation_feedback_surface_tree, with_surfaces_surface_tree,
            OutputPresentationFeedback,
        },
        Window, WindowSurfaceType,
    },
    output::Output,
    reexports::{
        wayland_protocols::{
            wp::presentation_time::server::wp_presentation_feedback,
            xdg::shell::server::xdg_toplevel,
        },
        wayland_server::protocol::wl_surface::WlSurface,
    },
    utils::{user_data::UserDataMap, IsAlive, Logical, Point},
    wayland::{
        compositor::{Blocker, BlockerState, SurfaceData},
        dmabuf::DmabufFeedback,
        seat::WaylandFocus,
    },
    xwayland::X11Surface,
};

use crate::{
    backend::Backend,
    state::{State, WithState},
};

use self::window_state::{Float, WindowState};

pub mod window_state;

#[derive(Debug, Clone, PartialEq)]
pub enum WindowElement {
    Wayland(Window),
    X11(X11Surface),
}

impl WindowElement {
    pub fn surface_under(
        &self,
        location: Point<i32, Logical>,
        window_type: WindowSurfaceType,
    ) -> Option<(WlSurface, Point<i32, Logical>)> {
        todo!()
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
}

impl IsAlive for WindowElement {
    fn alive(&self) -> bool {
        match self {
            WindowElement::Wayland(window) => window.alive(),
            WindowElement::X11(surface) => surface.alive(),
        }
    }
}

impl WithState for WindowElement {
    type State = WindowState;

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

impl<B: Backend> State<B> {
    /// Returns the [Window] associated with a given [WlSurface].
    pub fn window_for_surface(&self, surface: &WlSurface) -> Option<Window> {
        self.space
            .elements()
            .find(|window| window.wl_surface().map(|s| s == *surface).unwrap_or(false))
            .cloned()
            .or_else(|| {
                self.windows
                    .iter()
                    .find(|&win| win.toplevel().wl_surface() == surface)
                    .cloned()
            })
    }
}

/// Toggle a window's floating status.
pub fn toggle_floating<B: Backend>(state: &mut State<B>, window: &Window) {
    window.with_state(|window_state| {
        match window_state.floating {
            Float::Tiled(prev_loc_and_size) => {
                if let Some((prev_loc, prev_size)) = prev_loc_and_size {
                    window.toplevel().with_pending_state(|state| {
                        state.size = Some(prev_size);
                    });

                    window.toplevel().send_pending_configure();

                    state.space.map_element(window.clone(), prev_loc, false);
                    // TODO: should it activate?
                }

                window_state.floating = Float::Floating;
                window.toplevel().with_pending_state(|tl_state| {
                    tl_state.states.unset(xdg_toplevel::State::TiledTop);
                    tl_state.states.unset(xdg_toplevel::State::TiledBottom);
                    tl_state.states.unset(xdg_toplevel::State::TiledLeft);
                    tl_state.states.unset(xdg_toplevel::State::TiledRight);
                });
            }
            Float::Floating => {
                window_state.floating = Float::Tiled(Some((
                    // We get the location this way because window.geometry().loc
                    // doesn't seem to be the actual location
                    state.space.element_location(window).unwrap(),
                    window.geometry().size,
                )));
                window.toplevel().with_pending_state(|tl_state| {
                    tl_state.states.set(xdg_toplevel::State::TiledTop);
                    tl_state.states.set(xdg_toplevel::State::TiledBottom);
                    tl_state.states.set(xdg_toplevel::State::TiledLeft);
                    tl_state.states.set(xdg_toplevel::State::TiledRight);
                });
            }
        }
    });

    let output = state.focus_state.focused_output.clone().unwrap();
    state.re_layout(&output);

    let render = output.with_state(|op_state| {
        state
            .windows
            .iter()
            .cloned()
            .filter(|win| {
                win.with_state(|win_state| {
                    if win_state.floating.is_floating() {
                        return true;
                    }
                    for tag in win_state.tags.iter() {
                        if op_state.focused_tags().any(|tg| tg == tag) {
                            return true;
                        }
                    }
                    false
                })
            })
            .collect::<Vec<_>>()
    });

    let clone = window.clone();
    state.loop_handle.insert_idle(move |data| {
        crate::state::schedule_on_commit(data, render, move |dt| {
            dt.state.space.raise_element(&clone, true);
        });
    });
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
