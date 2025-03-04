use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use pinnacle_api_defs::pinnacle::{
    util::{self, v1::SetOrToggle},
    window::{
        self,
        v1::{
            self, CloseRequest, GetAppIdRequest, GetAppIdResponse, GetFocusedRequest,
            GetFocusedResponse, GetLayoutModeRequest, GetLayoutModeResponse, GetLocRequest,
            GetLocResponse, GetRequest, GetResponse, GetSizeRequest, GetSizeResponse,
            GetTagIdsRequest, GetTagIdsResponse, GetTitleRequest, GetTitleResponse,
            MoveGrabRequest, MoveToTagRequest, RaiseRequest, ResizeGrabRequest,
            SetDecorationModeRequest, SetFloatingRequest, SetFocusedRequest, SetFullscreenRequest,
            SetGeometryRequest, SetMaximizedRequest, SetTagRequest, WindowRuleRequest,
            WindowRuleResponse,
        },
    },
};
use smithay::{
    desktop::WindowSurface,
    utils::{Point, Size},
};
use tonic::{Request, Status, Streaming};
use tracing::error;

use crate::{
    api::{
        run_bidirectional_streaming_mapped, run_unary, run_unary_no_response, ResponseStream,
        TonicResult,
    },
    state::WithState,
    tag::TagId,
    window::window_state::{LayoutMode, LayoutModeKind, WindowId},
};

#[tonic::async_trait]
impl v1::window_service_server::WindowService for super::WindowService {
    type WindowRuleStream = ResponseStream<WindowRuleResponse>;

    async fn get(&self, _request: Request<GetRequest>) -> TonicResult<GetResponse> {
        run_unary(&self.sender, move |state| {
            let window_ids = state
                .pinnacle
                .windows
                .iter()
                .map(|win| win.with_state(|state| state.id.0))
                .collect::<Vec<_>>();

            Ok(GetResponse { window_ids })
        })
        .await
    }

    async fn get_app_id(&self, request: Request<GetAppIdRequest>) -> TonicResult<GetAppIdResponse> {
        let window_id = WindowId(request.into_inner().window_id);

        run_unary(&self.sender, move |state| {
            let app_id = window_id
                .window(&state.pinnacle)
                .or_else(|| {
                    window_id
                        .unmapped_window(&state.pinnacle)
                        .map(|unmapped| unmapped.window.clone())
                })
                .and_then(|win| win.class())
                .unwrap_or_default();

            Ok(GetAppIdResponse { app_id })
        })
        .await
    }

    async fn get_title(&self, request: Request<GetTitleRequest>) -> TonicResult<GetTitleResponse> {
        let window_id = WindowId(request.into_inner().window_id);

        run_unary(&self.sender, move |state| {
            let title = window_id
                .window(&state.pinnacle)
                .or_else(|| {
                    window_id
                        .unmapped_window(&state.pinnacle)
                        .map(|unmapped| unmapped.window.clone())
                })
                .and_then(|win| win.title())
                .unwrap_or_default();

            Ok(GetTitleResponse { title })
        })
        .await
    }

    async fn get_loc(&self, request: Request<GetLocRequest>) -> TonicResult<GetLocResponse> {
        let window_id = WindowId(request.into_inner().window_id);

        run_unary(&self.sender, move |state| {
            let loc = window_id
                .window(&state.pinnacle)
                .and_then(|win| state.pinnacle.space.element_location(&win));

            Ok(GetLocResponse {
                loc: loc.map(|loc| util::v1::Point { x: loc.x, y: loc.y }),
            })
        })
        .await
    }

    async fn get_size(&self, request: Request<GetSizeRequest>) -> TonicResult<GetSizeResponse> {
        let window_id = WindowId(request.into_inner().window_id);

        run_unary(&self.sender, move |state| {
            let size = window_id
                .window(&state.pinnacle)
                .map(|win| win.geometry().size);

            Ok(GetSizeResponse {
                size: size.map(|size| util::v1::Size {
                    width: size.w.try_into().unwrap_or_default(),
                    height: size.h.try_into().unwrap_or_default(),
                }),
            })
        })
        .await
    }

    async fn get_focused(
        &self,
        request: Request<GetFocusedRequest>,
    ) -> TonicResult<GetFocusedResponse> {
        let window_id = WindowId(request.into_inner().window_id);

        run_unary(&self.sender, move |state| {
            let focused = window_id
                .window(&state.pinnacle)
                .and_then(|win| {
                    let focused = state
                        .pinnacle
                        .focused_window(state.pinnacle.focused_output()?)?
                        == win;
                    Some(focused)
                })
                .unwrap_or_default();

            Ok(GetFocusedResponse { focused })
        })
        .await
    }

