//! Layout transactions.

use smithay::backend::renderer::element::utils::RescaleRenderElement;

use crate::render::util::{snapshot::RenderSnapshot, surface::WlSurfaceTextureRenderElement};

/// Type for window snapshots.
pub type LayoutSnapshot = RenderSnapshot<WlSurfaceTextureRenderElement>;

pub type SnapshotRenderElement = RescaleRenderElement<WlSurfaceTextureRenderElement>;
