use smithay_client_toolkit::reexports::protocols::xdg::shell::client::xdg_positioner;
use snowcap_api_defs::snowcap::popup::v1::{
    self, CloseRequest, NewPopupRequest, NewPopupResponse, UpdatePopupRequest, UpdatePopupResponse,
    ViewRequest, ViewResponse,
    new_popup_request::{self, ParentId},
    popup_service_server,
};
use tonic::{Request, Response, Status};

use crate::{
    api::{run_unary, run_unary_no_response, widget::v1::widget_def_to_fn},
    decoration::DecorationId,
    layer::LayerId,
    popup::{self, PopupId, SnowcapPopup},
    util::convert::FromApi,
};

#[tonic::async_trait]
impl popup_service_server::PopupService for super::PopupService {
    async fn new_popup(
        &self,
        request: Request<NewPopupRequest>,
    ) -> Result<Response<NewPopupResponse>, Status> {
        let request = request.into_inner();

        let Some(parent_id) = request.parent_id else {
            return Err(Status::invalid_argument("no parent id"));
        };

        if matches!(parent_id, ParentId::DecoId(_)) {
            return Err(Status::unimplemented("Decoration's popup are unavailable."));
        }

        let Some(position) = request.position.clone().map(popup::Position::from) else {
            return Err(Status::invalid_argument("no position."));
        };
        let anchor = Option::from_api(request.anchor());
        let gravity = Option::from_api(request.gravity());
        let offset = request.offset.map(popup::Offset::from);
        let no_grab = request.no_grab;

        let Some(widget_def) = request.widget_def else {
            return Err(Status::invalid_argument("no widget def"));
        };

        run_unary(&self.sender, move |state| {
            let Some(f) = crate::api::widget::v1::widget_def_to_fn(widget_def) else {
                return Err(Status::invalid_argument("widget def was null"));
            };

            let popup = SnowcapPopup::new(
                state,
                parent_id.into(),
                position,
                anchor,
                gravity,
                offset,
                !no_grab,
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

    async fn update_popup(
        &self,
        request: Request<UpdatePopupRequest>,
    ) -> Result<Response<UpdatePopupResponse>, Status> {
        let request = request.into_inner();

        let id = request.popup_id;
        let id = PopupId(id);

        let widget_def = request.widget_def;

        run_unary(&self.sender, move |state| {
            let Some(popup) = state.popup_for_id(id) else {
                return Ok(UpdatePopupResponse {});
            };

            popup.update_properties(widget_def.and_then(widget_def_to_fn));

            Ok(UpdatePopupResponse {})
        })
        .await
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
