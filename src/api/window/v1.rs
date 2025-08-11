use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use indexmap::IndexSet;
use pinnacle_api_defs::pinnacle::{
    util::{self, v1::SetOrToggle},
    window::{
        self,
        v1::{
            self, CloseRequest, GetAppIdRequest, GetAppIdResponse, GetFocusedRequest,
            GetFocusedResponse, GetForeignToplevelListIdentifierRequest,
            GetForeignToplevelListIdentifierResponse, GetLayoutModeRequest, GetLayoutModeResponse,
            GetLocRequest, GetLocResponse, GetRequest, GetResponse, GetSizeRequest,
            GetSizeResponse, GetTagIdsRequest, GetTagIdsResponse, GetTitleRequest,
            GetTitleResponse, GetWindowsInDirRequest, GetWindowsInDirResponse, LowerRequest,
            LowerResponse, MoveGrabRequest, MoveToOutputRequest, MoveToOutputResponse,
            MoveToTagRequest, RaiseRequest, ResizeGrabRequest, ResizeTileRequest,
            SetDecorationModeRequest, SetFloatingRequest, SetFocusedRequest, SetFullscreenRequest,
            SetGeometryRequest, SetMaximizedRequest, SetTagRequest, SetTagsRequest,
            SetTagsResponse, SwapRequest, SwapResponse, WindowRuleRequest, WindowRuleResponse,
        },
    },
};
use smithay::{
    reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1,
    utils::Size,
};
use tonic::{Request, Status, Streaming};
use tracing::warn;

