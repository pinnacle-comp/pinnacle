use std::{rc::Rc, time::Duration};

use smithay::{
    desktop::utils::surface_primary_scanout_output,
    output::Output,
    utils::{Logical, Rectangle},
};

use crate::{
    backend::Backend,
    state::{Pinnacle, WithState},
    util::transaction::{Location, TransactionBuilder},
};

use super::{UnmappingWindow, WindowElement};

impl Pinnacle {
    /// Unmap a window
    ///
    /// Take a screenshot of the window, and unmap it.
    /// If the window was mapped and a screenshot could be taken, the function create a new
    /// UnmappingWIndow, adds it to the z_index_stack and returns it.
    ///
    /// The window is then unmapped and all output currently displaying it are scheduled for render.
    pub fn unmap_window(
        &mut self,
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
        let loc = self.space.element_location(window);

        let unmapping = snap.zip(loc).map(|(snap, loc)| {
            let unmapping = Rc::new(UnmappingWindow {
                snapshot: snap,
                fullscreen: window.with_state(|state| state.layout_mode.is_fullscreen()),
                space_loc: loc,
            });

            let weak = Rc::downgrade(&unmapping);

            let z_index = self
                .z_index_stack
                .iter()
                .position(|z| matches!(z, crate::window::ZIndexElement::Window(w) if w == window));

            if let Some(z_index) = z_index {
                self.z_index_stack
                    .insert(z_index, crate::window::ZIndexElement::Unmapping(weak));
            };

            unmapping
        });

        let to_schedule = self.space.outputs_for_element(window);
        self.space.unmap_elem(window);
        self.loop_handle.insert_idle(move |state| {
            for output in to_schedule {
                state.schedule_render(&output);
            }
        });

        unmapping
    }

    /// Configure a window state and geometry and add it to the [`TransactionBuilder`]
    pub fn configure_window_and_add_map(
        &self,
        builder: &mut TransactionBuilder,
        window: &WindowElement,
        output: &Output,
        geo: Rectangle<i32, Logical>,
    ) {
        window.configure_states();
        window.set_pending_geo(geo.size, Some(geo.loc));

        let serial = window.configure();

        window.send_frame(
            output,
            self.clock.now(),
            Some(Duration::ZERO),
            surface_primary_scanout_output,
        );

        builder.add(window, Location::MapTo(geo.loc), serial, &self.loop_handle);
    }
}
