//! Render utilities.

pub mod snapshot;
pub mod surface;

use anyhow::{bail, Context};
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::element::utils::{Relocate, RelocateRenderElement};
use smithay::backend::renderer::element::{self, Element, Id};
use smithay::backend::renderer::utils::CommitCounter;
use smithay::backend::renderer::{Bind, Color32F, Frame, Offscreen, Renderer};
use smithay::utils::{Point, Rectangle};
use smithay::{
    backend::renderer::{
        element::RenderElement,
        gles::{GlesRenderer, GlesTexture},
        sync::SyncPoint,
    },
    utils::{Physical, Scale, Size, Transform},
};

use super::{OutputRenderElement, PRenderer};

/// A texture from [`render_to_encompassing_texture`].
///
/// Additionally contains the sync point and location that the elements would originally
/// be drawn at.
#[derive(Debug, Clone)]
pub struct EncompassingTexture {
    /// The rendered texture.
    pub texture: GlesTexture,
    /// The sync point.
    pub sync_point: SyncPoint,
    /// The location the elements would have been originally drawn at.
    pub loc: Point<i32, Physical>,
}

/// Renders the given elements to a [`GlesTexture`] that encompasses them all.
///
/// See [`render_to_texture`].
///
/// From https://github.com/YaLTeR/niri/blob/efb39e466b5248eb894745e899de33661493511d/src/render_helpers/mod.rs#L158
pub fn render_to_encompassing_texture<E: RenderElement<GlesRenderer>>(
    renderer: &mut GlesRenderer,
    elements: impl IntoIterator<Item = E>,
    scale: Scale<f64>,
    transform: Transform,
    fourcc: Fourcc,
) -> anyhow::Result<EncompassingTexture> {
    let elements = elements.into_iter().collect::<Vec<_>>();

    let encompassing_geo = elements
        .iter()
        .map(|elem| elem.geometry(scale))
        .reduce(|first, second| first.merge(second))
        .context("no elements to render")?;

    // Make elements relative to (0, 0) for rendering
    let elements = elements.iter().rev().map(|elem| {
        RelocateRenderElement::from_element(
            elem,
            (-encompassing_geo.loc.x, -encompassing_geo.loc.y),
            Relocate::Relative,
        )
    });

    let (texture, sync_point) = render_to_texture(
        renderer,
        elements,
        encompassing_geo.size,
        scale,
        transform,
        fourcc,
    )?;

    Ok(EncompassingTexture {
        texture,
        sync_point,
        loc: encompassing_geo.loc,
    })
}

/// Renders the given elements to a [`GlesTexture`].
///
/// `elements` should have their locations relative to (0, 0), as they will be rendered
/// to a texture with a rectangle of loc (0, 0) and size `size`. This can be achieved
/// by wrapping them in a
/// [`RelocateRenderElement`][smithay::backend::renderer::element::utils::RelocateRenderElement].
///
/// Elements outside of the rectangle will be clipped.
///
/// From https://github.com/YaLTeR/niri/blob/efb39e466b5248eb894745e899de33661493511d/src/render_helpers/mod.rs#L180
pub fn render_to_texture(
    renderer: &mut GlesRenderer,
    elements: impl IntoIterator<Item = impl RenderElement<GlesRenderer>>,
    size: Size<i32, Physical>,
    scale: Scale<f64>,
    transform: Transform,
    fourcc: Fourcc,
) -> anyhow::Result<(GlesTexture, SyncPoint)> {
    if size.is_empty() {
        // Causes GL_INVALID_VALUE when binding
        bail!("size was empty");
    }

    let buffer_size = size.to_logical(1).to_buffer(1, Transform::Normal);
    let texture: GlesTexture = renderer
        .create_buffer(fourcc, buffer_size)
        .context("failed to create texture")?;
    renderer
        .bind(texture.clone())
        .context("failed to bind texture")?;

    let sync_point =
        render_elements_to_bound_framebuffer(renderer, elements, size, scale, transform)?;

    Ok((texture, sync_point))
}

/// Renders the given elements into the currently bound framebuffer.
///
/// `elements` should have their locations relative to (0, 0), as they will be rendered
/// to a texture with a rectangle of loc (0, 0) and size `size`.
///
/// From https://github.com/YaLTeR/niri/blob/efb39e466b5248eb894745e899de33661493511d/src/render_helpers/mod.rs#L295
fn render_elements_to_bound_framebuffer(
    renderer: &mut GlesRenderer,
    elements: impl IntoIterator<Item = impl RenderElement<GlesRenderer>>,
    size: Size<i32, Physical>,
    scale: Scale<f64>,
    transform: Transform,
) -> anyhow::Result<SyncPoint> {
    // TODO: see what transform.invert() does here
    let dst_rect = Rectangle::from_size(transform.transform_size(size));

    let mut frame = renderer
        .render(size, transform)
        .context("failed to start render")?;

    frame
        .clear([0.0, 0.0, 0.0, 0.0].into(), &[dst_rect])
        .context("failed to clear frame")?;

    for elem in elements {
        let src = elem.src();
        let dst = elem.geometry(scale);

        if let Some(mut damage) = dst_rect.intersection(dst) {
            damage.loc -= dst.loc;
            elem.draw(&mut frame, src, dst, &[damage], &[])
                .context("failed to draw element")?;
        }
    }

    frame.finish().context("failed to finish frame")
}

/// Renders damage rectangles for the given elements.
///
/// https://github.com/YaLTeR/niri/blob/b351f6ff220560d96a260d8dd3ad794000923481/src/render_helpers/debug.rs#L61
pub fn render_damage<R: PRenderer>(
    damage_tracker: &mut OutputDamageTracker,
    elements: &mut Vec<OutputRenderElement<R>>,
    color: Color32F,
) {
    let _span = tracy_client::span!("render_damage");

    let Ok((Some(damage), _)) = damage_tracker.damage_output(1, elements) else {
        return;
    };

    for rect in damage {
        let solid_color = SolidColorRenderElement::new(
            Id::new(),
            *rect,
            CommitCounter::default(),
            color,
            element::Kind::Unspecified,
        );
        elements.insert(0, solid_color.into());
    }
}

/// Renders opaque region rectangles on top of each element.
///
/// https://github.com/YaLTeR/niri/blob/b351f6ff220560d96a260d8dd3ad794000923481/src/render_helpers/debug.rs#L10
pub fn render_opaque_regions<R: PRenderer>(
    elements: &mut Vec<OutputRenderElement<R>>,
    scale: Scale<f64>,
) {
    let _span = tracy_client::span!("render_opaque_regions");

    let mut i = 0;
    while i < elements.len() {
        let elem = &elements[i];
        i += 1;

        let geo = elem.geometry(scale);
        let mut opaque = elem.opaque_regions(scale).to_vec();

        for rect in &mut opaque {
            rect.loc += geo.loc;
        }

        let semitransparent = geo.subtract_rects(opaque.iter().copied());

        for rect in opaque {
            let color = SolidColorRenderElement::new(
                Id::new(),
                rect,
                CommitCounter::default(),
                [0., 0., 0.2, 0.2],
                element::Kind::Unspecified,
            );
            elements.insert(i - 1, OutputRenderElement::SolidColor(color));
            i += 1;
        }

        for rect in semitransparent {
            let color = SolidColorRenderElement::new(
                Id::new(),
                rect,
                CommitCounter::default(),
                [0.3, 0., 0., 0.3],
                element::Kind::Unspecified,
            );
            elements.insert(i - 1, OutputRenderElement::SolidColor(color));
            i += 1;
        }
    }
}
