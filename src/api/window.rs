use std::num::NonZeroU32;

use pinnacle_api_defs::pinnacle::{
    v0alpha1::{Geometry, SetOrToggle},
    window::{
        self,
        v0alpha1::{
            window_service_server, AddWindowRuleRequest, CloseRequest, FullscreenOrMaximized,
            MoveGrabRequest, MoveToTagRequest, RaiseRequest, ResizeGrabRequest, SetFloatingRequest,
            SetFocusedRequest, SetFullscreenRequest, SetGeometryRequest, SetMaximizedRequest,
            SetTagRequest, WindowRule, WindowRuleCondition,
        },
    },
};
use smithay::{
    desktop::space::SpaceElement,
    reexports::wayland_protocols::xdg::shell::server,
    utils::{Point, SERIAL_COUNTER},
    wayland::seat::WaylandFocus,
};
use tonic::{Request, Response, Status};
use tracing::warn;

use crate::{
    output::OutputName,
    state::WithState,
    tag::TagId,
    window::{
        rules::DecorationMode,
        window_state::{WindowId, WindowState},
    },
};

use super::{run_unary, run_unary_no_response, StateFnSender};

pub struct WindowService {
    sender: StateFnSender,
}

impl WindowService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}

#[tonic::async_trait]
impl window_service_server::WindowService for WindowService {
    async fn close(&self, request: Request<CloseRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let window_id = WindowId(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        run_unary_no_response(&self.sender, move |state| {
            let Some(window) = window_id.window(&state.pinnacle) else {
                return;
            };

            window.close();
        })
        .await
    }

    async fn set_geometry(
        &self,
        request: Request<SetGeometryRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let window_id = WindowId(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        let geometry = request.geometry.unwrap_or_default();
        let x = geometry.x;
        let y = geometry.y;
        let width = geometry.width;
        let height = geometry.height;

        run_unary_no_response(&self.sender, move |state| {
            let Some(window) = window_id.window(&state.pinnacle) else {
                return;
            };

            // TODO: with no x or y, defaults unmapped windows to 0, 0
            // FIXME: space stores loc in i32 not f64
            let mut window_loc = state
                .pinnacle
                .space
                .element_location(&window)
                .unwrap_or_default();
            window_loc.x = x.unwrap_or(window_loc.x);
            window_loc.y = y.unwrap_or(window_loc.y);

            // TODO: window.geometry.size or space.elem_geo
            let mut window_size = window.geometry().size;
            window_size.w = width.unwrap_or(window_size.w);
            window_size.h = height.unwrap_or(window_size.h);

            window.with_state_mut(|state| {
                state.floating_loc = Some(window_loc.to_f64());
                state.floating_size = Some(window_size);
            });

            state.pinnacle.update_window_state(&window);
        })
        .await
    }

    async fn set_fullscreen(
        &self,
        request: Request<SetFullscreenRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let window_id = WindowId(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        let set_or_toggle = request.set_or_toggle();

        if set_or_toggle == SetOrToggle::Unspecified {
            return Err(Status::invalid_argument("unspecified set or toggle"));
        }

        let fullscreen = match set_or_toggle {
            SetOrToggle::Set => Some(true),
            SetOrToggle::Unset => Some(false),
            SetOrToggle::Toggle => None,
            SetOrToggle::Unspecified => unreachable!(),
        };

        run_unary_no_response(&self.sender, move |state| {
            let Some(window) = window_id.window(&state.pinnacle) else {
                return;
            };

            match fullscreen {
                Some(fullscreen) => {
                    window.with_state_mut(|state| state.window_state.set_fullscreen(fullscreen))
                }
                None => window.with_state_mut(|state| state.window_state.toggle_fullscreen()),
            }

            state.update_window_state_and_layout(&window);
        })
        .await
    }

    async fn set_maximized(
        &self,
        request: Request<SetMaximizedRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let window_id = WindowId(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        let set_or_toggle = request.set_or_toggle();

        if set_or_toggle == SetOrToggle::Unspecified {
            return Err(Status::invalid_argument("unspecified set or toggle"));
        }

        let maximized = match set_or_toggle {
            SetOrToggle::Set => Some(true),
            SetOrToggle::Unset => Some(false),
            SetOrToggle::Toggle => None,
            SetOrToggle::Unspecified => unreachable!(),
        };

        run_unary_no_response(&self.sender, move |state| {
            let Some(window) = window_id.window(&state.pinnacle) else {
                return;
            };

            match maximized {
                Some(maximized) => {
                    window.with_state_mut(|state| state.window_state.set_maximized(maximized))
                }
                None => window.with_state_mut(|state| state.window_state.toggle_maximized()),
            }

            state.update_window_state_and_layout(&window);
        })
        .await
    }

    async fn set_floating(
        &self,
        request: Request<SetFloatingRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let window_id = WindowId(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        let set_or_toggle = request.set_or_toggle();

        if set_or_toggle == SetOrToggle::Unspecified {
            return Err(Status::invalid_argument("unspecified set or toggle"));
        }

        let floating = match set_or_toggle {
            SetOrToggle::Unspecified => unreachable!(),
            SetOrToggle::Set => Some(true),
            SetOrToggle::Unset => Some(false),
            SetOrToggle::Toggle => None,
        };

        run_unary_no_response(&self.sender, move |state| {
            let Some(window) = window_id.window(&state.pinnacle) else {
                return;
            };

            match floating {
                Some(floating) => {
                    window.with_state_mut(|state| state.window_state.set_floating(floating))
                }
                None => window.with_state_mut(|state| state.window_state.toggle_floating()),
            }

            state.update_window_state_and_layout(&window);
        })
        .await
    }

    async fn set_focused(
        &self,
        request: Request<SetFocusedRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let window_id = WindowId(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        let set_or_toggle = request.set_or_toggle();

        if set_or_toggle == SetOrToggle::Unspecified {
            return Err(Status::invalid_argument("unspecified set or toggle"));
        }

        run_unary_no_response(&self.sender, move |state| {
            let Some(window) = window_id.window(&state.pinnacle) else {
                return;
            };

            if window.is_x11_override_redirect() {
                return;
            }

            let Some(output) = window.output(&state.pinnacle) else {
                return;
            };

            for win in state.pinnacle.space.elements() {
                win.set_activate(false);
            }

            match set_or_toggle {
                SetOrToggle::Set => {
                    window.set_activate(true);
                    output.with_state_mut(|state| state.focus_stack.set_focus(window.clone()));
                    state.pinnacle.output_focus_stack.set_focus(output.clone());
                    state.update_keyboard_focus(&output);
                }
                SetOrToggle::Unset => {
                    if state.pinnacle.focused_window(&output) == Some(window) {
                        output.with_state_mut(|state| state.focus_stack.unset_focus());
                        if let Some(keyboard) = state.pinnacle.seat.get_keyboard() {
                            keyboard.set_focus(state, None, SERIAL_COUNTER.next_serial());
                        }
                    }
                }
                SetOrToggle::Toggle => {
                    if state.pinnacle.focused_window(&output).as_ref() == Some(&window) {
                        output.with_state_mut(|state| state.focus_stack.unset_focus());
                        if let Some(keyboard) = state.pinnacle.seat.get_keyboard() {
                            keyboard.set_focus(state, None, SERIAL_COUNTER.next_serial());
                        }
                    } else {
                        window.set_activate(true);
                        output.with_state_mut(|state| state.focus_stack.set_focus(window.clone()));
                        state.pinnacle.output_focus_stack.set_focus(output.clone());
                        state.update_keyboard_focus(&output);
                    }
                }
                SetOrToggle::Unspecified => unreachable!(),
            }

            for window in state.pinnacle.space.elements() {
                if let Some(toplevel) = window.toplevel() {
                    toplevel.send_configure();
                }
            }

            state.schedule_render(&output);
        })
        .await
    }

    async fn move_to_tag(
        &self,
        request: Request<MoveToTagRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let window_id = WindowId(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        let tag_id = TagId(
            request
                .tag_id
                .ok_or_else(|| Status::invalid_argument("no tag specified"))?,
        );

        run_unary_no_response(&self.sender, move |state| {
            let Some(window) = window_id.window(&state.pinnacle) else {
                return;
            };

            let Some(tag) = tag_id.tag(&state.pinnacle) else { return };

            let output = window.output(&state.pinnacle);

            if let Some(output) = output.as_ref() {
                state.capture_snapshots_on_output(output, [window.clone()]);
            }

            window.with_state_mut(|state| {
                state.tags = vec![tag.clone()];
            });

            let Some(output) = tag.output(&state.pinnacle) else {
                return;
            };

            state.pinnacle.begin_layout_transaction(&output);
            state.pinnacle.request_layout(&output);

            state.schedule_render(&output);

            state.pinnacle.fixup_xwayland_window_layering();
        })
        .await
    }

    async fn set_tag(&self, request: Request<SetTagRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let window_id = WindowId(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        let tag_id = TagId(
            request
                .tag_id
                .ok_or_else(|| Status::invalid_argument("no tag specified"))?,
        );

        let set_or_toggle = request.set_or_toggle();

        if set_or_toggle == SetOrToggle::Unspecified {
            return Err(Status::invalid_argument("unspecified set or toggle"));
        }

        run_unary_no_response(&self.sender, move |state| {
            let Some(window) = window_id.window(&state.pinnacle) else {
                return;
            };
            let Some(tag) = tag_id.tag(&state.pinnacle) else { return };

            let output = window.output(&state.pinnacle);

            if let Some(output) = output.as_ref() {
                state.capture_snapshots_on_output(output, [window.clone()]);
            }

            // TODO: turn state.tags into a hashset
            match set_or_toggle {
                SetOrToggle::Set => window.with_state_mut(|state| {
                    state.tags.retain(|tg| tg != &tag);
                    state.tags.push(tag.clone());
                }),
                SetOrToggle::Unset => window.with_state_mut(|state| {
                    state.tags.retain(|tg| tg != &tag);
                }),
                SetOrToggle::Toggle => window.with_state_mut(|state| {
                    if !state.tags.contains(&tag) {
                        state.tags.push(tag.clone());
                    } else {
                        state.tags.retain(|tg| tg != &tag);
                    }
                }),
                SetOrToggle::Unspecified => unreachable!(),
            }

            let Some(output) = tag.output(&state.pinnacle) else {
                return;
            };

            state.pinnacle.begin_layout_transaction(&output);
            state.pinnacle.request_layout(&output);

            state.schedule_render(&output);

            state.pinnacle.fixup_xwayland_window_layering();
        })
        .await
    }

    async fn raise(&self, request: Request<RaiseRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let window_id = WindowId(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        run_unary_no_response(&self.sender, move |state| {
            let pinnacle = &mut state.pinnacle;
            let Some(window) = window_id.window(pinnacle) else {
                warn!("`raise` was called on a nonexistent window");
                return;
            };

            pinnacle.raise_window(window, false);
        })
        .await
    }

    async fn move_grab(&self, request: Request<MoveGrabRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let button = request
            .button
            .ok_or_else(|| Status::invalid_argument("no button specified"))?;

        run_unary_no_response(&self.sender, move |state| {
            let Some(pointer_location) = state
                .pinnacle
                .seat
                .get_pointer()
                .map(|ptr| ptr.current_location())
            else {
                return;
            };
            let Some((pointer_focus, _)) =
                state.pinnacle.pointer_focus_target_under(pointer_location)
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
        })
        .await
    }

    async fn resize_grab(
        &self,
        request: Request<ResizeGrabRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let button = request
            .button
            .ok_or_else(|| Status::invalid_argument("no button specified"))?;

        run_unary_no_response(&self.sender, move |state| {
            let Some(pointer_loc) = state
                .pinnacle
                .seat
                .get_pointer()
                .map(|ptr| ptr.current_location())
            else {
                return;
            };
            let Some((pointer_focus, window_loc)) =
                state.pinnacle.pointer_focus_target_under(pointer_loc)
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
                    if (window_x..=half_width).contains(&x)
                        && (window_y..=half_height).contains(&y) =>
                {
                    server::xdg_toplevel::ResizeEdge::TopLeft
                }
                Point { x, y, .. }
                    if (half_width..=full_width).contains(&x)
                        && (window_y..=half_height).contains(&y) =>
                {
                    server::xdg_toplevel::ResizeEdge::TopRight
                }
                Point { x, y, .. }
                    if (window_x..=half_width).contains(&x)
                        && (half_height..=full_height).contains(&y) =>
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
        })
        .await
    }

    async fn get(
        &self,
        _request: Request<window::v0alpha1::GetRequest>,
    ) -> Result<Response<window::v0alpha1::GetResponse>, Status> {
        run_unary(&self.sender, move |state| {
            let window_ids = state
                .pinnacle
                .windows
                .iter()
                .map(|win| win.with_state(|state| state.id.0))
                .collect::<Vec<_>>();

            window::v0alpha1::GetResponse { window_ids }
        })
        .await
    }

    async fn get_properties(
        &self,
        request: Request<window::v0alpha1::GetPropertiesRequest>,
    ) -> Result<Response<window::v0alpha1::GetPropertiesResponse>, Status> {
        let request = request.into_inner();

        let window_id = WindowId(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        run_unary(&self.sender, move |state| {
            let pinnacle = &state.pinnacle;
            let window = window_id.window(pinnacle);

            let width = window.as_ref().map(|win| win.geometry().size.w);

            let height = window.as_ref().map(|win| win.geometry().size.h);

            let x = window
                .as_ref()
                .and_then(|win| state.pinnacle.space.element_location(win))
                .map(|loc| loc.x);

            let y = window
                .as_ref()
                .and_then(|win| state.pinnacle.space.element_location(win))
                .map(|loc| loc.y);

            let geometry = if width.is_none() && height.is_none() && x.is_none() && y.is_none() {
                None
            } else {
                Some(Geometry {
                    x,
                    y,
                    width,
                    height,
                })
            };

            let class = window.as_ref().and_then(|win| win.class());
            let title = window.as_ref().and_then(|win| win.title());

            let focused = window.as_ref().and_then(|win| {
                pinnacle
                    .focused_output()
                    .and_then(|output| pinnacle.focused_window(output))
                    .map(|foc_win| win == foc_win)
            });

            let floating = window
                .as_ref()
                .map(|win| win.with_state(|state| state.window_state.is_floating()));

            // TODO: change representation
            let fullscreen_or_maximized = window
                .as_ref()
                .map(|win| win.with_state(|state| state.window_state))
                .map(|fs_or_max| match fs_or_max {
                    // TODO: from impl
                    WindowState::Tiled | WindowState::Floating => FullscreenOrMaximized::Neither,
                    WindowState::Fullscreen { .. } => FullscreenOrMaximized::Fullscreen,
                    WindowState::Maximized { .. } => FullscreenOrMaximized::Maximized,
                } as i32);

            let tag_ids = window
                .as_ref()
                .map(|win| {
                    win.with_state(|state| {
                        state.tags.iter().map(|tag| tag.id().0).collect::<Vec<_>>()
                    })
                })
                .unwrap_or_default();

            let state = window.as_ref().map(|win| {
                let state = win.with_state(|state| state.window_state);
                (match state {
                    WindowState::Tiled => window::v0alpha1::WindowState::Tiled,
                    WindowState::Floating => window::v0alpha1::WindowState::Floating,
                    WindowState::Maximized { .. } => window::v0alpha1::WindowState::Maximized,
                    WindowState::Fullscreen { .. } => window::v0alpha1::WindowState::Fullscreen,
                }) as i32
            });

            #[allow(deprecated)]
            window::v0alpha1::GetPropertiesResponse {
                geometry,
                class,
                title,
                focused,
                floating,
                fullscreen_or_maximized,
                tag_ids,
                state,
            }
        })
        .await
    }

    async fn add_window_rule(
        &self,
        request: Request<AddWindowRuleRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let cond = request
            .cond
            .ok_or_else(|| Status::invalid_argument("no condition specified"))?
            .into();

        let rule = request
            .rule
            .ok_or_else(|| Status::invalid_argument("no rule specified"))?
            .into();

        run_unary_no_response(&self.sender, move |state| {
            state.pinnacle.config.window_rules.push((cond, rule));
        })
        .await
    }
}

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
            false => Some(cond.tags.into_iter().map(TagId).collect::<Vec<_>>()),
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
            false => Some(rule.tags.into_iter().map(TagId).collect::<Vec<_>>()),
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
