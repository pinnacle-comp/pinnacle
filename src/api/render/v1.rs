use pinnacle_api_defs::pinnacle::render::{
    self,
    v1::{Filter, SetDownscaleFilterRequest, SetUpscaleFilterRequest},
};
use smithay::backend::renderer::TextureFilter;
use tonic::{Request, Status};

use crate::{
    api::{run_unary_no_response, TonicResult},
    backend::BackendData,
};

#[tonic::async_trait]
impl render::v1::render_service_server::RenderService for super::RenderService {
    async fn set_upscale_filter(
        &self,
        request: Request<SetUpscaleFilterRequest>,
    ) -> TonicResult<()> {
        let request = request.into_inner();
        if let Filter::Unspecified = request.filter() {
            return Err(Status::invalid_argument("unspecified filter"));
        }

        let filter = match request.filter() {
            Filter::Bilinear => TextureFilter::Linear,
            Filter::NearestNeighbor => TextureFilter::Nearest,
            _ => unreachable!(),
        };

        run_unary_no_response(&self.sender, move |state| {
            state.backend.set_upscale_filter(filter);
            for output in state.pinnacle.outputs.clone() {
                state.backend.reset_buffers(&output);
                state.schedule_render(&output);
            }
        })
        .await
    }

    async fn set_downscale_filter(
        &self,
        request: Request<SetDownscaleFilterRequest>,
    ) -> TonicResult<()> {
        let request = request.into_inner();
        if let Filter::Unspecified = request.filter() {
            return Err(Status::invalid_argument("unspecified filter"));
        }

        let filter = match request.filter() {
            Filter::Bilinear => TextureFilter::Linear,
            Filter::NearestNeighbor => TextureFilter::Nearest,
            _ => unreachable!(),
        };

        run_unary_no_response(&self.sender, move |state| {
            state.backend.set_downscale_filter(filter);
            for output in state.pinnacle.outputs.clone() {
                state.backend.reset_buffers(&output);
                state.schedule_render(&output);
            }
        })
        .await
    }
}
