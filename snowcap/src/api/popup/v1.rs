use anyhow::Context;
use smithay_client_toolkit::reexports::protocols::xdg::shell::client::xdg_positioner;
use snowcap_api_defs::snowcap::popup::v1::{
    self, CloseRequest, GetPopupEventsRequest, GetPopupEventsResponse, NewPopupRequest,
    NewPopupResponse, OperatePopupRequest, OperatePopupResponse, UpdatePopupRequest,
    UpdatePopupResponse, ViewRequest, ViewResponse, new_popup_request, popup_service_server,
};
use tonic::{Request, Response, Status};

use crate::{
    api::{
        ResponseStream, run_server_streaming_mapped, run_unary, run_unary_no_response,
        widget::v1::widget_def_to_fn,
    },
    decoration::DecorationId,
    layer::LayerId,
    popup::{self, PopupEvent, PopupId, SnowcapPopup},
    util::convert::{FromApi, TryFromApi},
};

#[tonic::async_trait]
impl popup_service_server::PopupService for super::PopupService {
    type GetPopupEventsStream = ResponseStream<GetPopupEventsResponse>;

    async fn new_popup(
        &self,
        request: Request<NewPopupRequest>,
    ) -> Result<Response<NewPopupResponse>, Status> {
        let request = request.into_inner();

        let Some(parent_id) = request.parent_id else {
            return Err(Status::invalid_argument("no parent id"));
        };

        let parent_id: popup::ParentId = parent_id.into();

        let Some(position) = request.position.clone().map(popup::Position::from) else {
            return Err(Status::invalid_argument("no position."));
        };
        let anchor = Option::from_api(request.anchor());
        let gravity = Option::from_api(request.gravity());
        let offset = request.offset.map(popup::Offset::from);
        let constraints_adjust = request
            .constraints_adjust
            .map(xdg_positioner::ConstraintAdjustment::from_api);

        let Some(widget_def) = request.widget_def else {
            return Err(Status::invalid_argument("no widget def"));
        };

        let grab_keyboard = !request.no_grab;
        let replace = !request.no_replace;

        run_unary(&self.sender, move |state| {
            let Some(f) = crate::api::widget::v1::widget_def_to_fn(widget_def) else {
                return Err(Status::invalid_argument("widget def was null"));
            };

            let existing = state
                .popups
                .iter()
                .find(|p| p.parent_id == parent_id)
                .map(|p| p.popup_id);
            if let Some(existing) = existing
                && replace
            {
                state.popup_destroy(existing);
            } else if existing.is_some() {
                return Err(Status::failed_precondition(
                    "Another popup with the same parent already exists",
                ));
            }

            let toplevel_id = match parent_id {
                popup::ParentId::Popup(popup_id) => state
                    .popups
                    .iter()
                    .find(|p| p.popup_id == popup_id)
                    .map(|p| p.toplevel_id),
                _ => Some(parent_id),
            };

            // INFO: This is kinda the nuclear option. The original idea was to dismiss the popup
            // stack with keyboard focus since we can't have more than one, but Smithay doesn't
            // update focus when a popup is destroyed so we can't rely on this. It may be possible
            // to filter-out non-grabbing popups, but I'm not sure it's worth doing so anyway. As
            // it stand, we can only have one popup stack for Snowcap.

            let existing = state
                .popups
                .iter()
                .find(|p| Some(p.toplevel_id) != toplevel_id)
                .map(|p| p.popup_id);
            if let Some(existing) = existing
                && replace
            {
                state.popup_destroy(existing);
            } else if existing.is_some() {
                return Err(Status::failed_precondition("Another popup already exists."));
            }

            let popup = SnowcapPopup::new(
                state,
                parent_id,
                position,
                anchor,
                gravity,
                offset,
                constraints_adjust,
                grab_keyboard,
                f,
            )
            .map_err(|e| {
                use popup::Error;

                match e {
                    Error::ParentNotFound => Status::invalid_argument("parent not found."),
                    Error::ToplevelNotFound => {
                        Status::invalid_argument("toplevel surface not found.")
                    }
                    Error::InvalidPosition => Status::invalid_argument("invalid position."),
                    Error::Positioner => Status::internal("Failed to create positioner."),
                    Error::CreateFailed => Status::internal("Failed to create popup."),
                }
            })?;

            let ret = Ok(NewPopupResponse {
                popup_id: popup.popup_id.0,
            });

            state.popups.push(popup);

            ret
        })
        .await
    }

    async fn close(&self, request: Request<CloseRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let id = request.popup_id;
        let id = PopupId(id);

        run_unary_no_response(&self.sender, move |state| {
            state.popup_destroy(id);
        })
        .await
    }

