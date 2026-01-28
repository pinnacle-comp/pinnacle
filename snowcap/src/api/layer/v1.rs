use std::num::NonZeroU32;

use anyhow::Context;
use smithay_client_toolkit::shell::wlr_layer;
use snowcap_api_defs::snowcap::layer::{
    self,
    v1::{
        CloseRequest, NewLayerRequest, NewLayerResponse, OperateLayerRequest, OperateLayerResponse,
        UpdateLayerRequest, UpdateLayerResponse, ViewRequest, ViewResponse, layer_service_server,
    },
};
use tonic::{Request, Response, Status};

use crate::{
    api::{run_unary, run_unary_no_response, widget::v1::widget_def_to_fn},
    layer::{ExclusiveZone, LayerId, SnowcapLayer},
    util::convert::TryFromApi,
};

#[tonic::async_trait]
impl layer_service_server::LayerService for super::LayerService {
    async fn new_layer(
        &self,
        request: Request<NewLayerRequest>,
    ) -> Result<Response<NewLayerResponse>, Status> {
        let request = request.into_inner();

        let anchor = request.anchor();
        let exclusive_zone = request.exclusive_zone;
        let keyboard_interactivity = request.keyboard_interactivity();
        let layer = request.layer();

        let Some(widget_def) = request.widget_def else {
            return Err(Status::invalid_argument("no widget def"));
        };

        let anchor = match anchor {
            layer::v1::Anchor::Unspecified | layer::v1::Anchor::None => wlr_layer::Anchor::empty(),
            layer::v1::Anchor::Top => wlr_layer::Anchor::TOP,
            layer::v1::Anchor::Bottom => wlr_layer::Anchor::BOTTOM,
            layer::v1::Anchor::Left => wlr_layer::Anchor::LEFT,
            layer::v1::Anchor::Right => wlr_layer::Anchor::RIGHT,
            layer::v1::Anchor::TopLeft => wlr_layer::Anchor::TOP | wlr_layer::Anchor::LEFT,
            layer::v1::Anchor::TopRight => wlr_layer::Anchor::TOP | wlr_layer::Anchor::RIGHT,
            layer::v1::Anchor::BottomLeft => wlr_layer::Anchor::BOTTOM | wlr_layer::Anchor::LEFT,
            layer::v1::Anchor::BottomRight => wlr_layer::Anchor::BOTTOM | wlr_layer::Anchor::RIGHT,
        };
        let exclusive_zone = match exclusive_zone {
            0 => ExclusiveZone::Respect,
            x if x.is_positive() => ExclusiveZone::Exclusive(NonZeroU32::new(x as u32).unwrap()),
            _ => ExclusiveZone::Ignore,
        };

        let keyboard_interactivity = match keyboard_interactivity {
            layer::v1::KeyboardInteractivity::Unspecified
            | layer::v1::KeyboardInteractivity::None => wlr_layer::KeyboardInteractivity::None,
            layer::v1::KeyboardInteractivity::OnDemand => {
                wlr_layer::KeyboardInteractivity::OnDemand
            }
            layer::v1::KeyboardInteractivity::Exclusive => {
                wlr_layer::KeyboardInteractivity::Exclusive
            }
        };

        let layer = match layer {
            layer::v1::Layer::Unspecified => wlr_layer::Layer::Top,
            layer::v1::Layer::Background => wlr_layer::Layer::Background,
            layer::v1::Layer::Bottom => wlr_layer::Layer::Bottom,
            layer::v1::Layer::Top => wlr_layer::Layer::Top,
            layer::v1::Layer::Overlay => wlr_layer::Layer::Overlay,
        };

        run_unary(&self.sender, move |state| {
            let Some(f) = crate::api::widget::v1::widget_def_to_fn(widget_def, state) else {
                return Err(Status::invalid_argument("widget def was null"));
            };

            let layer = SnowcapLayer::new(
                state,
                None,
                layer,
                anchor,
                exclusive_zone,
                keyboard_interactivity,
                f,
            );

            let ret = Ok(NewLayerResponse {
                layer_id: layer.layer_id.0,
            });

            state.layers.push(layer);

            ret
        })
        .await
    }

    async fn close(&self, request: Request<CloseRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let id = request.layer_id;
        let id = LayerId(id);

        run_unary_no_response(&self.sender, move |state| {
            state.layers.retain(|sn_layer| sn_layer.layer_id != id);
        })
        .await
    }

