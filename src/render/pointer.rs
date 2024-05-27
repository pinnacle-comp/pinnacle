// SPDX-License-Identifier: GPL-3.0-or-later

use smithay::{
    backend::renderer::{
        element::{
            self,
            surface::{self, WaylandSurfaceRenderElement},
            texture::{TextureBuffer, TextureRenderElement},
            AsRenderElements,
        },
        ImportAll, Renderer, Texture,
    },
    input::pointer::CursorImageStatus,
    render_elements,
    utils::{Physical, Point, Scale},
};

use super::PRenderer;

pub struct PointerElement<T: Texture> {
    texture: Option<TextureBuffer<T>>,
    status: CursorImageStatus,
    kind: element::Kind,
}

impl<T: Texture> Default for PointerElement<T> {
    fn default() -> Self {
        Self {
            texture: Default::default(),
            status: CursorImageStatus::default_named(),
            kind: element::Kind::Cursor,
        }
    }
}

impl<T: Texture> PointerElement<T> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn set_status(&mut self, status: CursorImageStatus) {
        self.status = status;
    }

    pub fn set_texture(&mut self, texture: TextureBuffer<T>) {
        self.texture = Some(texture);
    }

    pub fn set_element_kind(&mut self, kind: element::Kind) {
        self.kind = kind;
    }
}

render_elements! {
    #[derive(Debug)]
    pub PointerRenderElement<R> where R: ImportAll;
    Surface=WaylandSurfaceRenderElement<R>,
    Texture=TextureRenderElement<<R as Renderer>::TextureId>,
}

impl<R: PRenderer> AsRenderElements<R> for PointerElement<R::TextureId> {
    type RenderElement = PointerRenderElement<R>;

    fn render_elements<C: From<Self::RenderElement>>(
        &self,
        renderer: &mut R,
        location: Point<i32, Physical>,
        scale: Scale<f64>,
        alpha: f32,
    ) -> Vec<C> {
        match &self.status {
            CursorImageStatus::Hidden => vec![],
            CursorImageStatus::Named(_) => {
                if let Some(texture) = self.texture.as_ref() {
                    vec![PointerRenderElement::<R>::from(
                        TextureRenderElement::from_texture_buffer(
                            location.to_f64(),
                            texture,
                            None,
                            None,
                            None,
                            self.kind,
                        ),
                    )
                    .into()]
                } else {
                    vec![]
                }
            }
            CursorImageStatus::Surface(surface) => {
                let elements: Vec<PointerRenderElement<R>> =
                    surface::render_elements_from_surface_tree(
                        renderer, surface, location, scale, alpha, self.kind,
                    );
                elements.into_iter().map(C::from).collect()
            }
        }
    }
}
