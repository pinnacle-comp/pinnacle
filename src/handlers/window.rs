use crate::{state::State, window::WindowElement};

impl State {
    pub fn set_window_maximized_and_layout(&mut self, window: &WindowElement, maximized: bool) {
        let output = window.output(&self.pinnacle);
        if let Some(output) = output.as_ref() {
            self.capture_snapshots_on_output(output, [window.clone()]);
        }

        self.pinnacle.set_window_maximized(window, maximized);

        if let Some(output) = output {
            self.pinnacle.begin_layout_transaction(&output);
            self.pinnacle.request_layout(&output);

            self.schedule_render(&output);
        }
    }

    pub fn set_window_fullscreen_and_layout(&mut self, window: &WindowElement, fullscreen: bool) {
        let output = window.output(&self.pinnacle);
        if let Some(output) = output.as_ref() {
            self.capture_snapshots_on_output(output, [window.clone()]);
        }

        self.pinnacle.set_window_fullscreen(window, fullscreen);

        if let Some(output) = window.output(&self.pinnacle) {
            self.pinnacle.begin_layout_transaction(&output);
            self.pinnacle.request_layout(&output);

            self.schedule_render(&output);
        }
    }
}
