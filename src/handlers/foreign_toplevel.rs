use smithay::reexports::wayland_server::protocol::{wl_output::WlOutput, wl_surface::WlSurface};

use crate::{
    api::signal::Signal,
    delegate_foreign_toplevel,
    protocol::foreign_toplevel::{ForeignToplevelHandler, ForeignToplevelManagerState},
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
            let new_active_tag = window.with_state(|state| {
                state
                    .tags
                    .iter()
                    .min_by_key(|tag| tag.id().to_inner())
                    .cloned()
            });

            if let Some(tag) = new_active_tag {
                self.capture_snapshots_on_output(&output, []);

                // FIXME: use `crate::api:tag::switch_to`
                output.with_state(|state| {
                    if state.tags.contains(&tag) {
                        for op_tag in state.tags.iter() {
                            if op_tag.set_active(false) {
                                self.pinnacle.signal_state.tag_active.signal(op_tag);
                            }
                        }
                        if tag.set_active(true) {
                            self.pinnacle.signal_state.tag_active.signal(&tag);
                        }
                    }
                });

                self.pinnacle.begin_layout_transaction(&output);
                self.pinnacle.request_layout(&output);
            }
        }

        output.with_state_mut(|state| state.focus_stack.set_focus(window.clone()));
        self.pinnacle.raise_window(window, true);
        self.update_keyboard_focus(&output);

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

        crate::api::window::set_fullscreen(self, &window, true);
    }

    fn unset_fullscreen(&mut self, wl_surface: WlSurface) {
        let Some(window) = self.pinnacle.window_for_surface(&wl_surface) else {
            return;
        };

        crate::api::window::set_fullscreen(self, &window, false);
    }

    fn set_maximized(&mut self, wl_surface: WlSurface) {
        let Some(window) = self.pinnacle.window_for_surface(&wl_surface) else {
            return;
        };

        crate::api::window::set_maximized(self, &window, true);
    }

    fn unset_maximized(&mut self, wl_surface: WlSurface) {
        let Some(window) = self.pinnacle.window_for_surface(&wl_surface) else {
            return;
        };

        crate::api::window::set_maximized(self, &window, false);
    }

    // TODO:
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

    // TODO:
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
