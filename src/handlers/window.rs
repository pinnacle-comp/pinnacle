use crate::{state::State, window::WindowElement};

impl State {
    pub fn update_window_state_and_layout(&mut self, window: &WindowElement) {
        let _span = tracy_client::span!("State::update_window_state_and_layout");

        let output = window.output(&self.pinnacle);
        if let Some(output) = output.as_ref() {
            self.capture_snapshots_on_output(output, [window.clone()]);
        }

        self.pinnacle.configure_window_if_nontiled(window);
        if let Some(toplevel) = window.toplevel() {
            toplevel.send_configure();
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
