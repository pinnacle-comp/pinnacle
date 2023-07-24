// SPDX-License-Identifier: GPL-3.0-or-later

use smithay::{
    backend::renderer::{
        element::{surface::WaylandSurfaceRenderElement, AsRenderElements, Wrap},
        ImportAll, ImportMem, Renderer, Texture,
    },
    desktop::space::{SpaceElement, SpaceRenderElements},
    render_elements,
    utils::{Physical, Point, Scale},
};

use crate::window::WindowElement;

use self::pointer::PointerRenderElement;

pub mod pointer;

render_elements! {
    pub CustomRenderElements<R> where R: ImportAll + ImportMem;
    Pointer=PointerRenderElement<R>,
    Surface=WaylandSurfaceRenderElement<R>,
}

render_elements! {
    pub OutputRenderElements<R, E> where R: ImportAll + ImportMem;
    Space=SpaceRenderElements<R, E>,
    Window=Wrap<E>,
    Custom=CustomRenderElements<R>,
    // TODO: preview
}

impl<R> AsRenderElements<R> for WindowElement
where
    R: Renderer + ImportAll + ImportMem,
    <R as Renderer>::TextureId: Texture + 'static,
{
    type RenderElement = WaylandSurfaceRenderElement<R>;

    fn render_elements<C: From<Self::RenderElement>>(
        &self,
        renderer: &mut R,
        location: Point<i32, Physical>,
        scale: Scale<f64>,
        alpha: f32,
    ) -> Vec<C> {
        let window_bbox = self.bbox();
        match self {
            WindowElement::Wayland(window) => {
                AsRenderElements::<R>::render_elements::<WaylandSurfaceRenderElement<R>>(
                    window, renderer, location, scale, alpha,
                )
            }
            WindowElement::X11(surface) => AsRenderElements::<R>::render_elements::<
                WaylandSurfaceRenderElement<R>,
            >(surface, renderer, location, scale, alpha),
        }
        .into_iter()
        .map(C::from)
        .collect()
    }
}
