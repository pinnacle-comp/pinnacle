use std::time::Duration;

use smithay::{
    desktop::{layer_map_for_output, utils::surface_primary_scanout_output},
    output::Output,
    utils::{Logical, Point, Rectangle},
};
use tracing::error;

use crate::{
    state::{State, WithState},
    util::transaction::{Location, TransactionBuilder},
    window::{WindowElement, window_state::LayoutMode},
};

impl State {
    pub fn update_window_geometry(
        &mut self,
        window: &WindowElement,
        output: &Output,
        geo: Option<Rectangle<i32, Logical>>,
        update_layout: bool,
    ) {
        let mode = window.with_state(|state| state.layout_mode);

        if update_layout {
            // Defer updating this window's state until the next incoming layout,
            // but ignore spilled window since they could become tiled then have
            // their geometry update.
            if !mode.is_spilled()
                && let Some(geo) = geo
            {
                self.pinnacle
                    .layout_state
                    .pending_window_updates
                    .add_for_output(output, vec![(window.clone(), geo)]);
            }
            self.pinnacle.request_layout(output);
        } else if let Some(geo) = geo {
            self.pinnacle.configure_window_if_nontiled(window);
            let mut transaction_builder = TransactionBuilder::new();
            let serial = window
                .toplevel()
                .and_then(|toplevel| toplevel.send_pending_configure());

            // if we have pending transactions, we don't want to map just yet.
            let pending = window.with_state(|s| !s.pending_transactions.is_empty());

            if serial.is_some() || pending {
                // Send a frame to get unmapped windows to update
                window.send_frame(
                    output,
                    self.pinnacle.clock.now(),
                    Some(Duration::ZERO),
                    surface_primary_scanout_output,
                );

                transaction_builder.add(
                    window,
                    Location::MapTo(geo.loc),
                    serial,
                    &self.pinnacle.loop_handle,
                );
                self.pinnacle
                    .layout_state
                    .pending_transactions
                    .add_for_output(
                        output,
                        transaction_builder.into_pending(
                            Vec::new(),
                            self.pinnacle.layout_state.pending_swap,
                            false,
                        ),
                    );
            } else {
                // No changes were needed, we can map immediately here
                self.pinnacle
                    .space
                    .map_element(window.clone(), geo.loc, false);
            }
        }
    }

    /// Updates a window's [`LayoutMode`], then either:
    /// 1. Maps the window directly if it wasn't tiled and no changes were needed,
    /// 2. Sets up a transaction if it wasn't tiled and changes were needed, or
    /// 3. Requests a layout if the layout needed a change in response to the update.
    ///
    /// Is this method pulling three duties good software design? Of course not!
    /// But I can't think of a better way to do this. PRs for refactoring welcome!
    pub fn update_window_layout_mode_and_layout(
        &mut self,
        window: &WindowElement,
        update_layout: impl FnOnce(&mut LayoutMode),
    ) {
        let _span = tracy_client::span!("State::update_window_layout_mode_and_layout");

        let output = window.output(&self.pinnacle);

        let Some(output) = output else {
            error!("Cannot update the state of a window with no output");
            return;
        };

        let Some(output_geo) = self.pinnacle.space.output_geometry(&output) else {
            error!("Cannot update the state of a window on an unmapped output");
            return;
        };

        let old_mode = window.with_state(|state| state.layout_mode);
        let mut new_mode = old_mode;
        update_layout(&mut new_mode);
        window.with_state_mut(|state| state.layout_mode = new_mode);

        let layout_needs_update = old_mode.current() != new_mode.current()
            && (old_mode.is_tiled() || new_mode.is_tiled() || new_mode.is_spilled());

        let non_exclusive_zone = layer_map_for_output(&output).non_exclusive_zone();
        let geo = self
            .pinnacle
            .compute_window_geometry(window, output_geo, non_exclusive_zone);

        if !window.is_on_active_tag() {
            return;
        }

        self.update_window_geometry(window, &output, geo, layout_needs_update);

        for output in self.pinnacle.space.outputs_for_element(window) {
            self.schedule_render(&output);
        }
    }

    pub fn move_window_to_output(&mut self, window: &WindowElement, target: Output) {
        let _span = tracy_client::span!("State::move_window_to_output");

        let current_output = window.output(&self.pinnacle);
        if current_output.as_ref() == Some(&target) {
            return;
        }

        let Some(output_geo) = self.pinnacle.space.output_geometry(&target) else {
            error!("Cannot move a window to an unmapped output.");
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

        let non_exclusive_zone = layer_map_for_output(&target).non_exclusive_zone();
        let geo = self
            .pinnacle
            .compute_window_geometry(window, output_geo, non_exclusive_zone);

        let layout_mode = window.with_state(|state| state.layout_mode);
        let layout_needs_update = layout_mode.is_tiled() || layout_mode.is_spilled();

        self.update_window_geometry(window, &target, geo, layout_needs_update);

        if let Some(output) = current_output
            && layout_mode.is_tiled()
        {
            self.pinnacle.request_layout(&output);
        }

        for output in self.pinnacle.space.outputs_for_element(window) {
            self.schedule_render(&output);
        }

        self.schedule_render(&target);
    }
}
