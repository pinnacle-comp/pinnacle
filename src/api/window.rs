mod v1;

use smithay::{
    desktop::space::SpaceElement,
    reexports::wayland_protocols::xdg::{
        decoration::zv1::server::zxdg_toplevel_decoration_v1, shell::server,
    },
    utils::{Point, SERIAL_COUNTER},
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
            state.pinnacle.output_focus_stack.set_focus(output.clone());
        } else {
            let currently_focused_op = state.pinnacle.focused_output();
            match currently_focused_op {
                Some(op) => {
                    if !window_outputs.contains(&op) {
                        state.pinnacle.output_focus_stack.set_focus(output.clone());
                    }
                }
                None => {
                    state.pinnacle.output_focus_stack.set_focus(output.clone());
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
    let Some((pointer_focus, window_loc)) = state.pinnacle.pointer_contents.focus_under.as_ref()
    else {
        return;
    };
    let Some(window) = pointer_focus.window_for(&state.pinnacle) else {
        return;
    };
    let Some(wl_surf) = window.wl_surface() else {
        return;
    };

    let window_geometry = window.geometry();
    let window_x = window_loc.x;
    let window_y = window_loc.y;
    let window_width = window_geometry.size.w as f64;
    let window_height = window_geometry.size.h as f64;
    let half_width = window_x + window_width / 2.0;
    let half_height = window_y + window_height / 2.0;
    let full_width = window_x + window_width;
    let full_height = window_y + window_height;

    let edges = match pointer_loc {
        Point { x, y, .. }
            if (window_x..=half_width).contains(&x) && (window_y..=half_height).contains(&y) =>
        {
            server::xdg_toplevel::ResizeEdge::TopLeft
        }
        Point { x, y, .. }
            if (half_width..=full_width).contains(&x) && (window_y..=half_height).contains(&y) =>
        {
            server::xdg_toplevel::ResizeEdge::TopRight
        }
        Point { x, y, .. }
            if (window_x..=half_width).contains(&x) && (half_height..=full_height).contains(&y) =>
        {
            server::xdg_toplevel::ResizeEdge::BottomLeft
        }
        Point { x, y, .. }
            if (half_width..=full_width).contains(&x)
                && (half_height..=full_height).contains(&y) =>
        {
            server::xdg_toplevel::ResizeEdge::BottomRight
        }
        _ => server::xdg_toplevel::ResizeEdge::None,
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