    async fn get_layout_mode(
        &self,
        request: Request<GetLayoutModeRequest>,
    ) -> TonicResult<GetLayoutModeResponse> {
        let window_id = WindowId(request.into_inner().window_id);

        run_unary(&self.sender, move |state| {
            let layout_mode = window_id
                .window(&state.pinnacle)
                .map(|win| win.with_state(|state| state.layout_mode))
                .unwrap_or(LayoutMode::tiled());

            Ok(GetLayoutModeResponse {
                layout_mode: match layout_mode.current() {
                    LayoutModeKind::Tiled => window::v1::LayoutMode::Tiled,
                    LayoutModeKind::Floating => window::v1::LayoutMode::Floating,
                    LayoutModeKind::Maximized => window::v1::LayoutMode::Maximized,
                    LayoutModeKind::Fullscreen => window::v1::LayoutMode::Fullscreen,
                }
                .into(),
            })
        })
        .await
    }

    async fn get_tag_ids(
        &self,
        request: Request<GetTagIdsRequest>,
    ) -> TonicResult<GetTagIdsResponse> {
        let window_id = WindowId(request.into_inner().window_id);

        run_unary(&self.sender, move |state| {
            let tag_ids = window_id
                .window(&state.pinnacle)
                .map(|win| {
                    win.with_state(|state| {
                        state
                            .tags
                            .iter()
                            .map(|tag| tag.id().to_inner())
                            .collect::<Vec<_>>()
                    })
                })
                .unwrap_or_default();

            Ok(GetTagIdsResponse { tag_ids })
        })
        .await
    }

    async fn close(&self, request: Request<CloseRequest>) -> TonicResult<()> {
        let request = request.into_inner();

        let window_id = WindowId(request.window_id);

        run_unary_no_response(&self.sender, move |state| {
            let Some(window) = window_id.window(&state.pinnacle) else {
                println!("window doesn't exist");
                return;
            };

            window.close();
        })
        .await
    }

    async fn set_geometry(&self, request: Request<SetGeometryRequest>) -> TonicResult<()> {
        let request = request.into_inner();

        let window_id = WindowId(request.window_id);

        let x = request.x;
        let y = request.y;
        let w = request.w;
        let h = request.h;

        run_unary_no_response(&self.sender, move |state| {
            if let Some(window) = window_id.window(&state.pinnacle) {
                crate::api::window::set_geometry(state, &window, x, y, w, h);
            } else if let Some(unmapped) = window_id.unmapped_window_mut(&mut state.pinnacle) {
                let loc = if x.is_some() || y.is_some() {
                    // FIXME: Only specifying one of x or y will cause the other to become
                    // zero, maybe split up the point into two options
                    Some(Point::from((x.unwrap_or_default(), y.unwrap_or_default())))
                } else {
                    None
                };

                let size = Size::from((w.unwrap_or_default() as i32, h.unwrap_or_default() as i32));

                unmapped.window_rules.floating_loc = loc.map(|loc| loc.to_f64());
                unmapped.window_rules.floating_size = Some(size);
            }
        })
        .await
    }

