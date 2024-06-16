//! The Snowcap widget system.
//! // TODO: these docs

use integration::Integration;
use snowcap_api::layer::Layer;

use crate::ApiModules;

pub mod integration;

/// Snowcap modules and Pinnacle integration.
pub struct Snowcap {
    /// Create layer surface widgets.
    pub layer: &'static Layer,
    /// Pinnacle integrations.
    pub integration: &'static Integration,
}

impl Snowcap {
    pub(crate) fn new(layer: Layer) -> Self {
        Self {
            layer: Box::leak(Box::new(layer)),
            integration: Box::leak(Box::new(Integration::new())),
        }
    }

    pub(crate) fn finish_init(&self, api: ApiModules) {
        self.integration.finish_init(api);
    }
}
