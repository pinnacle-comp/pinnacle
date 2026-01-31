mod v1;

use smithay::{
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
    util::transaction::TransactionBuilder,
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

    state.pinnacle.update_window_geometry(
        window,
        window.with_state(|state| state.layout_mode.is_tiled()),
    );
}

/// Sets or toggles if a window is minimized.
///
/// Minimized windows are always unfocused.
pub fn set_minimized(state: &mut State, window: &WindowElement, set: impl Into<Option<bool>>) {
    if window.is_x11_override_redirect() {
        return;
    }

    let set = set.into();

    let is_minimized = window.with_state(|state| state.minimized);
    let set = match set {
        Some(absolute_set) => absolute_set,
        None => !is_minimized,
    };
    window.with_state_mut(|state| state.minimized = set);

    if !set && state.pinnacle.keyboard_focus_stack.current_focus() == Some(window) {
        state.pinnacle.keyboard_focus_stack.unset_focus();
    }

    // Note: tag moving will automatically adjust the output on the window directly even if
    // minimised, so we can rely on this.
    let Some(output) = window.output(&state.pinnacle) else {
        warn!("adjusted minimization-state of window without an output.");
        return;
    };

    // This means we can rely on the output associated with the [`WindowElementState`] even while
    // minimized, and we can use it to schedule layouts.
    if set != is_minimized {
        state.pinnacle.request_layout(&output);
        state.schedule_render(&output);
        state.pinnacle.update_xwayland_stacking_order();
    }
}

/// Sets a window to focused or not.
///
/// If the window is on another output and an attempt is made to
/// focus it, the focused output will change to that output UNLESS
/// the window overlaps the currently focused output.
///
/// If the window is being set to be focused, then if it's minimized,
/// this will automatically unminimize it.
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

        if window.with_state(|window_state| window_state.minimized) {
            // Will automatically do correct scheduling of re-layouting and such ^.^
            set_minimized(state, window, false);
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

pub fn lower(state: &mut State, window: WindowElement) {
    for output in state.pinnacle.space.outputs_for_element(&window) {
        state.schedule_render(&output);
    }

    state.pinnacle.lower_window(window);
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

    let window_size = window.geometry().size;
    let window_width = Size::new(window_size.w, 0);
    let window_height = Size::new(0, window_size.h);

    let rel_x = (pointer_loc.x - window_loc.x).clamp(0, window_size.w);
    let rel_y = (pointer_loc.y - window_loc.y).clamp(0, window_size.h);

    let quadrant_x = (rel_x * 3 / window_size.w).clamp(0, 2);
    let quadrant_y = (rel_y * 3 / window_size.h).clamp(0, 2);

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
        let bottom_right = window_loc + window_size;
        let window_center = window_loc + window_size.downscale(2);

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

pub fn swap(state: &mut State, window: WindowElement, target: WindowElement) {
    if state.pinnacle.layout_state.pending_swap {
        return;
    }

    if window == target {
        return;
    }

    let output = window.output(&state.pinnacle);
    let target_output = target.output(&state.pinnacle);

    let Some((output, target_output)) = output.zip(target_output) else {
        tracing::warn!("Can't swap windows without output");
        return;
    };

    let window_was_on_active_tag = window.is_on_active_tag();
    let target_was_on_active_tag = target.is_on_active_tag();

    tracing::debug!("Swapping window positions");
    state.pinnacle.layout_state.pending_swap = true;
    state.pinnacle.swap_window_positions(&window, &target);

    tracing::debug!("Swapping window tags");
    let window_tags = window.with_state(|state| state.tags.clone());
    let target_tags = target.with_state(|state| state.tags.clone());

    window.with_state_mut(|state| state.tags = target_tags);
    target.with_state_mut(|state| state.tags = window_tags);

    // Swap floating attribute. In case of cross-output swap, this prevent window jumping back.
    let window_floating_x = window.with_state(|state| state.floating_x);
    let window_floating_y = window.with_state(|state| state.floating_y);
    let window_floating_size = window.with_state(|state| state.floating_size);

    let target_floating_x = target.with_state(|state| state.floating_x);
    let target_floating_y = target.with_state(|state| state.floating_y);
    let target_floating_size = target.with_state(|state| state.floating_size);

    target.with_state_mut(|state| {
        state.floating_x = window_floating_x;
        state.floating_y = window_floating_y;
        state.floating_size = window_floating_size
    });

    window.with_state_mut(|state| {
        state.floating_x = target_floating_x;
        state.floating_y = target_floating_y;
        state.floating_size = target_floating_size
    });

    let window_layout_mode = window.with_state(|state| state.layout_mode);
    let window_geo = state.pinnacle.space.element_geometry(&window);

    let target_layout_mode = target.with_state(|state| state.layout_mode);
    let target_geo = state.pinnacle.space.element_geometry(&target);

    let mut builder = TransactionBuilder::new();
    let mut unmappings = Vec::new();

    if target_was_on_active_tag {
        let geo = target_geo.expect("Target should have had a geometry");
        window.with_state_mut(|state| state.layout_mode.apply_mode(target_layout_mode));

        state
            .pinnacle
            .configure_window_and_add_map(&mut builder, &window, &output, geo);
    } else if let Some(unmapping) =
        state
            .pinnacle
            .unmap_window(&mut state.backend, &window, &output)
    {
        window.with_state_mut(|state| state.layout_mode.apply_mode(target_layout_mode));
        unmappings.push(unmapping);
    }

    if window_was_on_active_tag {
        let geo = window_geo.expect("Window should have had a geometry");
        target.with_state_mut(|state| state.layout_mode.apply_mode(window_layout_mode));

        state
            .pinnacle
            .configure_window_and_add_map(&mut builder, &target, &output, geo);
    } else if let Some(unmapping) =
        state
            .pinnacle
            .unmap_window(&mut state.backend, &target, &target_output)
    {
        target.with_state_mut(|state| state.layout_mode.apply_mode(window_layout_mode));
        unmappings.push(unmapping);
    }

    // We need one output here. I've picked the one the window was on, although I doubt it matters
    // in this specific case. I guess the alternative would be one TB per output.
    state
        .pinnacle
        .layout_state
        .pending_transactions
        .add_for_output(
            &output,
            builder.into_pending(unmappings, state.pinnacle.layout_state.pending_swap, false),
        );
}