    async fn set_fullscreen(&self, request: Request<SetFullscreenRequest>) -> TonicResult<()> {
        let request = request.into_inner();

        let window_id = WindowId(request.window_id);

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
            if let Some(window) = window_id.window(&state.pinnacle) {
                match fullscreen {
                    Some(set) => {
                        window.with_state_mut(|state| state.layout_mode.set_fullscreen(set));
                    }
                    None => {
                        window.with_state_mut(|state| state.layout_mode.toggle_fullscreen());
                    }
                }
                state.update_window_state_and_layout(&window);
            } else if let Some(unmapped) = window_id.unmapped_window_mut(&mut state.pinnacle) {
                match fullscreen {
                    Some(true) => {
                        unmapped
                            .window_rules
                            .layout_mode
                            .get_or_insert(LayoutMode::fullscreen())
                            .set_fullscreen(true);
                    }
                    Some(false) => {
                        if let Some(layout_mode) = unmapped.window_rules.layout_mode.as_mut() {
                            layout_mode.set_fullscreen(false);
                        }
                    }
                    None => {
                        unmapped
                            .window_rules
                            .layout_mode
                            .get_or_insert(LayoutMode::tiled())
                            .toggle_fullscreen();
                    }
                }
            }
        })
        .await
    }

    async fn set_maximized(&self, request: Request<SetMaximizedRequest>) -> TonicResult<()> {
        let request = request.into_inner();

        let window_id = WindowId(request.window_id);

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
            if let Some(window) = window_id.window(&state.pinnacle) {
                match maximized {
                    Some(set) => {
                        window.with_state_mut(|state| state.layout_mode.set_maximized(set));
                    }
                    None => {
                        window.with_state_mut(|state| state.layout_mode.toggle_maximized());
                    }
                }
                state.update_window_state_and_layout(&window);
            } else if let Some(unmapped) = window_id.unmapped_window_mut(&mut state.pinnacle) {
                match maximized {
                    Some(true) => {
                        unmapped
                            .window_rules
                            .layout_mode
                            .get_or_insert(LayoutMode::maximized())
                            .set_maximized(true);
                    }
                    Some(false) => {
                        if let Some(layout_mode) = unmapped.window_rules.layout_mode.as_mut() {
                            layout_mode.set_maximized(false);
                        }
                    }
                    None => {
                        unmapped
                            .window_rules
                            .layout_mode
                            .get_or_insert(LayoutMode::tiled())
                            .toggle_maximized();
                    }
                }
            }
        })
        .await
    }

    async fn set_floating(&self, request: Request<SetFloatingRequest>) -> TonicResult<()> {
        let request = request.into_inner();

        let window_id = WindowId(request.window_id);

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
            if let Some(window) = window_id.window(&state.pinnacle) {
                match floating {
                    Some(set) => {
                        window.with_state_mut(|state| state.layout_mode.set_floating(set));
                    }
                    None => {
                        window.with_state_mut(|state| state.layout_mode.toggle_floating());
                    }
                }
                state.update_window_state_and_layout(&window);
            } else if let Some(unmapped) = window_id.unmapped_window_mut(&mut state.pinnacle) {
                match floating {
                    Some(true) => {
                        unmapped
                            .window_rules
                            .layout_mode
                            .get_or_insert(LayoutMode::floating())
                            .set_floating(true);
                    }
                    Some(false) => {
                        unmapped
                            .window_rules
                            .layout_mode
                            .get_or_insert(LayoutMode::floating())
                            .set_floating(false);
                    }
                    None => {
                        unmapped
                            .window_rules
                            .layout_mode
                            .get_or_insert(LayoutMode::tiled())
                            .toggle_floating();
                    }
                }
            }
        })
        .await
    }

    async fn set_focused(&self, request: Request<SetFocusedRequest>) -> TonicResult<()> {
        let request = request.into_inner();

        let window_id = WindowId(request.window_id);

        let set_or_toggle = request.set_or_toggle();

        if set_or_toggle == SetOrToggle::Unspecified {
            return Err(Status::invalid_argument("unspecified set or toggle"));
        }

        let set = match set_or_toggle {
            SetOrToggle::Unspecified => unreachable!(),
            SetOrToggle::Set => Some(true),
            SetOrToggle::Unset => Some(false),
            SetOrToggle::Toggle => None,
        };

        run_unary_no_response(&self.sender, move |state| {
            if let Some(window) = window_id.window(&state.pinnacle) {
                crate::api::window::set_focused(state, &window, set);
            } else if let Some(unmapped) = window_id.unmapped_window_mut(&mut state.pinnacle) {
                match set {
                    Some(set) => unmapped.window_rules.focused = Some(set),
                    None => {
                        let focused = unmapped.window_rules.focused.get_or_insert(true);
                        *focused = !*focused;
                    }
                }
            }
        })
        .await
    }

    async fn set_decoration_mode(
        &self,
        request: Request<SetDecorationModeRequest>,
    ) -> TonicResult<()> {
        let request = request.into_inner();

        let window_id = WindowId(request.window_id);

        let mode = match request.decoration_mode() {
            v1::DecorationMode::Unspecified => {
                return Err(Status::invalid_argument("decoration mode was unspecified"))
            }
            v1::DecorationMode::ClientSide => crate::window::rules::DecorationMode::ClientSide,
            v1::DecorationMode::ServerSide => crate::window::rules::DecorationMode::ServerSide,
        };

        run_unary_no_response(&self.sender, move |state| {
            if let Some(window) = window_id.window(&state.pinnacle) {
                crate::api::window::set_decoration_mode(state, &window, mode);
            } else if let Some(unmapped) = window_id.unmapped_window_mut(&mut state.pinnacle) {
                unmapped.window_rules.decoration_mode = Some(mode);
            }
        })
        .await
    }

    async fn move_to_tag(&self, request: Request<MoveToTagRequest>) -> TonicResult<()> {
        let request = request.into_inner();

        let window_id = WindowId(request.window_id);
        let tag_id = TagId::new(request.tag_id);

        run_unary_no_response(&self.sender, move |state| {
            let Some(tag) = tag_id.tag(&state.pinnacle) else { return };

            if let Some(window) = window_id.window(&state.pinnacle) {
                crate::api::window::move_to_tag(state, &window, &tag);
            } else if let Some(unmapped) = window_id.unmapped_window_mut(&mut state.pinnacle) {
                unmapped.window_rules.tags = [tag].into_iter().collect();
            }
        })
        .await
    }

    async fn set_tag(&self, request: Request<SetTagRequest>) -> TonicResult<()> {
        let request = request.into_inner();

        let window_id = WindowId(request.window_id);
        let tag_id = TagId::new(request.tag_id);

        let set_or_toggle = request.set_or_toggle();

        if set_or_toggle == SetOrToggle::Unspecified {
            return Err(Status::invalid_argument("unspecified set or toggle"));
        }

        let set = match set_or_toggle {
            SetOrToggle::Unspecified => unreachable!(),
            SetOrToggle::Set => Some(true),
            SetOrToggle::Unset => Some(false),
            SetOrToggle::Toggle => None,
        };

        run_unary_no_response(&self.sender, move |state| {
            let Some(tag) = tag_id.tag(&state.pinnacle) else { return };

            if let Some(window) = window_id.window(&state.pinnacle) {
                crate::api::window::set_tag(state, &window, &tag, set);
            } else if let Some(unmapped) = window_id.unmapped_window_mut(&mut state.pinnacle) {
                match set {
                    Some(true) => {
                        unmapped.window_rules.tags.insert(tag.clone());
                    }
                    Some(false) => {
                        unmapped.window_rules.tags.shift_remove(&tag);
                    }
                    None => {
                        if unmapped.window_rules.tags.contains(&tag) {
                            // Prevent toggling that would leave a window tagless
                            if unmapped.window_rules.tags.len() > 1 {
                                unmapped.window_rules.tags.shift_remove(&tag);
                            }
                        } else {
                            unmapped.window_rules.tags.insert(tag.clone());
                        }
                    }
                }
            }
        })
        .await
    }

    async fn raise(&self, request: Request<RaiseRequest>) -> TonicResult<()> {
        let request = request.into_inner();

        let window_id = WindowId(request.window_id);

        run_unary_no_response(&self.sender, move |state| {
            let Some(window) = window_id.window(&state.pinnacle) else {
                return;
            };

            crate::api::window::raise(state, window);
        })
        .await
    }

    async fn move_grab(&self, request: Request<MoveGrabRequest>) -> TonicResult<()> {
        let request = request.into_inner();
        let button = request.button;

        run_unary_no_response(&self.sender, move |state| {
            crate::api::window::move_grab(state, button);
        })
        .await
    }

    async fn resize_grab(&self, request: Request<ResizeGrabRequest>) -> TonicResult<()> {
        let request = request.into_inner();
        let button = request.button;

        run_unary_no_response(&self.sender, move |state| {
            crate::api::window::resize_grab(state, button);
        })
        .await
    }

    async fn window_rule(
        &self,
        request: Request<Streaming<WindowRuleRequest>>,
    ) -> TonicResult<Self::WindowRuleStream> {
        let in_stream = request.into_inner();

        let id_ctr = Arc::new(AtomicU32::default());

        run_bidirectional_streaming_mapped(
            self.sender.clone(),
            in_stream,
            {
                let id_ctr = id_ctr.clone();
                move |state, request| {
                    let Some(request) = request.request else {
                        return;
                    };

                    match request {
                        v1::window_rule_request::Request::Finished(finished) => {
                            let id = finished.request_id;
                            id_ctr.store(id, Ordering::Release);

                            for win in state.pinnacle.window_rule_state.finished_windows() {
                                let Some(unmapped) = state
                                    .pinnacle
                                    .unmapped_windows
                                    .iter()
                                    .find(|unmapped| unmapped.window == win)
                                else {
                                    continue;
                                };

                                state.pinnacle.apply_window_rules(unmapped);

                                match unmapped.window.underlying_surface() {
                                    WindowSurface::Wayland(toplevel) => {
                                        // This should be an assert, but currently Smithay does not
                                        // raise a protocol error when a client commits a buffer
                                        // before the initial configure
                                        if toplevel.is_initial_configure_sent() {
                                            error!(
                                                app_id = ?win.class(),
                                                "toplevel already configured after window rules; \
                                                this is either a bug with Pinnacle or the client application \
                                                committed a buffer before receiving an initial configure, \
                                                which is a protocol error"
                                            );
                                        }
                                        toplevel.send_configure();
                                    }
                                    WindowSurface::X11(surface) => {
                                        assert!(!surface.is_mapped());
                                        let _ = surface.set_mapped(true);
                                    }
                                }
                            }
                        }
                    }
                }
            },
            |state, sender, _join_handle| {
                state.pinnacle.window_rule_state.new_sender(sender, id_ctr);
            },
            |request| {
                Ok(WindowRuleResponse {
                    response: Some(v1::window_rule_response::Response::NewWindow(
                        v1::window_rule_response::NewWindowRequest {
                            request_id: request.request_id,
                            window_id: request.window_id.0,
                        },
                    )),
                })
            },
        )
    }
}
