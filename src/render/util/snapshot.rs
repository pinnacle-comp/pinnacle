//! Utilities for capturing snapshots of windows and other elements.

use std::cell::OnceCell;
use std::collections::HashSet;
use std::rc::Rc;

use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element;
use smithay::backend::renderer::element::texture::{TextureBuffer, TextureRenderElement};
use smithay::backend::renderer::element::utils::RescaleRenderElement;
use smithay::output::Output;
use smithay::utils::Logical;
use smithay::{
    backend::renderer::{
        element::RenderElement,
        gles::{GlesRenderer, GlesTexture},
    },
    utils::{Physical, Point, Scale, Transform},
};
use tracing::debug;

use crate::layout::transaction::{LayoutSnapshot, SnapshotRenderElement, SnapshotTarget};
use crate::render::texture::CommonTextureRenderElement;
use crate::render::{AsGlesRenderer, PRenderer};
use crate::state::{Pinnacle, State, WithState};
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
        scale: Scale<f64>,
        alpha: f32,
    ) -> Option<SnapshotRenderElement<R>> {
        let (texture, loc) = self.texture(renderer.as_gles_renderer())?;
        let buffer = TextureBuffer::from_texture(renderer, texture, 1, Transform::Normal, None);
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

        Some(SnapshotRenderElement::Snapshot(
            RescaleRenderElement::from_element(
                WlSurfaceTextureRenderElement::Texture(common),
                loc,
                scale,
            ),
        ))
    }
}

impl WindowElement {
    /// Capture a snapshot for this window and store it in its user data.
    pub fn capture_snapshot_and_store(
        &self,
        renderer: &mut GlesRenderer,
        location: Point<i32, Logical>,
        scale: Scale<f64>,
        alpha: f32,
    ) -> Option<LayoutSnapshot> {
        self.with_state_mut(|state| {
            if state.snapshot.is_none() || self.is_x11() {
                let elements = self.texture_render_elements(renderer, location, scale, alpha);
                if !elements.is_empty() {
                    state.snapshot = Some(RenderSnapshot::new(elements, scale));
                }
            }

            state.snapshot.clone()
        })
    }
}

impl State {
    /// Capture snapshots for all tiled windows on this output.
    ///
    /// Any windows in `also_include` are also included in the capture.
    pub fn capture_snapshots_on_output(
        &mut self,
        output: &Output,
        also_include: impl IntoIterator<Item = WindowElement>,
    ) {
        self.backend.with_renderer(|renderer| {
            capture_snapshots_on_output(&mut self.pinnacle, renderer, output, also_include);
        });
    }
}

#[derive(Debug, Default)]
pub struct OutputSnapshots {
    pub fullscreen_and_above: Vec<SnapshotTarget>,
    pub under_fullscreen: Vec<SnapshotTarget>,
}

pub fn capture_snapshots_on_output(
    pinnacle: &mut Pinnacle,
    renderer: &mut GlesRenderer,
    output: &Output,
    also_include: impl IntoIterator<Item = WindowElement>,
) {
    let split_index = pinnacle
        .space
        .elements()
        .filter(|win| {
            win.is_on_active_tag_on_output(output)
                || (win.is_on_active_tag()
                    && win.with_state(|state| state.window_state.is_floating()))
        })
        .position(|win| win.with_state(|state| state.window_state.is_fullscreen()));

    let mut under_fullscreen = pinnacle
        .space
        .elements()
        .filter(|win| {
            win.is_on_active_tag_on_output(output)
                || (win.is_on_active_tag()
                    && win.with_state(|state| state.window_state.is_floating()))
        })
        .cloned()
        .collect::<Vec<_>>();

    let fullscreen_and_up =
        under_fullscreen.split_off(split_index.unwrap_or(under_fullscreen.len()));

    #[allow(clippy::mutable_key_type)]
    let also_include = also_include.into_iter().collect::<HashSet<_>>();

    let mut flat_map = |win: WindowElement| {
        if win.with_state(|state| state.window_state.is_tiled()) || also_include.contains(&win) {
            let loc = pinnacle.space.element_location(&win)? - output.current_location();
            let snapshot = win.capture_snapshot_and_store(
                renderer,
                loc,
                output.current_scale().fractional_scale().into(),
                1.0,
            );

            snapshot.map(|ss| SnapshotTarget::Snapshot {
                snapshot: ss,
                window: win.clone(),
            })
        } else {
            Some(SnapshotTarget::Window(win))
        }
    };

    let under_fullscreen_snapshots = under_fullscreen
        .into_iter()
        .rev()
        .flat_map(&mut flat_map)
        .collect();

    let fullscreen_and_up_snapshots = fullscreen_and_up
        .into_iter()
        .rev()
        .flat_map(&mut flat_map)
        .collect();

    output.with_state_mut(|state| {
        state.snapshots.fullscreen_and_above = fullscreen_and_up_snapshots;
        state.snapshots.under_fullscreen = under_fullscreen_snapshots;
    });
}
