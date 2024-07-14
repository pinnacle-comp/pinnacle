//! The Snowcap widget system.
//! // TODO: these docs

use integration::Integration;
use snowcap_api::layer::Layer;

pub mod integration;

/// Snowcap modules and Pinnacle integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Snowcap {
    /// Create layer surface widgets.
    pub layer: &'static Layer,
    /// Pinnacle integrations.
    pub integration: &'static Integration,
}

impl Default for Snowcap {
    fn default() -> Self {
        Self::new()
    }
}

impl Snowcap {
    /// Creates a new Snowcap struct.
    pub const fn new() -> Self {
        Self {
            layer: &Layer,
            integration: &Integration,
        }
    }
}
