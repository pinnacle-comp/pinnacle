//! Render utilities.

pub mod damage;
pub mod snapshot;
pub mod surface;

use anyhow::{Context, bail};
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::element::utils::{Relocate, RelocateRenderElement};
use smithay::backend::renderer::element::{self, Element, Id};
use smithay::backend::renderer::utils::CommitCounter;
use smithay::backend::renderer::{Bind, Color32F, Frame, Offscreen, Renderer, RendererSuper};
use smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer;
use smithay::reexports::wayland_server::protocol::wl_shm;
use smithay::utils::{Buffer, Point, Rectangle};
use smithay::wayland::shm::with_buffer_contents_mut;
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
    let mut texture: GlesTexture = renderer
        .create_buffer(fourcc, buffer_size)
        .context("failed to create texture")?;

    let sync_point = {
        let mut framebuffer = renderer
            .bind(&mut texture)
            .context("failed to bind texture")?;

        render_elements_to_framebuffer(
            renderer,
            &mut framebuffer,
            elements,
            size,
            scale,
            transform,
        )?
    };

    Ok((texture, sync_point))
}

/// Renders the given elements into the provided bound framebuffer.
///
/// `elements` should have their locations relative to (0, 0), as they will be rendered
/// to a texture with a rectangle of loc (0, 0) and size `size`.
///
/// From https://github.com/YaLTeR/niri/blob/efb39e466b5248eb894745e899de33661493511d/src/render_helpers/mod.rs#L295
fn render_elements_to_framebuffer(
    renderer: &mut GlesRenderer,
    framebuffer: &mut <GlesRenderer as RendererSuper>::Framebuffer<'_>,
    elements: impl IntoIterator<Item = impl RenderElement<GlesRenderer>>,
    size: Size<i32, Physical>,
    scale: Scale<f64>,
    transform: Transform,
) -> anyhow::Result<SyncPoint> {
    // TODO: see what transform.invert() does here
    let dst_rect = Rectangle::from_size(transform.transform_size(size));

    let mut frame = renderer
        .render(framebuffer, size, transform)
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
pub fn render_damage_from_elements<E: Element>(
    damage_tracker: &mut OutputDamageTracker,
    elements: &[E],
    color: Color32F,
) -> Vec<SolidColorRenderElement> {
    let _span = tracy_client::span!("render_damage");

    let Ok((Some(damage), _)) = damage_tracker.damage_output(1, elements) else {
        return Vec::new();
    };

    render_damage(damage, color)
}

/// Renders damage rectangles.
pub fn render_damage(
    damage: &[Rectangle<i32, Physical>],
    color: Color32F,
) -> Vec<SolidColorRenderElement> {
    damage
        .iter()
        .map(|rect| {
            SolidColorRenderElement::new(
                Id::new(),
                *rect,
                CommitCounter::default(),
                color,
                element::Kind::Unspecified,
            )
        })
        .collect()
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

/// Blits a rectangle of pixels from a source byte buffer into a shm wl buffer.
///
/// Fails if the provided wl buffer is not shm or either the src or dst are not Argb8888.
///
/// This function requires the src and dst to be in Argb8888 format.
pub fn blit(
    src: &[u8],
    src_size: Size<i32, Buffer>,
    src_rect: Rectangle<i32, Buffer>,

    dst: &WlBuffer,
) -> anyhow::Result<()> {
    if src.len() != (src_size.w * src_size.h * 4) as usize {
        anyhow::bail!("src was not correct len");
    }

    let Some(src_rect) = Rectangle::from_size(src_size).intersection(src_rect) else {
        anyhow::bail!("src_rect does not overlap src buffer");
    };

    with_buffer_contents_mut(dst, |mut dst, len, data| {
        if Size::new(data.width, data.height) != src_size {
            anyhow::bail!("src_size is different from dst size");
        }

        if data.format != wl_shm::Format::Argb8888 {
            anyhow::bail!("dst is not argb8888");
        }

        if src.len() != (data.stride * data.height) as usize {
            anyhow::bail!(
                "src and dst are different lens (src = {}, dst = {})",
                src.len(),
                len
            );
        }

        dst = dst.wrapping_offset(data.offset as isize);

        let stride = data.stride;

        for row_num in src_rect.loc.y..(src_rect.loc.y + src_rect.size.h) {
            let src_row = src[(stride * row_num) as usize..].as_ptr();
            // SAFETY:
            // - stride * row_num is always less than len so this is always within the allocation
            // - count * size_of::<u8>() always fits in an isize
            let dst_row = unsafe { dst.offset((stride * row_num) as isize) };

            unsafe {
                std::ptr::copy_nonoverlapping(
                    src_row.offset((src_rect.loc.x * 4) as isize),
                    dst_row.offset((src_rect.loc.x * 4) as isize),
                    (src_rect.size.w * 4) as usize,
                );
            }
        }

        Ok(())
    })
    .context("not a shm buffer")?
}

pub struct DynElement<'a, R: Renderer>(&'a dyn RenderElement<R>);

impl<'a, R: Renderer> DynElement<'a, R> {
    pub fn new(elem: &'a impl RenderElement<R>) -> Self {
        Self(elem as _)
    }
}

impl<'a, R: Renderer> Element for DynElement<'a, R> {
    fn id(&self) -> &Id {
        self.0.id()
    }

    fn current_commit(&self) -> CommitCounter {
        self.0.current_commit()
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        self.0.src()
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.0.geometry(scale)
    }

    fn location(&self, scale: Scale<f64>) -> Point<i32, Physical> {
        self.0.location(scale)
    }

    fn transform(&self) -> Transform {
        self.0.transform()
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> smithay::backend::renderer::utils::DamageSet<i32, Physical> {
        self.0.damage_since(scale, commit)
    }

    fn opaque_regions(
        &self,
        scale: Scale<f64>,
    ) -> smithay::backend::renderer::utils::OpaqueRegions<i32, Physical> {
        self.0.opaque_regions(scale)
    }

    fn alpha(&self) -> f32 {
        self.0.alpha()
    }

    fn kind(&self) -> element::Kind {
        self.0.kind()
    }
}

impl<'a, R: Renderer> RenderElement<R> for DynElement<'a, R> {
    fn draw(
        &self,
        frame: &mut <R>::Frame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
    ) -> Result<(), <R>::Error> {
        self.0.draw(frame, src, dst, damage, opaque_regions)
    }
}
