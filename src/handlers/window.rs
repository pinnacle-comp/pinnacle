use smithay::utils::Point;

use crate::{
    state::{State, WithState},
    window::WindowElement,
};

impl State {
    pub fn update_window_state_and_layout(&mut self, window: &WindowElement) {
        let _span = tracy_client::span!("State::update_window_state_and_layout");

        let output = window.output(&self.pinnacle);
        if let Some(output) = output.as_ref() {
            self.capture_snapshots_on_output(output, [window.clone()]);

            let output_geo = self.pinnacle.space.output_geometry(output);
            if let Some(output_geo) = output_geo {
                let mut size = window.with_state(|state| state.floating_size);
                if size.is_empty() {
                    size = window.geometry().size;
                }
                let loc = window
                    .with_state(|state| state.floating_loc)
                    .or_else(|| {
                        self.pinnacle
                            .space
                            .element_location(window)
                            .map(|loc| loc.to_f64())
                    })
                    .unwrap_or_else(|| {
                        let centered_loc = Point::from((
                            output_geo.loc.x + output_geo.size.w / 2 - size.w / 2,
                            output_geo.loc.y + output_geo.size.h / 2 - size.h / 2,
                        ));
                        centered_loc.to_f64()
                    });
                window.with_state_mut(|state| {
                    state.floating_loc = Some(loc);
                });
            }
        }

        self.pinnacle.configure_window_if_nontiled(window);
        if let Some(toplevel) = window.toplevel() {
            toplevel.send_configure();
        }

        if window.with_state(|state| state.layout_mode.is_floating()) && window.is_on_active_tag() {
            if let Some(floating_loc) = window.with_state(|state| state.floating_loc) {
                self.pinnacle
                    .space
                    .map_element(window.clone(), floating_loc.to_i32_round(), false);
            }
        }

        if let Some(output) = window.output(&self.pinnacle) {
            self.pinnacle.begin_layout_transaction(&output);
            self.pinnacle.request_layout(&output);
        }

        for output in self.pinnacle.space.outputs_for_element(window) {
            self.schedule_render(&output);
        }
    }
}
