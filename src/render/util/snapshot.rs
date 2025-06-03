//! Utilities for capturing snapshots of windows and other elements.

use std::cell::OnceCell;
use std::rc::Rc;

use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element;
use smithay::backend::renderer::element::texture::{TextureBuffer, TextureRenderElement};
use smithay::backend::renderer::element::utils::RescaleRenderElement;
use smithay::{
    backend::renderer::{
        element::RenderElement,
        gles::{GlesRenderer, GlesTexture},
    },
    utils::{Physical, Point, Scale, Transform},
};
use tracing::debug;

use crate::layout::transaction::{LayoutSnapshot, SnapshotRenderElement};
use crate::render::texture::CommonTextureRenderElement;
use crate::render::{AsGlesRenderer, PRenderer};
use crate::state::WithState;
use crate::window::WindowElement;

use super::surface::WlSurfaceTextureRenderElement;
use super::{render_to_encompassing_texture, EncompassingTexture};

/// A snapshot of given elements that can be rendered at some point in the future.
#[derive(Debug)]
pub struct RenderSnapshot<E> {
    /// Rendered elements.
    ///
    /// These are not used directly in rendering due to floating-point rounding
    /// inaccuracies that cause pixel imperfections when being displayed.
    elements: Rc<Vec<E>>,
    /// The original scale used to create this snapshot.
    scale: Scale<f64>,
    /// The texture that elements will be rendered into.
    ///
    /// Happens lazily for performance.
    texture: OnceCell<(GlesTexture, Point<i32, Physical>)>,
}

impl<E> Clone for RenderSnapshot<E> {
    fn clone(&self) -> Self {
        Self {
            elements: self.elements.clone(),
            scale: self.scale,
            texture: self.texture.clone(),
        }
    }
}

impl<E: RenderElement<GlesRenderer>> RenderSnapshot<E> {
    /// Creates a new snapshot from elements.
    pub fn new(elements: impl IntoIterator<Item = E>, scale: Scale<f64>) -> Self {
        Self {
            elements: Rc::new(elements.into_iter().collect()),
            scale,
            texture: OnceCell::new(),
        }
    }

    /// Get the texture, rendering it to a new one if it doesn't exist.
    fn texture(&self, renderer: &mut GlesRenderer) -> Option<(GlesTexture, Point<i32, Physical>)> {
        // Not `get_or_init` because that would require the cell be an option/result
        // and I didn't want that
        if self.texture.get().is_none() {
            let EncompassingTexture {
                texture,
                sync_point: _,
                loc,
            } = match render_to_encompassing_texture(
                renderer,
                self.elements.as_ref(),
                self.scale,
                Transform::Normal, // TODO: transform
                Fourcc::Argb8888,
            ) {
                Ok(tex) => tex,
                Err(err) => {
                    debug!("Failed to render to encompassing texture: {err}");
                    return None;
                }
            };
            let Ok(()) = self.texture.set((texture, loc)) else {
                unreachable!()
            };
        }
        self.texture.get().cloned()
    }

    /// Render elements for this snapshot.
    pub fn render_elements<R: PRenderer + AsGlesRenderer>(
        &self,
        renderer: &mut R,
        location: Point<i32, Physical>,
        scale: Scale<f64>,
        alpha: f32,
    ) -> Option<SnapshotRenderElement> {
        let renderer = renderer.as_gles_renderer();
        let (texture, offset) = self.texture(renderer)?;
        let loc = location + offset;
        let buffer: TextureBuffer<GlesTexture> =
            TextureBuffer::from_texture(renderer, texture, 1, Transform::Normal, None);
        let elem = TextureRenderElement::from_texture_buffer(
            loc.to_f64(),
            &buffer,
            Some(alpha),
            None,
            None,
            element::Kind::Unspecified,
        );

        let common = CommonTextureRenderElement::new(elem);

        // Scale in the opposite direction from the original scale to have it be the same size
        let scale = Scale::from((1.0 / scale.x, 1.0 / scale.y));

        Some(RescaleRenderElement::from_element(
            WlSurfaceTextureRenderElement::Texture(common),
            loc,
            scale,
        ))
    }
}

impl WindowElement {
    /// Capture a snapshot for this window and store it in its user data.
    pub fn capture_snapshot_and_store(
        &self,
        renderer: &mut GlesRenderer,
        scale: Scale<f64>,
        alpha: f32,
    ) -> Option<LayoutSnapshot> {
        self.with_state_mut(|state| {
            let elements = self.texture_render_elements(renderer, (0, 0).into(), scale, alpha);
            if !elements.surface_elements.is_empty() {
                state.snapshot = Some(RenderSnapshot::new(elements.surface_elements, scale));
            }
            state.snapshot.clone()
        })
    }
}
