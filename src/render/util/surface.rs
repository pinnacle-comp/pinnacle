//! Utils for rendering surface trees to textures.

use smithay::{
    backend::renderer::{
        element::{
            self,
            solid::{SolidColorBuffer, SolidColorRenderElement},
            surface::{WaylandSurfaceRenderElement, WaylandSurfaceTexture},
            texture::{TextureBuffer, TextureRenderElement},
        },
        gles::GlesRenderer,
        utils::RendererSurfaceStateUserData,
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Physical, Point, Scale},
    wayland::compositor::{self, TraversalAction},
};
use tracing::warn;

use crate::{pinnacle_render_elements, render::texture::CommonTextureRenderElement};

pinnacle_render_elements! {
    #[derive(Debug)]
    pub enum WlSurfaceTextureRenderElement {
        Texture = CommonTextureRenderElement,
        SolidColor = SolidColorRenderElement,
    }
}

/// Render a surface tree as [`WlSurfaceTextureRenderElement`]s instead of wayland ones.
///
/// Needed to allow WaylandSurfaceRenderElements to be dropped to free shm buffers.
pub fn texture_render_elements_from_surface_tree(
    renderer: &mut GlesRenderer,
    surface: &WlSurface,
    location: impl Into<Point<i32, Physical>>,
    scale: impl Into<Scale<f64>>,
    alpha: f32,
) -> Vec<WlSurfaceTextureRenderElement> {
    let location = location.into().to_f64();
    let scale = scale.into();
    let mut surfaces: Vec<WlSurfaceTextureRenderElement> = Vec::new();

    compositor::with_surface_tree_downward(
        surface,
        location,
        |_, states, location| {
            let mut location = *location;
            let data = states.data_map.get::<RendererSurfaceStateUserData>();

            if let Some(data) = data {
                let data = data.lock().unwrap();

                if let Some(view) = data.view() {
                    location += view.offset.to_f64().to_physical(scale);
                    TraversalAction::DoChildren(location)
                } else {
                    TraversalAction::SkipChildren
                }
            } else {
                TraversalAction::SkipChildren
            }
        },
        |surface, states, location| {
            let mut location = *location;
            let data = states.data_map.get::<RendererSurfaceStateUserData>();

            if let Some(data) = data {
                let has_view = {
                    let data = data.lock().unwrap();
                    if let Some(view) = data.view() {
                        location += view.offset.to_f64().to_physical(scale);
                        true
                    } else {
                        false
                    }
                };

                if has_view {
                    match WaylandSurfaceRenderElement::from_surface(
                        renderer,
                        surface,
                        states,
                        location,
                        alpha,
                        element::Kind::Unspecified,
                    ) {
                        Ok(Some(surface)) => {
                            // Reconstruct the element as a TextureRenderElement

                            let data = data.lock().unwrap();
                            let view = data.view().unwrap();

                            match surface.texture() {
                                WaylandSurfaceTexture::Texture(texture) => {
                                    let texture_buffer = TextureBuffer::from_texture(
                                        renderer,
                                        texture.clone(),
                                        data.buffer_scale(),
                                        data.buffer_transform(),
                                        None,
                                    );

                                    let texture_elem = TextureRenderElement::from_texture_buffer(
                                        location,
                                        &texture_buffer,
                                        Some(alpha),
                                        Some(view.src),
                                        Some(view.dst),
                                        element::Kind::Unspecified,
                                    );

                                    surfaces
                                        .push(CommonTextureRenderElement::new(texture_elem).into());
                                }
                                WaylandSurfaceTexture::SolidColor(color) => {
                                    let solid_color_buffer =
                                        SolidColorBuffer::new(view.dst, *color);
                                    let solid_color_elem = SolidColorRenderElement::from_buffer(
                                        &solid_color_buffer,
                                        location.to_i32_round(), // INFO: is this the correct rounding
                                        scale,
                                        alpha,
                                        element::Kind::Unspecified,
                                    );
                                    surfaces.push(solid_color_elem.into());
                                }
                            }
                        }
                        Ok(None) => {} // surface is not mapped
                        Err(err) => {
                            warn!("Failed to import surface: {}", err);
                        }
                    };
                }
            }
        },
        |_, _, _| true,
    );

    surfaces
}
