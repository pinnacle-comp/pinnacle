mod v1;

use std::num::NonZeroU32;

use pinnacle_api_defs::pinnacle::window::{
    self,
    v0alpha1::{FullscreenOrMaximized, WindowRule, WindowRuleCondition},
};
use smithay::{
    desktop::space::SpaceElement,
    reexports::wayland_protocols::xdg::shell::server,
    utils::{Point, SERIAL_COUNTER},
    wayland::seat::WaylandFocus,
};

use crate::{
    output::OutputName,
    state::{State, WithState},
    tag::{Tag, TagId},
    window::{rules::DecorationMode, window_state::WindowState, WindowElement},
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

    // TODO: window.geometry.size or space.elem_geo
    let mut window_size = window.geometry().size;
    window_size.w = w.map(|w| w as i32).unwrap_or(window_size.w);
    window_size.h = h.map(|h| h as i32).unwrap_or(window_size.h);

    window.with_state_mut(|state| {
        state.floating_loc = Some(window_loc.to_f64());
        state.floating_size = Some(window_size);
    });

    state.pinnacle.update_window_state(window);
}

/// Sets a window's fullscreen state.
///
/// If `set` is `None`, this toggles instead.
pub fn set_fullscreen(state: &mut State, window: &WindowElement, set: impl Into<Option<bool>>) {
    let set = set.into();
    match set {
        Some(set) => {
            window.with_state_mut(|state| state.window_state.set_fullscreen(set));
        }
        None => {
            window.with_state_mut(|state| state.window_state.toggle_fullscreen());
        }
    }

    state.update_window_state_and_layout(window);
}

/// Sets a window's maximized state.
///
/// If `set` is `None`, this toggles instead.
pub fn set_maximized(state: &mut State, window: &WindowElement, set: impl Into<Option<bool>>) {
    let set = set.into();
    match set {
        Some(set) => {
            window.with_state_mut(|state| state.window_state.set_maximized(set));
        }
        None => {
            window.with_state_mut(|state| state.window_state.toggle_maximized());
        }
    }

    state.update_window_state_and_layout(window);
}

/// Sets a window's floating state.
///
/// If `set` is `None`, this toggles instead.
pub fn set_floating(state: &mut State, window: &WindowElement, set: impl Into<Option<bool>>) {
    let set = set.into();
    match set {
        Some(set) => {
            window.with_state_mut(|state| state.window_state.set_floating(set));
        }
        None => {
            window.with_state_mut(|state| state.window_state.toggle_floating());
        }
    }

    state.update_window_state_and_layout(window);
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

    // TODO: check if the below is needed
    // state.schedule_render(&output);
}

pub fn move_to_tag(state: &mut State, window: &WindowElement, tag: &Tag) {
    let output = window.output(&state.pinnacle);

    if let Some(output) = output.as_ref() {
        state.capture_snapshots_on_output(output, [window.clone()]);
    }

    window.with_state_mut(|state| {
        state.tags = std::iter::once(tag.clone()).collect();
    });

    let Some(output) = tag.output(&state.pinnacle) else {
        return;
    };

    state.pinnacle.begin_layout_transaction(&output);
    state.pinnacle.request_layout(&output);

    state.schedule_render(&output);

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

// #[tonic::async_trait]
// impl window_service_server::WindowService for WindowService {
//     async fn add_window_rule(
//         &self,
//         request: Request<AddWindowRuleRequest>,
//     ) -> Result<Response<()>, Status> {
//         let request = request.into_inner();
//
//         let cond = request
//             .cond
//             .ok_or_else(|| Status::invalid_argument("no condition specified"))?
//             .into();
//
//         let rule = request
//             .rule
//             .ok_or_else(|| Status::invalid_argument("no rule specified"))?
//             .into();
//
//         run_unary_no_response(&self.sender, move |state| {
//             state.pinnacle.config.window_rules.push((cond, rule));
//         })
//         .await
//     }
// }

impl From<WindowRuleCondition> for crate::window::rules::WindowRuleCondition {
    fn from(cond: WindowRuleCondition) -> Self {
        let cond_any = match cond.any.is_empty() {
            true => None,
            false => Some(
                cond.any
                    .into_iter()
                    .map(crate::window::rules::WindowRuleCondition::from)
                    .collect::<Vec<_>>(),
            ),
        };

        let cond_all = match cond.all.is_empty() {
            true => None,
            false => Some(
                cond.all
                    .into_iter()
                    .map(crate::window::rules::WindowRuleCondition::from)
                    .collect::<Vec<_>>(),
            ),
        };

        let class = match cond.classes.is_empty() {
            true => None,
            false => Some(cond.classes),
        };

        let title = match cond.titles.is_empty() {
            true => None,
            false => Some(cond.titles),
        };

        let tag = match cond.tags.is_empty() {
            true => None,
            false => Some(cond.tags.into_iter().map(TagId::new).collect::<Vec<_>>()),
        };

        crate::window::rules::WindowRuleCondition {
            cond_any,
            cond_all,
            class,
            title,
            tag,
        }
    }
}

impl From<WindowRule> for crate::window::rules::WindowRule {
    fn from(rule: WindowRule) -> Self {
        let fullscreen_or_maximized = match rule.fullscreen_or_maximized() {
            FullscreenOrMaximized::Unspecified => None,
            FullscreenOrMaximized::Neither => {
                Some(crate::window::window_state::FullscreenOrMaximized::Neither)
            }
            FullscreenOrMaximized::Fullscreen => {
                Some(crate::window::window_state::FullscreenOrMaximized::Fullscreen)
            }
            FullscreenOrMaximized::Maximized => {
                Some(crate::window::window_state::FullscreenOrMaximized::Maximized)
            }
        };

        let window_state = match rule.state() {
            window::v0alpha1::WindowState::Unspecified => None,
            window::v0alpha1::WindowState::Tiled => Some(WindowState::Tiled),
            window::v0alpha1::WindowState::Floating => Some(WindowState::Floating),
            window::v0alpha1::WindowState::Fullscreen => Some(WindowState::Fullscreen {
                previous_state: crate::window::window_state::FloatingOrTiled::Tiled,
            }),
            window::v0alpha1::WindowState::Maximized => Some(WindowState::Fullscreen {
                previous_state: crate::window::window_state::FloatingOrTiled::Tiled,
            }),
        };

        let output = rule.output.map(OutputName);
        let tags = match rule.tags.is_empty() {
            true => None,
            false => Some(rule.tags.into_iter().map(TagId::new).collect::<Vec<_>>()),
        };
        let floating_or_tiled = rule.floating.map(|floating| match floating {
            true => crate::window::window_state::FloatingOrTiled::Floating,
            false => crate::window::window_state::FloatingOrTiled::Tiled,
        });
        let size = rule.width.and_then(|w| {
            rule.height.and_then(|h| {
                Some((
                    NonZeroU32::try_from(w as u32).ok()?,
                    NonZeroU32::try_from(h as u32).ok()?,
                ))
            })
        });
        let location = rule.x.and_then(|x| rule.y.map(|y| (x, y)));
        let decoration_mode = rule.ssd.map(|ssd| match ssd {
            true => DecorationMode::ServerSide,
            false => DecorationMode::ClientSide,
        });

        crate::window::rules::WindowRule {
            output,
            tags,
            floating_or_tiled,
            fullscreen_or_maximized,
            size,
            location,
            decoration_mode,
            window_state,
        }
    }
}
