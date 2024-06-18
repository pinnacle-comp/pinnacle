use smithay::reexports::wayland_server::protocol::{wl_output::WlOutput, wl_surface::WlSurface};

use crate::{
    delegate_foreign_toplevel,
    protocol::foreign_toplevel::{ForeignToplevelHandler, ForeignToplevelManagerState},
    render::util::snapshot::capture_snapshots_on_output,
    state::{State, WithState},
};

impl ForeignToplevelHandler for State {
    fn foreign_toplevel_manager_state(&mut self) -> &mut ForeignToplevelManagerState {
        &mut self.pinnacle.foreign_toplevel_manager_state
    }

    fn activate(&mut self, wl_surface: WlSurface) {
        let Some(window) = self.pinnacle.window_for_surface(&wl_surface) else {
            return;
        };
        let Some(output) = window.output(&self.pinnacle) else {
            return;
        };

        if !window.is_on_active_tag() {
            let new_active_tag =
                window.with_state(|state| state.tags.iter().min_by_key(|tag| tag.id().0).cloned());
            if let Some(tag) = new_active_tag {
                let snapshots = self.backend.with_renderer(|renderer| {
                    capture_snapshots_on_output(&mut self.pinnacle, renderer, &output, [])
                });

                output.with_state(|state| {
                    if state.tags.contains(&tag) {
                        for op_tag in state.tags.iter() {
                            op_tag.set_active(false, &mut self.pinnacle);
                        }
                        tag.set_active(true, &mut self.pinnacle);
                    }
                });

                if let Some((above, below)) = snapshots {
                    output.with_state_mut(|state| {
                        state.new_wait_layout_transaction(
                            self.pinnacle.loop_handle.clone(),
                            above,
                            below,
                        )
                    });
                }
            }
        }

        output.with_state_mut(|state| state.focus_stack.set_focus(window.clone()));
        self.pinnacle.raise_window(window, true);
        self.update_keyboard_focus(&output);

        self.pinnacle.request_layout(&output);
        self.schedule_render(&output);
    }

    fn close(&mut self, wl_surface: WlSurface) {
        let Some(window) = self.pinnacle.window_for_surface(&wl_surface) else {
            return;
        };

        window.close();
    }

    fn set_fullscreen(&mut self, wl_surface: WlSurface, _wl_output: Option<WlOutput>) {
        let Some(window) = self.pinnacle.window_for_surface(&wl_surface) else {
            return;
        };

        self.set_window_fullscreen(&window, true);
    }

    fn unset_fullscreen(&mut self, wl_surface: WlSurface) {
        let Some(window) = self.pinnacle.window_for_surface(&wl_surface) else {
            return;
        };

        self.set_window_fullscreen(&window, false);
    }

    fn set_maximized(&mut self, wl_surface: WlSurface) {
        let Some(window) = self.pinnacle.window_for_surface(&wl_surface) else {
            return;
        };

        self.set_window_maximized(&window, true);
    }

    fn unset_maximized(&mut self, wl_surface: WlSurface) {
        let Some(window) = self.pinnacle.window_for_surface(&wl_surface) else {
            return;
        };

        self.set_window_maximized(&window, false);
    }

    fn set_minimized(&mut self, wl_surface: WlSurface) {
        let Some(window) = self.pinnacle.window_for_surface(&wl_surface) else {
            return;
        };

        window.with_state_mut(|state| state.minimized = true);

        let Some(output) = window.output(&self.pinnacle) else {
            return;
        };

        self.pinnacle.request_layout(&output);
        self.schedule_render(&output);
    }

    fn unset_minimized(&mut self, wl_surface: WlSurface) {
        let Some(window) = self.pinnacle.window_for_surface(&wl_surface) else {
            return;
        };

        window.with_state_mut(|state| state.minimized = false);

        let Some(output) = window.output(&self.pinnacle) else {
            return;
        };

        self.pinnacle.request_layout(&output);
        self.schedule_render(&output);
    }
}
delegate_foreign_toplevel!(State);
