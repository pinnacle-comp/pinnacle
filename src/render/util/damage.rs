use smithay::{
    backend::renderer::{
        Renderer,
        element::{Element, Id, RenderElement},
        utils::CommitCounter,
    },
    utils::{Buffer, Physical, Rectangle, Scale, Size},
};

#[derive(Debug, Clone)]
pub struct BufferDamageElement {
    id: Id,
    commit: CommitCounter,
    geometry: Rectangle<i32, Buffer>,
}

impl BufferDamageElement {
    pub fn new(geometry: Rectangle<i32, Buffer>) -> Self {
        Self {
            id: Id::new(),
            commit: Default::default(),
            geometry,
        }
    }
}

impl Element for BufferDamageElement {
    fn id(&self) -> &Id {
        &self.id
    }

    fn current_commit(&self) -> CommitCounter {
        self.commit
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        Rectangle::from_size(Size::new(1.0, 1.0))
    }

    fn geometry(&self, _scale: Scale<f64>) -> Rectangle<i32, Physical> {
        Rectangle::new(
            (self.geometry.loc.x, self.geometry.loc.y).into(),
            (self.geometry.size.w, self.geometry.size.h).into(),
        )
    }
}

impl<R: Renderer> RenderElement<R> for BufferDamageElement {
    fn draw(
        &self,
        _frame: &mut <R>::Frame<'_, '_>,
        _src: Rectangle<f64, Buffer>,
        _dst: Rectangle<i32, Physical>,
        _damage: &[Rectangle<i32, Physical>],
        _opaque_regions: &[Rectangle<i32, Physical>],
    ) -> Result<(), <R>::Error> {
        Ok(())
    }
}
