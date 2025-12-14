use smithay_client_toolkit::shell::xdg::XdgPositioner;
use snowcap_api_defs::snowcap::popup::v1::{
    CloseRequest, NewPopupRequest, NewPopupResponse, UpdatePopupRequest, UpdatePopupResponse,
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

        let Some(widget_def) = request.widget_def else {
            return Err(Status::invalid_argument("no widget def"));
        };

        run_unary(&self.sender, move |state| {
            let Some(f) = crate::api::widget::v1::widget_def_to_fn(widget_def) else {
                return Err(Status::invalid_argument("widget def was null"));
            };

            let Ok(positioner) = XdgPositioner::new(&state.xdg_shell) else {
                return Err(Status::internal("Could not create xdg_positioner"));
            };

            positioner.set_anchor_rect(10, 10, 1, 1);

            let Some(popup) = SnowcapPopup::new(state, parent_id.into(), positioner, f) else {
                return Err(Status::internal("Failed to create popup"));
            };

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
