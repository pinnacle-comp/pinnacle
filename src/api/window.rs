mod v1;

use smithay::{
    desktop::space::SpaceElement,
    reexports::wayland_protocols::xdg::{
        decoration::zv1::server::zxdg_toplevel_decoration_v1, shell::server,
    },
    utils::{Point, SERIAL_COUNTER, Size},
    wayland::seat::WaylandFocus,
};
use tracing::warn;

use crate::{
    focus::keyboard::KeyboardFocusTarget,
    state::{State, WithState},
    tag::Tag,
    window::WindowElement,
};

use super::StateFnSender;

pub struct WindowService {
    sender: StateFnSender,
}

impl WindowService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}

pub fn set_geometry(
    state: &mut State,
    window: &WindowElement,
    x: impl Into<Option<i32>>,
    y: impl Into<Option<i32>>,
    w: impl Into<Option<u32>>,
    h: impl Into<Option<u32>>,
) {
    let x: Option<i32> = x.into();
    let y: Option<i32> = y.into();
    let w: Option<u32> = w.into();
    let h: Option<u32> = h.into();

    let mut window_size = window.with_state(|state| state.floating_size);
    if window_size.w == 0 {
        window_size.w = window.geometry().size.w;
    }
    if window_size.h == 0 {
        window_size.h = window.geometry().size.h;
    }

    window_size.w = w.map(|w| w as i32).unwrap_or(window_size.w);
    window_size.h = h.map(|h| h as i32).unwrap_or(window_size.h);

    window.with_state_mut(|state| {
        state.floating_x = x.or(state.floating_x);
        state.floating_y = y.or(state.floating_y);
        state.floating_size = window_size;
    });

    state.update_window_layout_mode_and_layout(window, |_| ());
}

// TODO: minimized

/// Sets a window to focused or not.
///
/// If the window is on another output and an attempt is made to
/// focus it, the focused output will change to that output UNLESS
/// the window overlaps the currently focused output.
pub fn set_focused(state: &mut State, window: &WindowElement, set: impl Into<Option<bool>>) {
    if window.is_x11_override_redirect() {
        return;
    }

    let Some(output) = window.output(&state.pinnacle) else {
        return;
    };

    let set = set.into();

    let Some(keyboard) = state.pinnacle.seat.get_keyboard() else {
        return;
    };

    let is_focused = keyboard
        .current_focus()
        .is_some_and(|focus| matches!(focus, KeyboardFocusTarget::Window(win) if win == window));

    let set = match set {
        Some(set) => set,
        None => !is_focused,
    };

    if set {
        state
            .pinnacle
            .keyboard_focus_stack
            .set_focus(window.clone());

        state.pinnacle.on_demand_layer_focus = None;

        let window_outputs = state
            .pinnacle
            .space
            .outputs()
            .filter(|op| {
                let win_geo = state.pinnacle.space.element_geometry(window);
                let op_geo = state.pinnacle.space.output_geometry(op);

                if let (Some(win_geo), Some(op_geo)) = (win_geo, op_geo) {
                    win_geo.overlaps(op_geo)
                } else {
                    false
                }
            })
            .collect::<Vec<_>>();

        if window_outputs.is_empty() {
            warn!("Cannot focus an unmapped window");
            return;
        }

        if window_outputs.len() == 1 {
            state.pinnacle.focus_output(&output);
        } else {
            let currently_focused_op = state.pinnacle.focused_output();
            match currently_focused_op {
                Some(op) => {
                    if !window_outputs.contains(&op) {
                        state.pinnacle.focus_output(&output);
                    }
                }
                None => {
                    state.pinnacle.focus_output(&output);
                }
            }
        }
    } else {
        state.pinnacle.keyboard_focus_stack.unset_focus();
    }
}

pub fn set_decoration_mode(
    _state: &mut State,
    window: &WindowElement,
    decoration_mode: zxdg_toplevel_decoration_v1::Mode,
) {
    window.with_state_mut(|state| {
        state.decoration_mode = Some(decoration_mode);
    });

    if let Some(toplevel) = window.toplevel() {
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(decoration_mode);
        });

        crate::handlers::decoration::update_kde_decoration_mode(
            toplevel.wl_surface(),
            decoration_mode,
        );

        toplevel.send_pending_configure();
    }
}

pub fn move_to_tag(state: &mut State, window: &WindowElement, tag: &Tag) {
    let source_output = window.output(&state.pinnacle);

    window.with_state_mut(|state| {
        state.tags = std::iter::once(tag.clone()).collect();
    });

    if let Some(output) = source_output.as_ref() {
        state.pinnacle.request_layout(output);
        state.schedule_render(output);
    }

    let Some(target_output) = tag.output(&state.pinnacle) else {
        state.pinnacle.update_xwayland_stacking_order();
        return;
    };

    if source_output.as_ref() != Some(&target_output) && tag.active() {
        state.pinnacle.request_layout(&target_output);
        state.schedule_render(&target_output);
    }

    state.pinnacle.update_xwayland_stacking_order();
}

pub fn set_tag(state: &mut State, window: &WindowElement, tag: &Tag, set: impl Into<Option<bool>>) {
    let set = set.into();

    match set {
        Some(true) => {
            window.with_state_mut(|state| state.tags.insert(tag.clone()));
        }
        Some(false) => {
            window.with_state_mut(|state| state.tags.shift_remove(tag));
        }
        None => {
            window.with_state_mut(|state| {
                if state.tags.contains(tag) {
                    // Prevent toggling that would leave a window tagless
                    if state.tags.len() > 1 {
                        state.tags.shift_remove(tag);
                    }
                } else {
                    state.tags.insert(tag.clone());
                }
            });
        }
    }

    let Some(output) = tag.output(&state.pinnacle) else {
        return;
    };

    state.pinnacle.request_layout(&output);
    state.schedule_render(&output);
    state.pinnacle.update_xwayland_stacking_order();
}

