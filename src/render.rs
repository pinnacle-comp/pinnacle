use smithay::{
    backend::renderer::{
        element::{surface::WaylandSurfaceRenderElement, Wrap},
        ImportAll, ImportMem,
    },
    desktop::space::SpaceRenderElements,
    render_elements,
};

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