    async fn operate_layer(
        &self,
        request: Request<OperateLayerRequest>,
    ) -> Result<Response<OperateLayerResponse>, Status> {
        let OperateLayerRequest {
            layer_id,
            operation,
        } = request.into_inner();

        let id = LayerId(layer_id);

        run_unary(&self.sender, move |state| {
            let Some(layer) = state.layers.iter_mut().find(|layer| layer.layer_id == id) else {
                return Ok(OperateLayerResponse {});
            };
            let Some(operation) = operation else {
                return Ok(OperateLayerResponse {});
            };

            let operation =
                Box::try_from_api(operation).context("While processing OperateLayerRequest");
            let mut operation = match operation {
                Err(e) => {
                    tracing::error!("{e:?}");
                    return Ok(OperateLayerResponse {});
                }
                Ok(o) => o,
            };

            layer.operate(&mut operation);

            Ok(OperateLayerResponse {})
        })
        .await
    }

    async fn update_layer(
        &self,
        request: Request<UpdateLayerRequest>,
    ) -> Result<Response<UpdateLayerResponse>, Status> {
        let request = request.into_inner();

        let id = request.layer_id;
        let id = LayerId(id);

        let anchor = match request.anchor() {
            layer::v1::Anchor::Unspecified => None,
            layer::v1::Anchor::Top => Some(wlr_layer::Anchor::TOP),
            layer::v1::Anchor::Bottom => Some(wlr_layer::Anchor::BOTTOM),
            layer::v1::Anchor::Left => Some(wlr_layer::Anchor::LEFT),
            layer::v1::Anchor::Right => Some(wlr_layer::Anchor::RIGHT),
            layer::v1::Anchor::TopLeft => Some(wlr_layer::Anchor::TOP | wlr_layer::Anchor::LEFT),
            layer::v1::Anchor::TopRight => Some(wlr_layer::Anchor::TOP | wlr_layer::Anchor::RIGHT),
            layer::v1::Anchor::BottomLeft => {
                Some(wlr_layer::Anchor::BOTTOM | wlr_layer::Anchor::LEFT)
            }
            layer::v1::Anchor::BottomRight => {
                Some(wlr_layer::Anchor::BOTTOM | wlr_layer::Anchor::RIGHT)
            }
            layer::v1::Anchor::None => Some(wlr_layer::Anchor::empty()),
        };
        let exclusive_zone = request
            .exclusive_zone
            .map(|exclusive_zone| match exclusive_zone {
                0 => ExclusiveZone::Respect,
                x if x.is_positive() => {
                    ExclusiveZone::Exclusive(NonZeroU32::new(x as u32).unwrap())
                }
                _ => ExclusiveZone::Ignore,
            });
        let keyboard_interactivity = match request.keyboard_interactivity() {
            layer::v1::KeyboardInteractivity::Unspecified => None,
            layer::v1::KeyboardInteractivity::None => Some(wlr_layer::KeyboardInteractivity::None),
            layer::v1::KeyboardInteractivity::OnDemand => {
                Some(wlr_layer::KeyboardInteractivity::OnDemand)
            }
            layer::v1::KeyboardInteractivity::Exclusive => {
                Some(wlr_layer::KeyboardInteractivity::Exclusive)
            }
        };
        let z_layer = match request.layer() {
            layer::v1::Layer::Unspecified => None,
            layer::v1::Layer::Background => Some(wlr_layer::Layer::Background),
            layer::v1::Layer::Bottom => Some(wlr_layer::Layer::Bottom),
            layer::v1::Layer::Top => Some(wlr_layer::Layer::Top),
            layer::v1::Layer::Overlay => Some(wlr_layer::Layer::Overlay),
        };

        let widget_def = request.widget_def;

        run_unary(&self.sender, move |state| {
            let widget_def = widget_def.and_then(|def| widget_def_to_fn(def, state));

            let Some(layer) = state.layers.iter_mut().find(|layer| layer.layer_id == id) else {
                return Ok(UpdateLayerResponse {});
            };

            layer.update_properties(
                z_layer,
                anchor,
                exclusive_zone,
                keyboard_interactivity,
                widget_def,
            );

            Ok(UpdateLayerResponse {})
        })
        .await
    }

    async fn request_view(
        &self,
        request: Request<ViewRequest>,
    ) -> Result<Response<ViewResponse>, Status> {
        let request = request.into_inner();

        let id = request.layer_id;
        let id = LayerId(id);

        run_unary(&self.sender, move |state| {
            let Some(layer) = state.layers.iter_mut().find(|layer| layer.layer_id == id) else {
                return Ok(ViewResponse {});
            };

            layer.request_view();
            layer.schedule_redraw();

            Ok(ViewResponse {})
        })
        .await
    }
}
