// SPDX-License-Identifier: GPL-3.0-or-later

use std::rc::Rc;

use smithay::{
    backend::renderer::{
        ImportAll, ImportMem,
        element::{
            self, AsRenderElements, Element, Id,
            memory::MemoryRenderBufferRenderElement,
            surface::{WaylandSurfaceRenderElement, render_elements_from_surface_tree},
        },
    },
    input::pointer::CursorImageSurfaceData,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    render_elements,
    utils::{Clock, Monotonic, Physical, Point},
    wayland::compositor,
};

use crate::cursor::{CursorState, XCursor};

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

/// Render pointer elements.
///
/// Additionally returns the ids of cursor elements for use in screencopy.
pub fn pointer_render_elements<R: PRenderer>(
    location: Point<i32, Physical>,
    scale: f64,
    renderer: &mut R,
    cursor_state: &mut CursorState,
    dnd_icon: Option<&WlSurface>,
    clock: &Clock<Monotonic>,
) -> (Vec<PointerRenderElement<R>>, Vec<Id>) {
    let integer_scale = scale.ceil() as i32;

    let pointer_elem = cursor_state.pointer_element();

    let mut pointer_elements = match &pointer_elem {
        PointerElement::Hidden => vec![],
        PointerElement::Named { cursor, size } => {
            let image = cursor.image(clock.now().into(), *size * integer_scale as u32);
            let hotspot = (image.xhot as i32, image.yhot as i32);
            let buffer = cursor_state.buffer_for_image(image, integer_scale);
            let elem = MemoryRenderBufferRenderElement::from_buffer(
                renderer,
                (location - Point::from(hotspot).downscale(integer_scale)).to_f64(),
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
                    .get::<CursorImageSurfaceData>()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .hotspot
            });

            let elems = render_elements_from_surface_tree(
                renderer,
                surface,
                location - hotspot.to_physical_precise_round(scale),
                scale,
                1.0,
                element::Kind::Cursor,
            );

            elems
        }
    };

    let cursor_ids = pointer_elements
        .iter()
        .map(|elem| elem.id())
        .cloned()
        .collect();

    if let Some(dnd_icon) = dnd_icon {
        pointer_elements.extend(AsRenderElements::render_elements(
            &smithay::desktop::space::SurfaceTree::from_surface(dnd_icon),
            renderer,
            location,
            scale.into(),
            1.0,
        ));
    }

    (pointer_elements, cursor_ids)
}
