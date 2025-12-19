use snowcap_api_defs::snowcap::decoration::v1::{
    CloseRequest, CloseResponse, NewDecorationRequest, NewDecorationResponse,
    UpdateDecorationRequest, UpdateDecorationResponse, ViewRequest, ViewResponse,
    decoration_service_server,
};
use tonic::{Request, Response, Status};
use tracing::warn;

use crate::{
    api::{run_unary, widget::v1::widget_def_to_fn},
    decoration::{DecorationId, SnowcapDecoration},
};

#[tonic::async_trait]
impl decoration_service_server::DecorationService for super::DecorationService {
    async fn new_decoration(
        &self,
        request: Request<NewDecorationRequest>,
    ) -> Result<Response<NewDecorationResponse>, Status> {
        let request = request.into_inner();

        let Some(widget_def) = request.widget_def else {
            return Err(Status::invalid_argument("no widget def"));
        };

        let toplevel_identifier = request.foreign_toplevel_handle_identifier;
        let bounds = request.bounds.unwrap_or_default();
        let extents = request.extents.unwrap_or_default();
        let z_index = request.z_index;

        run_unary(&self.sender, move |state| {
            let Some(f) = crate::api::widget::v1::widget_def_to_fn(widget_def) else {
                return Err(Status::invalid_argument("widget def was null"));
            };

            let Some(deco) = SnowcapDecoration::new(
                state,
                toplevel_identifier,
                crate::decoration::Bounds {
                    left: bounds.left,
                    right: bounds.right,
                    top: bounds.top,
                    bottom: bounds.bottom,
                },
                z_index,
                crate::decoration::Bounds {
                    left: extents.left,
                    right: extents.right,
                    top: extents.top,
                    bottom: extents.bottom,
                },
                f,
            ) else {
                warn!("no toplevel for identifier");
                return Err(Status::not_found("no toplevel for identifier"));
            };

            let ret = Ok(NewDecorationResponse {
                decoration_id: deco.decoration_id.0,
            });

            state.decorations.push(deco);

            ret
        })
        .await
    }

    async fn close(
        &self,
        request: Request<CloseRequest>,
    ) -> Result<Response<CloseResponse>, Status> {
        let request = request.into_inner();

        let id = request.decoration_id;
        let id = DecorationId(id);

        run_unary(&self.sender, move |state| {
            state.decorations.retain(|deco| deco.decoration_id != id);
            Ok(CloseResponse {})
        })
        .await
    }

    async fn update_decoration(
        &self,
        request: Request<UpdateDecorationRequest>,
    ) -> Result<Response<UpdateDecorationResponse>, Status> {
        let request = request.into_inner();

        let id = request.decoration_id;
        let id = DecorationId(id);

        let widget_def = request.widget_def;
        let bounds = request.bounds;
        let extents = request.extents;
        let z_index = request.z_index;

        run_unary(&self.sender, move |state| {
            let Some(deco) = state
                .decorations
                .iter_mut()
                .find(|deco| deco.decoration_id == id)
            else {
                return Ok(UpdateDecorationResponse {});
            };

            deco.update_properties(
                widget_def.and_then(widget_def_to_fn),
                bounds.map(|bounds| crate::decoration::Bounds {
                    left: bounds.left,
                    right: bounds.right,
                    top: bounds.top,
                    bottom: bounds.bottom,
                }),
                extents.map(|extents| crate::decoration::Bounds {
                    left: extents.left,
                    right: extents.right,
                    top: extents.top,
                    bottom: extents.bottom,
                }),
                z_index,
            );

            Ok(UpdateDecorationResponse {})
        })
        .await
    }

    async fn request_view(
        &self,
        request: Request<ViewRequest>,
    ) -> Result<Response<ViewResponse>, Status> {
        let request = request.into_inner();

        let id = request.decoration_id;
        let id = DecorationId(id);

        run_unary(&self.sender, move |state| {
            let Some(deco) = state
                .decorations
                .iter_mut()
                .find(|deco| deco.decoration_id == id)
            else {
                return Ok(ViewResponse {});
            };

            deco.request_view();
            deco.schedule_redraw();

            Ok(ViewResponse {})
        })
        .await
    }
}
