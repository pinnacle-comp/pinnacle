use smithay::{
    backend::renderer::{
        element::{self, texture::TextureRenderElement, Element, RenderElement},
        gles::{GlesRenderer, GlesTexture},
        utils::{CommitCounter, DamageSet, OpaqueRegions},
        Renderer,
    },
    utils::{Buffer, Physical, Rectangle, Scale},
};

use crate::backend::udev::UdevRenderer;

/// TODO: docs
pub struct CommonTextureRenderElement(TextureRenderElement<GlesTexture>);

impl Element for CommonTextureRenderElement {
    fn id(&self) -> &element::Id {
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

    fn location(&self, scale: Scale<f64>) -> smithay::utils::Point<i32, Physical> {
        self.0.location(scale)
    }

    fn transform(&self) -> smithay::utils::Transform {
        self.0.transform()
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        self.0.damage_since(scale, commit)
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        self.0.opaque_regions(scale)
    }

    fn alpha(&self) -> f32 {
        self.0.alpha()
    }

    fn kind(&self) -> element::Kind {
        self.0.kind()
    }
}

impl RenderElement<GlesRenderer> for CommonTextureRenderElement {
    fn draw(
        &self,
        frame: &mut <GlesRenderer as Renderer>::Frame<'_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
    ) -> Result<(), <GlesRenderer as Renderer>::Error> {
        RenderElement::<GlesRenderer>::draw(&self.0, frame, src, dst, damage)
    }

    fn underlying_storage(
        &self,
        renderer: &mut GlesRenderer,
    ) -> Option<element::UnderlyingStorage<'_>> {
        let _ = renderer;
        None
    }
}

impl<'a> RenderElement<UdevRenderer<'a>> for CommonTextureRenderElement {
    fn draw(
        &self,
        frame: &mut <UdevRenderer<'a> as Renderer>::Frame<'_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
    ) -> Result<(), <UdevRenderer<'a> as Renderer>::Error> {
        RenderElement::<GlesRenderer>::draw(&self.0, frame.as_mut(), src, dst, damage)?;
        Ok(())
    }

    fn underlying_storage(
        &self,
        renderer: &mut UdevRenderer<'a>,
    ) -> Option<element::UnderlyingStorage<'_>> {
        let _ = renderer;
        None
    }
}