use crate::{
    api::{
        ResponseStream, TonicResult, run_bidirectional_streaming_mapped, run_unary,
        run_unary_no_response,
    },
    focus::keyboard::KeyboardFocusTarget,
    layout::tree::ResizeDir,
    output::OutputName,
    state::WithState,
    tag::TagId,
    util::rect::Direction,
    window::{
        UnmappedState,
        window_state::{LayoutMode, LayoutModeKind, WindowId},
    },
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
                .and_then(|win| state.pinnacle.space.element_geometry(&win))
                .map(|geo| geo.size);

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
                    let current_keyboard_focus =
                        state.pinnacle.seat.get_keyboard()?.current_focus()?;

                    Some(matches!(
                        current_keyboard_focus,
                        KeyboardFocusTarget::Window(window) if window == win
                    ))
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
                .or_else(|| {
                    window_id
                        .unmapped_window(&state.pinnacle)
                        .map(|unmapped| unmapped.window.clone())
                })
                .map(|win| win.with_state(|state| state.layout_mode))
                .unwrap_or(LayoutMode::new_tiled());

            Ok(GetLayoutModeResponse {
                layout_mode: match layout_mode.current() {
                    LayoutModeKind::Tiled => window::v1::LayoutMode::Tiled,
                    LayoutModeKind::Floating => window::v1::LayoutMode::Floating,
                    LayoutModeKind::Maximized => window::v1::LayoutMode::Maximized,
                    LayoutModeKind::Fullscreen => window::v1::LayoutMode::Fullscreen,
                    LayoutModeKind::Spilled => window::v1::LayoutMode::Spilled,
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
                .or_else(|| {
                    window_id
                        .unmapped_window(&state.pinnacle)
                        .map(|unmapped| unmapped.window.clone())
                })
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

    async fn get_windows_in_dir(
        &self,
        request: Request<GetWindowsInDirRequest>,
    ) -> TonicResult<GetWindowsInDirResponse> {
        let request = request.into_inner();
        let window_id = WindowId(request.window_id);
        let dir = request.dir();

        if dir == util::v1::Dir::Unspecified {
            return Err(Status::invalid_argument("no dir was specified"));
        }

        run_unary(&self.sender, move |state| {
            let Some(win) = window_id.window(&state.pinnacle) else {
                return Ok(GetWindowsInDirResponse {
                    window_ids: Vec::new(),
                });
            };

            let Some(win_rect) = state.pinnacle.space.element_geometry(&win) else {
                return Ok(GetWindowsInDirResponse {
                    window_ids: Vec::new(),
                });
            };

            let candidates = state.pinnacle.space.elements().collect::<Vec<_>>();
            let rects = candidates
                .iter()
                .map(|win| state.pinnacle.space.element_geometry(win).expect("mapped"))
                .collect::<Vec<_>>();

            let idxs = crate::util::rect::closest_in_dir(
                win_rect,
                &rects,
                match dir {
                    util::v1::Dir::Unspecified => unreachable!(),
                    util::v1::Dir::Left => Direction::Left,
                    util::v1::Dir::Right => Direction::Right,
                    util::v1::Dir::Up => Direction::Up,
                    util::v1::Dir::Down => Direction::Down,
                },
            );

            let window_ids = idxs
                .into_iter()
                .map(|idx| candidates[idx].with_state(|state| state.id.0))
                .collect();

            Ok(GetWindowsInDirResponse { window_ids })
        })
        .await
    }

    async fn get_foreign_toplevel_list_identifier(
        &self,
        request: Request<GetForeignToplevelListIdentifierRequest>,
    ) -> TonicResult<GetForeignToplevelListIdentifierResponse> {
        let window_id = WindowId(request.into_inner().window_id);

        run_unary(&self.sender, move |state| {
            let identifier = window_id
                .window(&state.pinnacle)
                .or_else(|| {
                    window_id
                        .unmapped_window(&state.pinnacle)
                        .map(|unmapped| unmapped.window.clone())
                })
                .and_then(|win| {
                    win.with_state(|state| {
                        state
                            .foreign_toplevel_list_handle
                            .as_ref()
                            .map(|handle| handle.identifier())
                    })
                });

            Ok(GetForeignToplevelListIdentifierResponse { identifier })
        })
        .await
    }

    async fn close(&self, request: Request<CloseRequest>) -> TonicResult<()> {
        let request = request.into_inner();

        let window_id = WindowId(request.window_id);

        run_unary_no_response(&self.sender, move |state| {
            let Some(window) = window_id.window(&state.pinnacle) else {
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
            } else if let Some(unmapped) = window_id.unmapped_window_mut(&mut state.pinnacle)
                && let UnmappedState::WaitingForRules { rules, .. } = &mut unmapped.state
            {
                rules.floating_x = x;
                rules.floating_y = y;

                let size = Size::from((w.unwrap_or_default() as i32, h.unwrap_or_default() as i32));
                rules.floating_size = Some(size);
            }
        })
        .await
    }

    async fn resize_tile(&self, request: Request<ResizeTileRequest>) -> TonicResult<()> {
        let request = request.into_inner();

        let window_id = WindowId(request.window_id);

        run_unary_no_response(&self.sender, move |state| {
            if let Some(window) = window_id.window(&state.pinnacle) {
                if window.with_state(|state| !state.layout_mode.is_tiled()) {
                    return;
                }
                let mut size = window.geometry().size;

                size.w += request.right;
                size.h += request.bottom;
                state.resize_tile(&window, size, ResizeDir::Ahead, ResizeDir::Ahead);

                size.w -= request.left;
                size.h -= request.top;
                state.resize_tile(&window, size, ResizeDir::Behind, ResizeDir::Behind);
                // Perform one more resize ahead to grow in the other direction
                // if we couldn't resize behind
                state.resize_tile(&window, size, ResizeDir::Ahead, ResizeDir::Ahead);
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
                state
                    .pinnacle
                    .update_window_layout_mode(&window, |layout_mode| match fullscreen {
                        Some(set) => layout_mode.set_fullscreen(set),
                        None => layout_mode.toggle_fullscreen(),
                    });
            } else if let Some(unmapped) = window_id.unmapped_window_mut(&mut state.pinnacle)
                && let UnmappedState::WaitingForRules { rules, .. } = &mut unmapped.state
            {
                match fullscreen {
                    Some(true) => {
                        rules
                            .layout_mode
                            .get_or_insert(LayoutMode::new_fullscreen())
                            .set_fullscreen(true);
                    }
                    Some(false) => {
                        if let Some(layout_mode) = rules.layout_mode.as_mut() {
                            layout_mode.set_fullscreen(false);
                        }
                    }
                    None => {
                        rules
                            .layout_mode
                            .get_or_insert(LayoutMode::new_tiled())
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
                state
                    .pinnacle
                    .update_window_layout_mode(&window, |layout_mode| match maximized {
                        Some(set) => layout_mode.set_maximized(set),
                        None => layout_mode.toggle_maximized(),
                    });
            } else if let Some(unmapped) = window_id.unmapped_window_mut(&mut state.pinnacle)
                && let UnmappedState::WaitingForRules { rules, .. } = &mut unmapped.state
            {
                match maximized {
                    Some(true) => {
                        rules
                            .layout_mode
                            .get_or_insert(LayoutMode::new_maximized())
                            .set_maximized(true);
                    }
                    Some(false) => {
                        if let Some(layout_mode) = rules.layout_mode.as_mut() {
                            layout_mode.set_maximized(false);
                        }
                    }
                    None => {
                        rules
                            .layout_mode
                            .get_or_insert(LayoutMode::new_tiled())
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
                state
                    .pinnacle
                    .update_window_layout_mode(&window, |layout_mode| match floating {
                        Some(set) => layout_mode.set_floating(set),
                        None => layout_mode.toggle_floating(),
                    });
            } else if let Some(unmapped) = window_id.unmapped_window_mut(&mut state.pinnacle)
                && let UnmappedState::WaitingForRules { rules, .. } = &mut unmapped.state
            {
                match floating {
                    Some(true) => {
                        rules
                            .layout_mode
                            .get_or_insert(LayoutMode::new_floating())
                            .set_floating(true);
                    }
                    Some(false) => {
                        rules
                            .layout_mode
                            .get_or_insert(LayoutMode::new_floating())
                            .set_floating(false);
                    }
                    None => {
                        rules
                            .layout_mode
                            .get_or_insert(LayoutMode::new_tiled())
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
            } else if let Some(unmapped) = window_id.unmapped_window_mut(&mut state.pinnacle)
                && let UnmappedState::WaitingForRules { rules, .. } = &mut unmapped.state
            {
                match set {
                    Some(set) => rules.focused = Some(set),
                    None => {
                        let focused = rules.focused.get_or_insert(true);
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
                return Err(Status::invalid_argument("decoration mode was unspecified"));
            }
            v1::DecorationMode::ClientSide => zxdg_toplevel_decoration_v1::Mode::ClientSide,
            v1::DecorationMode::ServerSide => zxdg_toplevel_decoration_v1::Mode::ServerSide,
        };

        run_unary_no_response(&self.sender, move |state| {
            if let Some(window) = window_id.window(&state.pinnacle) {
                crate::api::window::set_decoration_mode(state, &window, mode);
            } else if let Some(unmapped) = window_id.unmapped_window_mut(&mut state.pinnacle)
                && let UnmappedState::WaitingForRules { rules, .. } = &mut unmapped.state
            {
                rules.decoration_mode = Some(mode);
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
            } else if let Some(unmapped) = window_id.unmapped_window_mut(&mut state.pinnacle)
                && let UnmappedState::WaitingForRules { rules, .. } = &mut unmapped.state
            {
                rules.tags = Some([tag].into_iter().collect());
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
            } else if let Some(unmapped) = window_id.unmapped_window_mut(&mut state.pinnacle)
                && let UnmappedState::WaitingForRules { rules, .. } = &mut unmapped.state
            {
                let tags = rules.tags.get_or_insert(Default::default());
                match set {
                    Some(true) => {
                        tags.insert(tag.clone());
                    }
                    Some(false) => {
                        tags.shift_remove(&tag);
                    }
                    None => {
                        if tags.contains(&tag) {
                            // Prevent toggling that would leave a window tagless
                            if tags.len() > 1 {
                                tags.shift_remove(&tag);
                            }
                        } else {
                            tags.insert(tag.clone());
                        }
                    }
                }
            }
        })
        .await
    }

    async fn set_tags(&self, request: Request<SetTagsRequest>) -> TonicResult<SetTagsResponse> {
        let request = request.into_inner();

        let window_id = WindowId(request.window_id);
        let tag_ids = request.tag_ids.into_iter().map(TagId::new);

        run_unary(&self.sender, move |state| {
            // Could possibly just filter instead of failing if any tag doesn't exist
            let Some(tags) = tag_ids
                .into_iter()
                .map(|tag_id| tag_id.tag(&state.pinnacle))
                .collect::<Option<IndexSet<_>>>()
            else {
                return Ok(SetTagsResponse {});
            };

            if tags.is_empty() {
                warn!("Cannot set a windows tags to empty");
                return Ok(SetTagsResponse {});
            }

            if let Some(window) = window_id.window(&state.pinnacle) {
                window.with_state_mut(|state| state.tags = tags);
            } else if let Some(unmapped) = window_id.unmapped_window_mut(&mut state.pinnacle)
                && let UnmappedState::WaitingForRules { rules, .. } = &mut unmapped.state
            {
                rules.tags = Some(tags);
            }

            Ok(SetTagsResponse {})
        })
        .await
    }

    async fn move_to_output(
        &self,
        request: Request<MoveToOutputRequest>,
    ) -> TonicResult<MoveToOutputResponse> {
        let request = request.into_inner();
        let window_id = WindowId(request.window_id);
        let output_name = OutputName(request.output_name);

        run_unary(&self.sender, move |state| {
            if let Some(output) = output_name.output(&state.pinnacle) {
                if let Some(window) = window_id.window(&state.pinnacle) {
                    state.pinnacle.move_window_to_output(&window, output);
                } else if let Some(unmapped) = window_id.unmapped_window_mut(&mut state.pinnacle)
                    && let UnmappedState::WaitingForRules { rules, .. } = &mut unmapped.state
                {
                    rules.tags = output
                        .with_state(|s| Some(s.focused_tags().cloned().collect::<IndexSet<_>>()));
                }
            }

            Ok(MoveToOutputResponse {})
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

    async fn lower(&self, request: Request<LowerRequest>) -> TonicResult<LowerResponse> {
        let request = request.into_inner();
        let window_id = WindowId(request.window_id);

        run_unary(&self.sender, move |state| {
            if let Some(window) = window_id.window(&state.pinnacle) {
                crate::api::window::lower(state, window);
            }

            Ok(LowerResponse {})
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

    async fn swap(&self, request: Request<SwapRequest>) -> TonicResult<SwapResponse> {
        let inner = request.into_inner();
        let window_id = WindowId(inner.window_id);
        let target_id = WindowId(inner.target_id);

        run_unary(&self.sender, move |state| {
            let window = window_id.window(&state.pinnacle);
            let target = target_id.window(&state.pinnacle);

            // Both window & target must be mapped
            if let Some((window, target)) = window.zip(target) {
                crate::api::window::swap(state, window, target);
            };

            Ok(SwapResponse {})
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
                                let Some(unmapped_idx) = state
                                    .pinnacle
                                    .unmapped_windows
                                    .iter_mut()
                                    .position(|unmapped| unmapped.window == win)
                                else {
                                    continue;
                                };

                                let mut unmapped =
                                    state.pinnacle.unmapped_windows.swap_remove(unmapped_idx);

                                state
                                    .pinnacle
                                    .apply_window_rules_and_send_initial_configure(&mut unmapped);

                                state.pinnacle.unmapped_windows.push(unmapped);
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
