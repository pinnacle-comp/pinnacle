use std::rc::Rc;

use smithay::output::Output;

use crate::{
    backend::Backend,
    state::{Pinnacle, WithState},
};

use super::{UnmappingWindow, WindowElement};

/// Unmap a window
///
/// Take a screenshot of the window, and unmap it.
/// If the window was mapped and a screenshot could be taken, the function create a new
/// UnmappingWIndow, adds it to the z_index_stack and returns it.
///
/// The window is then unmapped and all output currently displaying it are scheduled for render.
pub fn unmap_window(
    pinnacle: &mut Pinnacle,
    backend: &mut Backend,
    window: &WindowElement,
    output: &Output,
) -> Option<Rc<UnmappingWindow>> {
    backend.with_renderer(|renderer| {
        window.capture_snapshot_and_store(
            renderer,
            output.current_scale().fractional_scale().into(),
            1.0,
        );
    });

    let snap = window.with_state_mut(|state| state.snapshot.take());
    let loc = pinnacle.space.element_location(window);

    let unmapping = snap.zip(loc).map(|(snap, loc)| {
        let unmapping = Rc::new(UnmappingWindow {
            snapshot: snap,
            fullscreen: window.with_state(|state| state.layout_mode.is_fullscreen()),
            space_loc: loc,
        });

        let weak = Rc::downgrade(&unmapping);

        let z_index = pinnacle
            .z_index_stack
            .iter()
            .position(|z| matches!(z, crate::window::ZIndexElement::Window(w) if w == window));

        if let Some(z_index) = z_index {
            pinnacle
                .z_index_stack
                .insert(z_index, crate::window::ZIndexElement::Unmapping(weak));
        };

        unmapping
    });

    let to_schedule = pinnacle.space.outputs_for_element(window);
    pinnacle.space.unmap_elem(window);
    pinnacle.loop_handle.insert_idle(move |state| {
        for output in to_schedule {
            state.schedule_render(&output);
        }
    });

    unmapping
}
