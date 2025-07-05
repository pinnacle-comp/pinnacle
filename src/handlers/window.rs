use std::time::Duration;

use smithay::desktop::{layer_map_for_output, utils::surface_primary_scanout_output};
use tracing::error;

use crate::{
    state::{State, WithState},
    util::transaction::{Location, TransactionBuilder},
    window::{WindowElement, window_state::LayoutMode},
};

impl State {
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
            && (old_mode.is_tiled() || new_mode.is_tiled());

        let non_exclusive_zone = layer_map_for_output(&output).non_exclusive_zone();
        let geo = self
            .pinnacle
            .compute_window_geometry(window, output_geo, non_exclusive_zone);

        if !window.is_on_active_tag() {
            return;
        }

        if layout_needs_update {
            // Defer updating this window's state until the next incoming layout

            if let Some(geo) = geo {
                self.pinnacle
                    .layout_state
                    .pending_window_updates
                    .add_for_output(&output, vec![(window.clone(), geo)]);
            }
            self.pinnacle.request_layout(&output);
        } else if let Some(geo) = geo {
            self.pinnacle.configure_window_if_nontiled(window);
            let mut transaction_builder = TransactionBuilder::new();
            let serial = window
                .toplevel()
                .and_then(|toplevel| toplevel.send_pending_configure());

            if serial.is_some() {
                // Send a frame to get unmapped windows to update
                window.send_frame(
                    &output,
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
                        &output,
                        transaction_builder.into_pending(Vec::new(), false, false),
                    );
            } else {
                // No changes were needed, we can map immediately here
                self.pinnacle
                    .space
                    .map_element(window.clone(), geo.loc, false);
            }
        }

        for output in self.pinnacle.space.outputs_for_element(window) {
            self.schedule_render(&output);
        }
    }
}
