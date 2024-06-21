// SPDX-License-Identifier: GPL-3.0-or-later

use std::{rc::Rc, sync::Mutex};

use smithay::{
    backend::renderer::{
        element::{
            self,
            memory::MemoryRenderBufferRenderElement,
            surface::{render_elements_from_surface_tree, WaylandSurfaceRenderElement},
            AsRenderElements,
        },
        ImportAll, ImportMem,
    },
    desktop::Space,
    input::pointer::CursorImageAttributes,
    output::Output,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    render_elements,
    utils::{Clock, Logical, Monotonic, Point, Scale},
    wayland::compositor,
};

use crate::{
    cursor::{CursorState, XCursor},
    window::WindowElement,
};

use super::PRenderer;

pub enum PointerElement {
    Hidden,
    Named { cursor: Rc<XCursor>, size: u32 },
    Surface { surface: WlSurface },
}

render_elements! {
    #[derive(Debug)]
    pub PointerRenderElement<R> where R: ImportAll + ImportMem;
    Surface = WaylandSurfaceRenderElement<R>,
    Memory = MemoryRenderBufferRenderElement<R>,
}

pub fn pointer_render_elements<R: PRenderer>(
    output: &Output,
    renderer: &mut R,
    cursor_state: &mut CursorState,
    space: &Space<WindowElement>,
    pointer_location: Point<f64, Logical>,
    dnd_icon: Option<&WlSurface>,
    clock: &Clock<Monotonic>,
) -> Vec<PointerRenderElement<R>> {
    let mut pointer_render_elements = Vec::new();

    let Some(output_geometry) = space.output_geometry(output) else {
        return pointer_render_elements;
    };

    let scale = Scale::from(output.current_scale().fractional_scale());

    let pointer_elem = cursor_state.pointer_element();

    if output_geometry.to_f64().contains(pointer_location) {
        let cursor_pos = pointer_location - output_geometry.loc.to_f64();

        let mut elements = match &pointer_elem {
            PointerElement::Hidden => vec![],
            PointerElement::Named { cursor, size } => {
                let image = cursor.image(clock.now().into(), *size);
                let hotspot = (image.xhot as i32, image.yhot as i32);
                let buffer = cursor_state.buffer_for_image(image);
                let elem = MemoryRenderBufferRenderElement::from_buffer(
                    renderer,
                    (cursor_pos - Point::from(hotspot).to_f64()).to_physical_precise_round(scale),
                    &buffer,
                    None,
                    None,
                    None,
                    element::Kind::Cursor,
                );

                elem.map(|elem| vec![PointerRenderElement::Memory(elem)])
                    .unwrap_or_default()
            }
            PointerElement::Surface { surface } => {
                let hotspot = compositor::with_states(surface, |states| {
                    states
                        .data_map
                        .get::<Mutex<CursorImageAttributes>>()
                        .expect("surface data map had no CursorImageAttributes")
                        .lock()
                        .expect("failed to lock mutex")
                        .hotspot
                });

                let elems = render_elements_from_surface_tree(
                    renderer,
                    surface,
                    (cursor_pos - hotspot.to_f64()).to_physical_precise_round(scale),
                    scale,
                    1.0,
                    element::Kind::Cursor,
                );

                elems
            }
        };

        if let Some(dnd_icon) = dnd_icon {
            elements.extend(AsRenderElements::render_elements(
                &smithay::desktop::space::SurfaceTree::from_surface(dnd_icon),
                renderer,
                cursor_pos.to_physical_precise_round(scale),
                scale,
                1.0,
            ));
        }

        pointer_render_elements = elements;
    }

    pointer_render_elements
}
