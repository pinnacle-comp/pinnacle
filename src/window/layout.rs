use std::{rc::Rc, time::Duration};

use smithay::{
    desktop::{layer_map_for_output, utils::surface_primary_scanout_output},
    output::Output,
    utils::{Logical, Point, Rectangle},
};

use crate::{
    backend::Backend,
    state::{Pinnacle, WithState},
    util::transaction::{Location, TransactionBuilder},
    window::window_state::LayoutMode,
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

    /// Create a transaction to map the window.
    pub fn map_window_to(&mut self, window: &WindowElement, loc: Point<i32, Logical>) {
        if let Some(output) = window.output(self) {
            let mut transaction_builder = TransactionBuilder::new();
            let serial = window.configure();

            if serial.is_some() {
                window.send_frame(
                    &output,
                    self.clock.now(),
                    Some(Duration::ZERO),
                    surface_primary_scanout_output,
                );
            }

            transaction_builder.add(window, Location::MapTo(loc), serial, &self.loop_handle);

            self.layout_state.pending_transactions.add_for_output(
                &output,
                transaction_builder.into_pending(Vec::new(), self.layout_state.pending_swap, false),
            );
        }
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

    /// Configure a window state and geometry, then schedule mapping it.
    pub fn configure_window_and_map(
        &mut self,
        window: &WindowElement,
        output: &Output,
        geo: Rectangle<i32, Logical>,
    ) {
        let mut builder = TransactionBuilder::new();

        self.configure_window_and_add_map(&mut builder, window, output, geo);

        self.layout_state
            .pending_transactions
            .add_for_output(output, builder.into_pending(Vec::new(), false, false));
    }

    /// Compute a new geometry and applies it.
    ///
    /// If need_layout is true a new layout is requested. The computed geometry (if any) will be
    /// applied at the same time the new layout is.
    pub fn update_window_geometry(&mut self, window: &WindowElement, need_layout: bool) {
        let _span = tracy_client::span!("Pinnacle::update_window_geometry");
        let output = window.output(self);

        let Some(output) = output else {
            tracing::error!("Cannot update the state of a window with no output");
            window.configure();
            return;
        };

        let Some(output_geo) = self.space.output_geometry(&output) else {
            tracing::error!("Cannot update the state of a window on an unmapped output");
            window.configure();
            return;
        };

        let mode = window.with_state(|state| state.layout_mode);

        let non_exclusive_zone = layer_map_for_output(&output).non_exclusive_zone();
        let geo = self.compute_window_geometry(window, output_geo, non_exclusive_zone);

        if !window.is_on_active_tag() {
            if let Some(geo) = geo {
                window.set_pending_geo(geo.size, Some(geo.loc));
            }
            window.configure();
            return;
        }

        if need_layout {
            if geo.is_some() && !mode.is_spilled() {
                self.layout_state
                    .pending_window_updates
                    .add_for_output(&output, vec![(window.clone(), geo.unwrap())]);
            }

            self.request_layout(&output);
        } else if let Some(geo) = geo {
            self.configure_window_and_map(window, &output, geo);
        } else {
            window.configure();
        }
    }

    /// Update the window [`LayoutMode`].
    ///
    /// If the layout_mode changed the window geometry will be re-computed with a call to
    /// [`Pinnacle::update_window_geometry`].
    pub fn update_window_layout_mode(
        &mut self,
        window: &WindowElement,
        update_layout: impl FnOnce(&mut LayoutMode),
    ) {
        let _span = tracy_client::span!("Pinnacle::update_window_layout_mode");
        let output = window.output(self);

        let Some(output) = output else {
            tracing::error!("Cannot update the state of a window with no output");
            window.configure();
            return;
        };

        if self.space.output_geometry(&output).is_none() {
            tracing::error!("Cannot update the state of a window on an unmapped output");
            window.configure();
            return;
        };

        let old_mode = window.with_state(|state| state.layout_mode);
        let mut new_mode = old_mode;
        update_layout(&mut new_mode);
        window.with_state_mut(|state| state.layout_mode = new_mode);

        if old_mode != new_mode {
            let need_layout = old_mode.is_tiled() || new_mode.is_tiled() || new_mode.is_spilled();

            window.configure_states();
            self.update_window_geometry(window, need_layout);
        } else {
            window.configure();
        }
    }

    /// Move a window to a new output.
    ///
    /// The move itself is done by updating the window tags.
    ///
    /// This function additionally fixup the window floating location to prevent the window from
    /// 'jumping back' if it becomes tiled or spilled.
    ///
    /// The function will request a new layout if the window was in a tiled state.
    ///
    /// [`Pinnacle::update_window_geometry`] or [`Pinnacle::update_window_layout_mode`] should be
    /// called for the move to be effective.
    pub fn move_window_to_output(&mut self, window: &WindowElement, target: Output) {
        let _span = tracy_client::span!("Pinnacle::move_window_to_output");

        let current_output = window.output(self);
        if current_output.as_ref() == Some(&target) {
            return;
        }

        if self.space.output_geometry(&target).is_none() {
            tracing::error!("Cannot move a window to an unmapped output.");
            return;
        };

        window.set_tags_to_output(&target);

        // Reset the floating loc since we're changing output.
        let output_loc = target.current_location();

        let Rectangle { mut loc, size } = layer_map_for_output(&target).non_exclusive_zone();

        // Slightly offset the location so the window is not jammed in a corner
        let offset = {
            let (w, h) = size.downscale(100).into();
            i32::min(w, h)
        };

        loc += output_loc + Point::new(offset, offset);

        window.with_state_mut(|state| state.set_floating_loc(Some(loc)));

        let layout_mode = window.with_state(|state| state.layout_mode);

        if let Some(output) = current_output
            && layout_mode.is_tiled()
        {
            self.request_layout(&output);
        }
    }
}
