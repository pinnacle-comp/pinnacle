use std::time::Duration;

use smithay::{
    desktop::{layer_map_for_output, utils::surface_primary_scanout_output},
    reexports::wayland_protocols::xdg::shell::server::xdg_positioner::{
        Anchor, ConstraintAdjustment, Gravity,
    },
    utils::Rectangle,
    wayland::shell::xdg::PositionerState,
};
use tracing::error;

use crate::{
    state::{State, WithState},
    util::transaction::TransactionBuilder,
    window::{
        window_state::{LayoutMode, LayoutModeKind},
        WindowElement,
    },
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

        let geo = match window.with_state(|state| state.layout_mode.current()) {
            LayoutModeKind::Tiled => None,
            LayoutModeKind::Floating => {
                let mut size = window.with_state(|state| state.floating_size);
                if size.is_empty() {
                    size = window.geometry().size;
                }

                let mut working_output_geo = layer_map_for_output(&output).non_exclusive_zone();
                working_output_geo.loc += output_geo.loc;

                let center_rect = self
                    .pinnacle
                    .parent_window_for(window)
                    .and_then(|parent| self.pinnacle.space.element_geometry(parent))
                    .unwrap_or(working_output_geo);

                let set_x = window.with_state(|state| state.floating_x);
                let set_y = window.with_state(|state| state.floating_y);

                let floating_loc = window
                    .with_state(|state| state.floating_loc())
                    .or_else(|| self.pinnacle.space.element_location(window))
                    .unwrap_or_else(|| {
                        // Attempt to center the window within its parent.
                        // If it has no parent, center it within the non-exclusive zone of its output.
                        //
                        // We use a positioner to slide the window so that it isn't off screen.

                        let positioner = PositionerState {
                            rect_size: size,
                            anchor_rect: center_rect,
                            anchor_edges: Anchor::None,
                            gravity: Gravity::None,
                            constraint_adjustment: ConstraintAdjustment::SlideX
                                | ConstraintAdjustment::SlideY,
                            offset: (0, 0).into(),
                            ..Default::default()
                        };

                        positioner
                            .get_unconstrained_geometry(working_output_geo)
                            .loc
                    });

                window.with_state_mut(|state| {
                    state.floating_x = Some(set_x.unwrap_or(floating_loc.x));
                    state.floating_y = Some(set_y.unwrap_or(floating_loc.y));
                    state.floating_size = size;
                });

                Some(Rectangle::new(floating_loc, size))
            }
            LayoutModeKind::Maximized => {
                let mut non_exclusive_geo = layer_map_for_output(&output).non_exclusive_zone();
                non_exclusive_geo.loc += output_geo.loc;
                Some(non_exclusive_geo)
            }
            LayoutModeKind::Fullscreen => Some(output_geo),
        };

        if !window.is_on_active_tag() {
            return;
        }

        if layout_needs_update {
            // Defer updating this window's state until the next incoming layout

            if let Some(geo) = geo {
                self.pinnacle
                    .layout_state
                    .pending_latched
                    .add_for_output(&output, vec![(window.clone(), geo)]);
            }
            self.pinnacle.request_layout(&output);
        } else if let Some(geo) = geo {
            self.pinnacle.configure_window_if_nontiled(window);
            let mut transaction_builder = TransactionBuilder::new(false);
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

                transaction_builder.add(window, geo.loc, serial, &self.pinnacle.loop_handle);
                self.pinnacle
                    .layout_state
                    .pending_transactions
                    .entry(output.downgrade())
                    .or_default()
                    .push(transaction_builder.into_pending(Vec::new()));
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