    async fn operate_popup(
        &self,
        request: Request<OperatePopupRequest>,
    ) -> Result<Response<OperatePopupResponse>, Status> {
        let OperatePopupRequest {
            popup_id,
            operation,
        } = request.into_inner();

        let id = PopupId(popup_id);

        run_unary(&self.sender, move |state| {
            let Some(popup) = state.popup_for_id(id) else {
                return Ok(OperatePopupResponse {});
            };

            let Some(operation) = operation else {
                return Ok(OperatePopupResponse {});
            };

            let operation =
                Box::try_from_api(operation).context("While processing OperatePopupRequest");
            let mut operation = match operation {
                Err(e) => {
                    tracing::error!("{e:?}");
                    return Ok(OperatePopupResponse {});
                }
                Ok(o) => o,
            };

            popup.operate(&mut operation);

            Ok(OperatePopupResponse {})
        })
        .await
    }

    async fn update_popup(
        &self,
        request: Request<UpdatePopupRequest>,
    ) -> Result<Response<UpdatePopupResponse>, Status> {
        let request = request.into_inner();

        let id = request.popup_id;
        let id = PopupId(id);

        let position = request.position.clone().map(popup::Position::from);
        let anchor = Option::from_api(request.anchor());
        let gravity = Option::from_api(request.gravity());
        let offset = request.offset.map(popup::Offset::from);
        let constraints_adjust = request
            .constraints_adjust
            .map(xdg_positioner::ConstraintAdjustment::from_api);

        let widget_def = request.widget_def;

        run_unary(&self.sender, move |state| {
            let mut new_anchor_rect = None;

            let Some(parent_id) = state.popup_for_id(id).map(|p| p.parent_id) else {
                return Ok(UpdatePopupResponse {});
            };

            if let Some(position) = position {
                let anchor_rect = match parent_id {
                    popup::ParentId::Popup(id) => {
                        let p = state
                            .popups
                            .iter_mut()
                            .find(|p| p.popup_id == id)
                            .ok_or(Status::internal("parent not found."))?;

                        position
                            .anchor_rect_for(&mut p.surface)
                            .ok_or(Status::invalid_argument("invalid position."))?
                    }
                    popup::ParentId::Layer(id) => {
                        let l = state
                            .layers
                            .iter_mut()
                            .find(|l| l.layer_id == id)
                            .ok_or(Status::internal("parent not found."))?;

                        position
                            .anchor_rect_for(&mut l.surface)
                            .ok_or(Status::invalid_argument("invalid position."))?
                    }
                    popup::ParentId::Decoration(id) => {
                        let deco = state
                            .decorations
                            .iter_mut()
                            .find(|deco| deco.decoration_id == id)
                            .ok_or(Status::internal("parent not found."))?;

                        position
                            .anchor_rect_for(&mut deco.surface)
                            .ok_or(Status::invalid_argument("invalid position."))?
                    }
                };

                new_anchor_rect = Some(anchor_rect);
            }

            let Some(popup) = state.popup_for_id(id) else {
                return Ok(UpdatePopupResponse {});
            };

            popup.update_properties(
                new_anchor_rect,
                anchor,
                gravity,
                offset,
                constraints_adjust,
                widget_def.and_then(widget_def_to_fn),
            );

            Ok(UpdatePopupResponse {})
        })
        .await
    }

    async fn get_popup_events(
        &self,
        request: Request<GetPopupEventsRequest>,
    ) -> Result<Response<Self::GetPopupEventsStream>, Status> {
        let request = request.into_inner();

        let id = request.popup_id;

        run_server_streaming_mapped(
            &self.sender,
            move |state, sender| {
                if let Some(popup) = state.popup_for_id(PopupId(id)) {
                    popup.popup_event_sender = Some(sender);
                }
            },
            move |events| {
                Ok(GetPopupEventsResponse {
                    popup_events: events.into_iter().map(Into::into).collect(),
                })
            },
        )
    }

    async fn request_view(
        &self,
        request: Request<ViewRequest>,
    ) -> Result<Response<ViewResponse>, Status> {
        let request = request.into_inner();

        let id = request.popup_id;
        let id = PopupId(id);

        run_unary(&self.sender, move |state| {
            let Some(popup) = state.popups.iter_mut().find(|p| p.popup_id == id) else {
                return Ok(ViewResponse {});
            };

            popup.request_view();
            popup.schedule_redraw();

            Ok(ViewResponse {})
        })
        .await
    }
}

