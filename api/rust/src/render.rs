//! Rendering management.

use pinnacle_api_defs::pinnacle::render::v0alpha1::{
    SetDownscaleFilterRequest, SetUpscaleFilterRequest,
};

use crate::{block_on_tokio, render};

/// A struct that allows you to manage rendering.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Render;

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
        block_on_tokio(render().set_upscale_filter(SetUpscaleFilterRequest {
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
        block_on_tokio(render().set_downscale_filter(SetDownscaleFilterRequest {
            filter: Some(filter as i32),
        }))
        .unwrap();
    }
}
