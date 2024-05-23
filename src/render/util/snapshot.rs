//! Utilities for capturing snapshots of windows and other elements.

use std::cell::OnceCell;

use smithay::backend::allocator::Fourcc;
use smithay::{
    backend::renderer::{
        element::RenderElement,
        gles::{GlesRenderer, GlesTexture},
    },
    utils::{Physical, Point, Scale, Transform},
};
use tracing::error;

use super::{render_to_encompassing_texture, EncompassingTexture};

/// A snapshot of given elements that can be rendered at some point in the future.
pub struct RenderSnapshot<E> {
    /// Rendered elements.
    ///
    /// These are not used directly in rendering due to floating-point rounding
    /// inaccuracies that cause pixel imperfections when being displayed.
    elements: Vec<E>,
    /// The original scale used to create this snapshot.
    ///
    /// Used to render this snapshot at different scales.
    scale: Scale<f64>,
    /// The texture that elements will be rendered into.
    ///
    /// Happens lazily for performance.
    texture: OnceCell<(GlesTexture, Point<i32, Physical>)>,
}

impl<E: RenderElement<GlesRenderer>> RenderSnapshot<E> {
    /// Creates a new snapshot from elements.
    pub fn new(elements: impl IntoIterator<Item = E>, scale: Scale<f64>) -> Self {
        Self {
            elements: elements.into_iter().collect(),
            scale,
            texture: OnceCell::new(),
        }
    }

    /// Get the texture, rendering it to a new one if it doesn't exist.
    pub fn texture(
        &self,
        renderer: &mut GlesRenderer,
    ) -> Option<(GlesTexture, Point<i32, Physical>)> {
        // Not `get_or_init` because that would require the cell be an option/result
        // and I didn't want that
        if self.texture.get().is_none() {
            let EncompassingTexture {
                texture,
                sync_point: _,
                loc,
            } = match render_to_encompassing_texture(
                renderer,
                &self.elements,
                self.scale,
                Transform::Normal, // TODO: transform
                Fourcc::Argb8888,
            ) {
                Ok(tex) => tex,
                Err(err) => {
                    error!("Failed to render to encompassing texture: {err}");
                    return None;
                }
            };
            let Ok(()) = self.texture.set((texture, loc)) else {
                unreachable!()
            };
        }
        self.texture.get().cloned()
    }
}
