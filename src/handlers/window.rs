use crate::{
    render::util::snapshot::capture_snapshots_on_output,
    state::{State, WithState},
    window::WindowElement,
};

impl State {
    pub fn set_window_maximized(&mut self, window: &WindowElement, maximized: bool) {
        let snapshots = window.output(&self.pinnacle).map(|output| {
            self.backend.with_renderer(|renderer| {
                capture_snapshots_on_output(&mut self.pinnacle, renderer, &output, [window.clone()])
            })
        });

        if maximized {
            if !window.with_state(|state| state.fullscreen_or_maximized.is_maximized()) {
                window.toggle_maximized();
            }
        } else if window.with_state(|state| state.fullscreen_or_maximized.is_maximized()) {
            window.toggle_maximized();
        }

        if let Some(output) = window.output(&self.pinnacle) {
            if let Some((fs_and_up_snapshots, under_fs_snapshots)) = snapshots {
                output.with_state_mut(|op_state| {
                    op_state.new_wait_layout_transaction(
                        self.pinnacle.loop_handle.clone(),
                        fs_and_up_snapshots,
                        under_fs_snapshots,
                    )
                });
            }

            self.pinnacle.request_layout(&output);
            self.schedule_render(&output);
        }
    }

    pub fn set_window_fullscreen(&mut self, window: &WindowElement, fullscreen: bool) {
        let snapshots = window.output(&self.pinnacle).map(|output| {
            self.backend.with_renderer(|renderer| {
                capture_snapshots_on_output(&mut self.pinnacle, renderer, &output, [window.clone()])
            })
        });

        if fullscreen {
            if !window.with_state(|state| state.fullscreen_or_maximized.is_fullscreen()) {
                window.toggle_fullscreen();
            }
        } else if window.with_state(|state| state.fullscreen_or_maximized.is_fullscreen()) {
            window.toggle_fullscreen();
        }

        if let Some(output) = window.output(&self.pinnacle) {
            if let Some((fs_and_up_snapshots, under_fs_snapshots)) = snapshots {
                output.with_state_mut(|op_state| {
                    op_state.new_wait_layout_transaction(
                        self.pinnacle.loop_handle.clone(),
                        fs_and_up_snapshots,
                        under_fs_snapshots,
                    )
                });
            }

            self.pinnacle.request_layout(&output);
            self.schedule_render(&output);
        }
    }
}
