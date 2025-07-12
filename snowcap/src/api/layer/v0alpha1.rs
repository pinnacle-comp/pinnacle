use std::num::NonZeroU32;

use smithay_client_toolkit::shell::wlr_layer;
use snowcap_api_defs::snowcap::layer;
use snowcap_api_defs::snowcap::layer::v0alpha1::CloseRequest;
use snowcap_api_defs::snowcap::layer::v0alpha1::NewLayerRequest;
use snowcap_api_defs::snowcap::layer::v0alpha1::NewLayerResponse;
use snowcap_api_defs::snowcap::layer::v0alpha1::layer_service_server;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::api::run_unary;
use crate::api::run_unary_no_response;
use crate::api::widget::v0alpha1::widget_def_to_fn;
use crate::layer::ExclusiveZone;
use crate::layer::LayerId;
use crate::layer::SnowcapLayer;

#[tonic::async_trait]
impl layer_service_server::LayerService for super::LayerService {
    async fn new_layer(
        &self,
        request: Request<NewLayerRequest>,
    ) -> Result<Response<NewLayerResponse>, Status> {
        let request = request.into_inner();

        let anchor = request.anchor();
        let exclusive_zone = request.exclusive_zone();
        let keyboard_interactivity = request.keyboard_interactivity();
        let layer = request.layer();

        let Some(widget_def) = request.widget_def else {
            return Err(Status::invalid_argument("no widget def"));
        };

        let width = request.width.unwrap_or(600);
        let height = request.height.unwrap_or(480);

        let anchor = match anchor {
            layer::v0alpha1::Anchor::Unspecified => wlr_layer::Anchor::empty(),
            layer::v0alpha1::Anchor::Top => wlr_layer::Anchor::TOP,
            layer::v0alpha1::Anchor::Bottom => wlr_layer::Anchor::BOTTOM,
            layer::v0alpha1::Anchor::Left => wlr_layer::Anchor::LEFT,
            layer::v0alpha1::Anchor::Right => wlr_layer::Anchor::RIGHT,
            layer::v0alpha1::Anchor::TopLeft => wlr_layer::Anchor::TOP | wlr_layer::Anchor::LEFT,
            layer::v0alpha1::Anchor::TopRight => wlr_layer::Anchor::TOP | wlr_layer::Anchor::RIGHT,
            layer::v0alpha1::Anchor::BottomLeft => {
                wlr_layer::Anchor::BOTTOM | wlr_layer::Anchor::LEFT
            }
            layer::v0alpha1::Anchor::BottomRight => {
                wlr_layer::Anchor::BOTTOM | wlr_layer::Anchor::RIGHT
            }
        };
        let exclusive_zone = match exclusive_zone {
            0 => ExclusiveZone::Respect,
            x if x.is_positive() => ExclusiveZone::Exclusive(NonZeroU32::new(x as u32).unwrap()),
            _ => ExclusiveZone::Ignore,
        };

        let keyboard_interactivity = match keyboard_interactivity {
            layer::v0alpha1::KeyboardInteractivity::Unspecified
            | layer::v0alpha1::KeyboardInteractivity::None => {
                wlr_layer::KeyboardInteractivity::None
            }
            layer::v0alpha1::KeyboardInteractivity::OnDemand => {
                wlr_layer::KeyboardInteractivity::OnDemand
            }
            layer::v0alpha1::KeyboardInteractivity::Exclusive => {
                wlr_layer::KeyboardInteractivity::Exclusive
            }
        };

        let layer = match layer {
            layer::v0alpha1::Layer::Unspecified => wlr_layer::Layer::Top,
            layer::v0alpha1::Layer::Background => wlr_layer::Layer::Background,
            layer::v0alpha1::Layer::Bottom => wlr_layer::Layer::Bottom,
            layer::v0alpha1::Layer::Top => wlr_layer::Layer::Top,
            layer::v0alpha1::Layer::Overlay => wlr_layer::Layer::Overlay,
        };

        run_unary(&self.sender, move |state| {
            let Some(f) = widget_def_to_fn(widget_def) else {
                return Err(Status::invalid_argument("widget def was null"));
            };

            let layer = SnowcapLayer::new(
                state,
                width,
                height,
                layer,
                anchor,
                exclusive_zone,
                keyboard_interactivity,
                f,
            );

            let ret = Ok(NewLayerResponse {
                layer_id: Some(layer.layer_id.0),
            });

            state.layers.push(layer);

            ret
        })
        .await
    }

    async fn close(&self, request: Request<CloseRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let Some(id) = request.layer_id else {
            return Err(Status::invalid_argument("layer id was null"));
        };

        let id = LayerId(id);

        run_unary_no_response(&self.sender, move |state| {
            state.layers.retain(|sn_layer| sn_layer.layer_id != id);
        })
        .await
    }
}
