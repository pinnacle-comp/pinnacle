mod v1;

use smithay::{
    desktop::space::SpaceElement,
    reexports::{
        wayland_protocols::xdg::{
            decoration::zv1::server::zxdg_toplevel_decoration_v1, shell::server,
        },
        wayland_protocols_misc::server_decoration::server::org_kde_kwin_server_decoration,
    },
    utils::{Point, SERIAL_COUNTER},
    wayland::{compositor, seat::WaylandFocus},
};

use crate::{
    handlers::decoration::KdeDecorationObject,
    state::{State, WithState},
    tag::Tag,
    window::{rules::DecorationMode, WindowElement},
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

    // TODO: with no x or y, defaults unmapped windows to 0, 0
    // FIXME: space stores loc in i32 not f64
    let mut window_loc = state
        .pinnacle
        .space
        .element_location(window)
        .unwrap_or_default();
    window_loc.x = x.unwrap_or(window_loc.x);
    window_loc.y = y.unwrap_or(window_loc.y);

    let mut window_size = window.geometry().size;
    window_size.w = w.map(|w| w as i32).unwrap_or(window_size.w);
    window_size.h = h.map(|h| h as i32).unwrap_or(window_size.h);

    window.with_state_mut(|state| {
        state.floating_loc = Some(window_loc.to_f64());
        state.floating_size = window_size;
    });

    state.pinnacle.configure_window_if_nontiled(window);
    if let Some(toplevel) = window.toplevel() {
        toplevel.send_pending_configure();
    }
}

// TODO: minimized

pub fn set_focused(state: &mut State, window: &WindowElement, set: impl Into<Option<bool>>) {
    if window.is_x11_override_redirect() {
        return;
    }

    let Some(output) = window.output(&state.pinnacle) else {
        return;
    };

    let set = set.into();

    let is_focused = state.pinnacle.focused_window(&output).as_ref() == Some(window);

    let set = match set {
        Some(set) => set,
        None => !is_focused,
    };

    for win in state.pinnacle.space.elements() {
        win.set_activate(false);
    }

    if set {
        window.set_activate(true);
        output.with_state_mut(|state| state.focus_stack.set_focus(window.clone()));
        state.pinnacle.output_focus_stack.set_focus(output.clone());
        state.update_keyboard_focus(&output);
    } else {
        output.with_state_mut(|state| state.focus_stack.unset_focus());
        if let Some(keyboard) = state.pinnacle.seat.get_keyboard() {
            keyboard.set_focus(state, None, SERIAL_COUNTER.next_serial());
        }
    }

    for window in state.pinnacle.space.elements() {
        if let Some(toplevel) = window.toplevel() {
            toplevel.send_pending_configure();
        }
    }
}

pub fn set_decoration_mode(
    _state: &mut State,
    window: &WindowElement,
    decoration_mode: DecorationMode,
) {
    window.with_state_mut(|state| {
        state.decoration_mode = Some(decoration_mode);
    });
    if let Some(toplevel) = window.toplevel() {
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(match decoration_mode {
                DecorationMode::ClientSide => zxdg_toplevel_decoration_v1::Mode::ClientSide,
                DecorationMode::ServerSide => zxdg_toplevel_decoration_v1::Mode::ServerSide,
            })
        });

        compositor::with_states(toplevel.wl_surface(), |states| {
            let kde_decoration = states.data_map.get::<KdeDecorationObject>();
            if let Some(kde_decoration) = kde_decoration {
                if let Some(object) = kde_decoration
                    .borrow()
                    .as_ref()
                    .and_then(|obj| obj.upgrade().ok())
                {
                    let mode = match decoration_mode {
                        DecorationMode::ClientSide => org_kde_kwin_server_decoration::Mode::Client,
                        DecorationMode::ServerSide => org_kde_kwin_server_decoration::Mode::Server,
                    };
                    tracing::debug!(?mode, "Window rule set KDE decoration mode");
                    object.mode(mode);
                }
            }
        });

        toplevel.send_pending_configure();
    }
}

pub fn move_to_tag(state: &mut State, window: &WindowElement, tag: &Tag) {
    let source_output = window.output(&state.pinnacle);

    if let Some(output) = source_output.as_ref() {
        state.capture_snapshots_on_output(output, [window.clone()]);
    }

    window.with_state_mut(|state| {
        state.tags = std::iter::once(tag.clone()).collect();
    });

    if let Some(output) = source_output.as_ref() {
        state.pinnacle.begin_layout_transaction(output);
        state.pinnacle.request_layout(output);

        state.schedule_render(output);
    }

    let Some(target_output) = tag.output(&state.pinnacle) else {
        state.pinnacle.update_xwayland_stacking_order();
        return;
    };

    if source_output.as_ref() != Some(&target_output) && tag.active() {
        state.capture_snapshots_on_output(&target_output, [window.clone()]);

        state.pinnacle.begin_layout_transaction(&target_output);
        state.pinnacle.request_layout(&target_output);

        state.schedule_render(&target_output);
    }

    state.pinnacle.update_xwayland_stacking_order();
}

pub fn set_tag(state: &mut State, window: &WindowElement, tag: &Tag, set: impl Into<Option<bool>>) {
    let set = set.into();

    let output = window.output(&state.pinnacle);

    if let Some(output) = output.as_ref() {
        state.capture_snapshots_on_output(output, [window.clone()]);
    }

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

    state.pinnacle.begin_layout_transaction(&output);
    state.pinnacle.request_layout(&output);

    state.schedule_render(&output);

    state.pinnacle.update_xwayland_stacking_order();
}

pub fn raise(state: &mut State, window: WindowElement) {
    for output in state.pinnacle.space.outputs_for_element(&window) {
        state.schedule_render(&output);
    }

    state.pinnacle.raise_window(window, false);
}

pub fn move_grab(state: &mut State, button: u32) {
    let Some(pointer_location) = state
        .pinnacle
        .seat
        .get_pointer()
        .map(|ptr| ptr.current_location())
    else {
        return;
    };
    let Some((pointer_focus, _)) = state.pinnacle.pointer_focus_target_under(pointer_location)
    else {
        return;
    };
    let Some(window) = pointer_focus.window_for(state) else {
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
    let Some((pointer_focus, window_loc)) = state.pinnacle.pointer_focus_target_under(pointer_loc)
    else {
        return;
    };
    let Some(window) = pointer_focus.window_for(state) else {
        tracing::info!("Move grabs are currently not implemented for non-windows");
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
