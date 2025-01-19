//! Rendering management.

use pinnacle_api_defs::pinnacle::render::{
    self,
    v1::{SetDownscaleFilterRequest, SetUpscaleFilterRequest},
};

use crate::{client::Client, BlockOnTokio};

/// What filter to use when scaling.
pub enum ScalingFilter {
    /// Use a bilinear filter.
    ///
    /// This will make up- and downscaling blurry.
    Bilinear,
    /// Use a nearest neighbor filter.
    ///
    /// This will cause scaling to look pixelated.
    NearestNeighbor,
}

impl From<ScalingFilter> for render::v1::Filter {
    fn from(value: ScalingFilter) -> Self {
        match value {
            ScalingFilter::Bilinear => render::v1::Filter::Bilinear,
            ScalingFilter::NearestNeighbor => render::v1::Filter::NearestNeighbor,
        }
    }
}

/// Sets the upscaling filter that will be used for rendering.
///
/// # Examples
///
/// ```
/// use pinnacle_api::render::ScalingFilter;
///
/// render::set_upscale_filter(ScalingFilter::NearestNeighbor);
/// ```
pub fn set_upscale_filter(filter: ScalingFilter) {
    Client::render()
        .set_upscale_filter(SetUpscaleFilterRequest {
            filter: render::v1::Filter::from(filter).into(),
        })
        .block_on_tokio()
        .unwrap();
}

/// Sets the downscaling filter that will be used for rendering.
///
/// # Examples
///
/// ```
/// use pinnacle_api::render::ScalingFilter;
///
/// render::set_downscale_filter(ScalingFilter::NearestNeighbor);
/// ```
pub fn set_downscale_filter(filter: ScalingFilter) {
    Client::render()
        .set_downscale_filter(SetDownscaleFilterRequest {
            filter: render::v1::Filter::from(filter).into(),
        })
        .block_on_tokio()
        .unwrap();
}
