//! Rendering management.

use pinnacle_api_defs::pinnacle::render::v0alpha1::{
    render_service_client::RenderServiceClient, SetDownscaleFilterRequest, SetUpscaleFilterRequest,
};
use tonic::transport::Channel;

use crate::block_on_tokio;

/// A struct that allows you to manage rendering.
#[derive(Debug, Clone)]
pub struct Render {
    client: RenderServiceClient<Channel>,
}

/// What filter to use when scaling.
pub enum ScalingFilter {
    /// Use a bilinear filter.
    ///
    /// This will make up- and downscaling blurry.
    Bilinear = 1,
    /// Use a nearest neighbor filter.
    ///
    /// This will cause scaling to look pixelated.
    NearestNeighbor,
}

impl Render {
    pub(crate) fn new(channel: Channel) -> Self {
        Self {
            client: RenderServiceClient::new(channel),
        }
    }

    /// Set the upscaling filter that will be used for rendering.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::render::ScalingFilter;
    ///
    /// render.set_upscale_filter(ScalingFilter::NearestNeighbor);
    /// ```
    pub fn set_upscale_filter(&self, filter: ScalingFilter) {
        let mut client = self.client.clone();
        block_on_tokio(client.set_upscale_filter(SetUpscaleFilterRequest {
            filter: Some(filter as i32),
        }))
        .unwrap();
    }

    /// Set the downscaling filter that will be used for rendering.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::render::ScalingFilter;
    ///
    /// render.set_downscale_filter(ScalingFilter::NearestNeighbor);
    /// ```
    pub fn set_downscale_filter(&self, filter: ScalingFilter) {
        let mut client = self.client.clone();
        block_on_tokio(client.set_downscale_filter(SetDownscaleFilterRequest {
            filter: Some(filter as i32),
        }))
        .unwrap();
    }
}