pub fn raise(state: &mut State, window: WindowElement) {
    for output in state.pinnacle.space.outputs_for_element(&window) {
        state.schedule_render(&output);
    }

    state.pinnacle.raise_window(window);
}

pub fn move_grab(state: &mut State, button: u32) {
    let Some((pointer_focus, _)) = state.pinnacle.pointer_contents.focus_under.as_ref() else {
        return;
    };
    let Some(window) = pointer_focus.window_for(&state.pinnacle) else {
        return;
    };
    let Some(wl_surf) = window.wl_surface() else {
        return;
    };
    let seat = state.pinnacle.seat.clone();

    state.move_request_server(&wl_surf, &seat, SERIAL_COUNTER.next_serial(), button);

    if let Some(output) = state.pinnacle.focused_output().cloned() {
        state.schedule_render(&output);
    }
}

pub fn resize_grab(state: &mut State, button: u32) {
    let Some(pointer_loc) = state
        .pinnacle
        .seat
        .get_pointer()
        .map(|ptr| ptr.current_location())
    else {
        return;
    };
    let Some((pointer_focus, _window_loc)) = state.pinnacle.pointer_contents.focus_under.as_ref()
    else {
        return;
    };
    let Some(window) = pointer_focus.window_for(&state.pinnacle) else {
        return;
    };
    let Some(wl_surf) = window.wl_surface() else {
        return;
    };
    let Some(window_loc) = state.pinnacle.space.element_location(&window) else {
        return;
    };

    let pointer_loc: Point<i32, _> = pointer_loc.to_i32_round();

    let window_geometry = window.geometry();
    let window_sz = window_geometry.size;
    let window_width = Size::new(window_sz.w, 0);
    let window_height = Size::new(0, window_sz.h);

    let rel_x = (pointer_loc.x - window_loc.x).clamp(0, window_sz.w);
    let rel_y = (pointer_loc.y - window_loc.y).clamp(0, window_sz.h);

    let quadrant_x = (rel_x * 3 / window_sz.w).clamp(0, 2);
    let quadrant_y = (rel_y * 3 / window_sz.h).clamp(0, 2);

    let edges = match (quadrant_x, quadrant_y) {
        (0, 0) => server::xdg_toplevel::ResizeEdge::TopLeft,
        (2, 0) => server::xdg_toplevel::ResizeEdge::TopRight,
        (0, 2) => server::xdg_toplevel::ResizeEdge::BottomLeft,
        (2, 2) => server::xdg_toplevel::ResizeEdge::BottomRight,

        (1, 0) => server::xdg_toplevel::ResizeEdge::Top,
        (1, 2) => server::xdg_toplevel::ResizeEdge::Bottom,
        (0, 1) => server::xdg_toplevel::ResizeEdge::Left,
        (2, 1) => server::xdg_toplevel::ResizeEdge::Right,

        _ => server::xdg_toplevel::ResizeEdge::None,
    };

    let edges = if edges != server::xdg_toplevel::ResizeEdge::None {
        edges
    } else {
        // Find the closest edge by figuring out which corners the pointer is between.
        // This works by drawing lines from the window's center to all four corners and the pointer.
        // Whichever two lines the pointer line is between determines the edge chosen.
        
        // A bit of an explanation here.
        //
        // The cross product of two vector is `||v1|| * ||v2|| * sin(th)`, with `th` being the
        // angle between the vectors. Since `sin(th)` is the only factor influencing the
        // signed-ness, we can use that to find the 'direction' of the rotation to go from one
        // vector to the other.
        //
        // More formally, given v1 and v2 such that the angle v1->v2 is between 0 and 180⁰, a
        // third vector v is between v1 and v2 if (v1)x(v) > 0 and (v)x(v2) > 0.

        fn cross(lhs: (i32, i32), rhs: (i32, i32)) -> i32 {
            lhs.0 * rhs.1 - lhs.1 * rhs.0
        }

        let top_left = window_loc;
        let top_right = window_loc + window_width;
        let bottom_left = window_loc + window_height;
        let bottom_right = window_loc + window_sz;
        let window_center = window_loc + window_sz.downscale(2);

        let v_tl: (i32, i32) = (top_left - window_center).into();
        let v_tr: (i32, i32) = (top_right - window_center).into();
        let v_bl: (i32, i32) = (bottom_left - window_center).into();
        let v_br: (i32, i32) = (bottom_right - window_center).into();
        let v_pointer: (i32, i32) = (pointer_loc - window_center).into();

        let vectors = [
            (v_tl, v_tr, server::xdg_toplevel::ResizeEdge::Top),
            (v_tr, v_br, server::xdg_toplevel::ResizeEdge::Right),
            (v_br, v_bl, server::xdg_toplevel::ResizeEdge::Bottom),
            (v_bl, v_tl, server::xdg_toplevel::ResizeEdge::Left),
        ];

        vectors
            .into_iter()
            .map(|(v1, v2, e)| {
                (
                    cross(v1, v_pointer).signum(),
                    cross(v_pointer, v2).signum(),
                    e,
                )
            })
            .find(|(s1, s2, _)| *s1 >= 0 && *s2 >= 0)
            .map(|(_, _, e)| e)
            .unwrap_or(server::xdg_toplevel::ResizeEdge::None)
    };

    state.resize_request_server(
        &wl_surf,
        &state.pinnacle.seat.clone(),
        SERIAL_COUNTER.next_serial(),
        edges.into(),
        button,
    );

    if let Some(output) = state.pinnacle.focused_output().cloned() {
        state.schedule_render(&output);
    }
}