impl From<new_popup_request::ParentId> for popup::ParentId {
    fn from(value: new_popup_request::ParentId) -> Self {
        use new_popup_request::ParentId;
        match value {
            ParentId::LayerId(id) => popup::ParentId::Layer(LayerId(id)),
            ParentId::DecoId(id) => popup::ParentId::Decoration(DecorationId(id)),
            ParentId::PopupId(id) => popup::ParentId::Popup(PopupId(id)),
        }
    }
}

impl From<v1::Position> for popup::Position {
    fn from(value: v1::Position) -> Self {
        use v1::position::Strategy;

        let Some(strategy) = value.strategy else {
            return popup::Position::AtCursor;
        };

        match strategy {
            Strategy::AtCursor(_) => popup::Position::AtCursor,
            Strategy::Absolute(v1::Rectangle {
                x,
                y,
                width,
                height,
            }) => popup::Position::Absolute {
                x,
                y,
                width,
                height,
            },
            Strategy::AtWidget(id) => popup::Position::Widget(id),
        }
    }
}

impl From<v1::Offset> for popup::Offset {
    fn from(value: v1::Offset) -> Self {
        let v1::Offset { x, y } = value;

        popup::Offset {
            x: x as i32,
            y: y as i32,
        }
    }
}

impl FromApi<v1::Anchor> for Option<xdg_positioner::Anchor> {
    fn from_api(value: v1::Anchor) -> Self {
        use v1::Anchor;

        let ret = match value {
            Anchor::Unspecified => return None,
            Anchor::Top => xdg_positioner::Anchor::Top,
            Anchor::Bottom => xdg_positioner::Anchor::Bottom,
            Anchor::Left => xdg_positioner::Anchor::Left,
            Anchor::Right => xdg_positioner::Anchor::Right,
            Anchor::TopLeft => xdg_positioner::Anchor::TopLeft,
            Anchor::BottomLeft => xdg_positioner::Anchor::BottomLeft,
            Anchor::TopRight => xdg_positioner::Anchor::TopRight,
            Anchor::BottomRight => xdg_positioner::Anchor::BottomRight,
            Anchor::None => xdg_positioner::Anchor::None,
        };

        Some(ret)
    }
}

impl FromApi<v1::Gravity> for Option<xdg_positioner::Gravity> {
    fn from_api(value: v1::Gravity) -> Self {
        use v1::Gravity;

        let ret = match value {
            Gravity::Unspecified => return None,
            Gravity::Top => xdg_positioner::Gravity::Top,
            Gravity::Bottom => xdg_positioner::Gravity::Bottom,
            Gravity::Left => xdg_positioner::Gravity::Left,
            Gravity::Right => xdg_positioner::Gravity::Right,
            Gravity::TopLeft => xdg_positioner::Gravity::TopLeft,
            Gravity::BottomLeft => xdg_positioner::Gravity::BottomLeft,
            Gravity::TopRight => xdg_positioner::Gravity::TopRight,
            Gravity::BottomRight => xdg_positioner::Gravity::BottomRight,
            Gravity::None => xdg_positioner::Gravity::None,
        };

        Some(ret)
    }
}

impl FromApi<v1::ConstraintsAdjust> for xdg_positioner::ConstraintAdjustment {
    fn from_api(api_type: v1::ConstraintsAdjust) -> Self {
        let v1::ConstraintsAdjust {
            none,
            slide_x,
            slide_y,
            flip_x,
            flip_y,
            resize_x,
            resize_y,
        } = api_type;

        let mut ret = Self::None;

        if none {
            return ret;
        };

        if slide_x {
            ret |= Self::SlideX;
        }

        if slide_y {
            ret |= Self::SlideY;
        }

        if flip_x {
            ret |= Self::FlipX;
        }

        if flip_y {
            ret |= Self::FlipY;
        }

        if resize_x {
            ret |= Self::ResizeX;
        }

        if resize_y {
            ret |= Self::ResizeY;
        }

        ret
    }
}

impl From<PopupEvent> for snowcap_api_defs::snowcap::popup::v1::PopupEvent {
    fn from(value: PopupEvent) -> Self {
        use crate::handlers::keyboard::KeyboardFocusEvent;
        use snowcap_api_defs::snowcap::popup::v1::popup_event::{self, Focus};

        let PopupEvent::Focus(f) = value;

        match f {
            KeyboardFocusEvent::FocusGained => Self {
                event: Some(popup_event::Event::Focus(Focus::Gained.into())),
            },
            KeyboardFocusEvent::FocusLost => Self {
                event: Some(popup_event::Event::Focus(Focus::Lost.into())),
            },
        }
    }
}
