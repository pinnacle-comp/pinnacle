use smithay::{
    delegate_xdg_shell,
    desktop::{
        PopupKeyboardGrab, PopupKind, PopupPointerGrab, PopupUngrabStrategy, Window,
        WindowSurfaceType, find_popup_root_surface, layer_map_for_output,
    },
    input::{Seat, pointer::Focus},
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge,
        wayland_server::protocol::{wl_output::WlOutput, wl_seat::WlSeat},
    },
    utils::Serial,
    wayland::shell::xdg::{
        PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
    },
};
use tracing::warn;

use crate::{
    api::signal::Signal,
    focus::keyboard::KeyboardFocusTarget,
    state::{State, WithState},
    window::{
        Unmapped, UnmappedState, WindowElement, rules::ClientRequests,
        window_state::FullscreenOrMaximized,
    },
};

impl XdgShellHandler for State {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.pinnacle.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let _span = tracy_client::span!("XdgShellHandler::new_toplevel");

        let window = WindowElement::new(Window::new_wayland_window(surface.clone()));

        let handle = self
            .pinnacle
            .foreign_toplevel_list_state
            .new_toplevel::<State>(
                // These will most likely be empty at this point
                window.title().unwrap_or_default(),
                window.class().unwrap_or_default(),
            );
        window.with_state_mut(|state| {
            assert!(state.foreign_toplevel_list_handle.is_none());
            state.foreign_toplevel_list_handle = Some(handle);
        });

        // Gets wleird-slow-ack-configure working
        // surface.with_pending_state(|state| {
        //     state.size = Some((600, 400).into());
        // });

        self.pinnacle.unmapped_windows.push(Unmapped {
            window,
            activation_token_data: None,
            state: UnmappedState::WaitingForTags {
                client_requests: ClientRequests::default(),
            },
        });
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        let _span = tracy_client::span!("XdgShellHandler::toplevel_destroyed");

        let Some(window) = self
            .pinnacle
            .window_for_surface(surface.wl_surface())
            .cloned()
        else {
            return;
        };

        let is_tiled = window.with_state(|state| state.layout_mode.is_tiled());

        let output = window.output(&self.pinnacle);

        if let Some(output) = output.as_ref() {
            self.backend.with_renderer(|renderer| {
                window.capture_snapshot_and_store(
                    renderer,
                    output.current_scale().fractional_scale().into(),
                    1.0,
                );
            });
        }

        self.pinnacle.remove_window(&window, false);

        if let Some(output) = output {
            if is_tiled {
                self.pinnacle.request_layout(&output);
            }

            self.schedule_render(&output);
        }
    }

    fn new_popup(&mut self, surface: PopupSurface, _positioner: PositionerState) {
        let _span = tracy_client::span!("XdgShellHandler::new_popup");

        self.pinnacle.position_popup(&surface);

        if let Err(err) = self
            .pinnacle
            .popup_manager
            .track_popup(PopupKind::from(surface))
        {
            tracing::warn!("failed to track popup: {}", err);
        }
    }

    fn popup_destroyed(&mut self, _surface: PopupSurface) {
        let _span = tracy_client::span!("XdgShellHandler::popup_destroyed");

        // TODO: only schedule on the outputs the popup is on
        for output in self.pinnacle.space.outputs().cloned().collect::<Vec<_>>() {
            self.schedule_render(&output);
        }
    }

    fn move_request(&mut self, surface: ToplevelSurface, seat: WlSeat, serial: Serial) {
        let _span = tracy_client::span!("XdgShellHandler::move_request");

        self.move_request_client(
            surface.wl_surface(),
            &Seat::from_resource(&seat).expect("couldn't get seat from WlSeat"),
            serial,
        );
    }

    fn resize_request(
        &mut self,
        surface: ToplevelSurface,
        seat: WlSeat,
        serial: Serial,
        edges: ResizeEdge,
    ) {
        let _span = tracy_client::span!("XdgShellHandler::resize_request");

        const BUTTON_LEFT: u32 = 0x110;
        self.resize_request_client(
            surface.wl_surface(),
            &Seat::from_resource(&seat).expect("couldn't get seat from WlSeat"),
            serial,
            edges.into(),
            BUTTON_LEFT,
        );
    }

    fn reposition_request(
        &mut self,
        surface: PopupSurface,
        positioner: PositionerState,
        token: u32,
    ) {
        let _span = tracy_client::span!("XdgShellHandler::reposition_request");

        surface.with_pending_state(|state| {
            state.geometry = positioner.get_geometry();
            state.positioner = positioner;
        });
        self.pinnacle.position_popup(&surface);
        surface.send_repositioned(token);
    }

    fn grab(&mut self, surface: PopupSurface, seat: WlSeat, serial: Serial) {
        let _span = tracy_client::span!("XdgShellHandler::grab");

        let seat: Seat<Self> = Seat::from_resource(&seat).expect("couldn't get seat from WlSeat");
        let popup_kind = PopupKind::Xdg(surface);

        let Some(root) = find_popup_root_surface(&popup_kind).ok().and_then(|root| {
            self.pinnacle
                .window_for_surface(&root)
                .cloned()
                .map(KeyboardFocusTarget::Window)
                .or_else(|| {
                    self.pinnacle.space.outputs().find_map(|op| {
                        layer_map_for_output(op)
                            .layer_for_surface(&root, WindowSurfaceType::TOPLEVEL)
                            .cloned()
                            .map(KeyboardFocusTarget::LayerSurface)
                    })
                })
        }) else {
            return;
        };

        let mut grab = match self
            .pinnacle
            .popup_manager
            .grab_popup(root, popup_kind, &seat, serial)
        {
            Ok(grab) => grab,
            Err(err) => {
                warn!("Failed to grab popup: {err}");
                return;
            }
        };

        if let Some(keyboard) = seat.get_keyboard() {
            if keyboard.is_grabbed()
                && !(keyboard.has_grab(serial)
                    || keyboard.has_grab(grab.previous_serial().unwrap_or(serial)))
            {
                grab.ungrab(PopupUngrabStrategy::All);
                return;
            }

            keyboard.set_focus(self, grab.current_grab(), serial);
            keyboard.set_grab(self, PopupKeyboardGrab::new(&grab), serial);
        }

        if let Some(pointer) = seat.get_pointer() {
            if pointer.is_grabbed()
                && !(pointer.has_grab(serial)
                    || pointer.has_grab(grab.previous_serial().unwrap_or_else(|| grab.serial())))
            {
                grab.ungrab(PopupUngrabStrategy::All);
                return;
            }
            pointer.set_grab(self, PopupPointerGrab::new(&grab), serial, Focus::Keep);
        }
    }

    fn fullscreen_request(&mut self, surface: ToplevelSurface, wl_output: Option<WlOutput>) {
        let _span = tracy_client::span!("XdgShellHandler::fullscreen_request");

        let requested_output = wl_output.and_then(|wl_output| {
            self.pinnacle
                .outputs
                .iter()
                .find(|output| output.owns(&wl_output))
                .filter(|output| {
                    output.with_state(|state| !state.tags.is_empty())
                        && self.pinnacle.space.output_geometry(output).is_some()
                })
                .cloned()
        });

        if let Some(window) = self
            .pinnacle
            .window_for_surface(surface.wl_surface())
            .cloned()
        {
            let mut geometry_only = false;

            window.with_state_mut(|state| state.need_configure = true);

            if window.output(&self.pinnacle) != requested_output
                && let Some(output) = requested_output
            {
                self.pinnacle.move_window_to_output(&window, output);

                geometry_only = window.with_state(|state| state.layout_mode.is_fullscreen());
            }

            if geometry_only {
                self.pinnacle.update_window_geometry(&window, false);
            } else {
                self.pinnacle
                    .update_window_layout_mode(&window, |mode| mode.set_client_fullscreen(true));
            }
        } else if let Some(unmapped) = self
            .pinnacle
            .unmapped_window_for_surface_mut(surface.wl_surface())
        {
            if let Some(output) = requested_output {
                unmapped.window.set_tags_to_output(&output);
            }

            match &mut unmapped.state {
                UnmappedState::WaitingForTags { client_requests } => {
                    client_requests.layout_mode = Some(FullscreenOrMaximized::Fullscreen);
                }
                UnmappedState::WaitingForRules {
                    rules: _,
                    client_requests,
                } => {
                    client_requests.layout_mode = Some(FullscreenOrMaximized::Fullscreen);
                }
                UnmappedState::PostInitialConfigure {
                    attempt_float_on_map,
                    ..
                } => {
                    // guys i think some of these methods borrowing all of pinnacle isn't good
                    let window = unmapped.window.clone();
                    window.with_state_mut(|state| state.layout_mode.set_client_fullscreen(true));
                    *attempt_float_on_map = false;
                    self.pinnacle.configure_window_if_nontiled(&window);
                    window.toplevel().expect("in xdgshell").send_configure();
                }
            }
        }
    }

    fn unfullscreen_request(&mut self, surface: ToplevelSurface) {
        let _span = tracy_client::span!("XdgShellHandler::unfullscreen_request");

        if let Some(window) = self
            .pinnacle
            .window_for_surface(surface.wl_surface())
            .cloned()
        {
            window.with_state_mut(|state| state.need_configure = true);
            self.pinnacle
                .update_window_layout_mode(&window, |layout_mode| {
                    layout_mode.set_client_fullscreen(false);
                });
        } else if let Some(unmapped) = self
            .pinnacle
            .unmapped_window_for_surface_mut(surface.wl_surface())
        {
            match &mut unmapped.state {
                UnmappedState::WaitingForTags { client_requests } => {
                    client_requests
                        .layout_mode
                        .take_if(|mode| matches!(mode, FullscreenOrMaximized::Fullscreen));
                }
                UnmappedState::WaitingForRules {
                    rules: _,
                    client_requests,
                } => {
                    client_requests
                        .layout_mode
                        .take_if(|mode| matches!(mode, FullscreenOrMaximized::Fullscreen));
                }
                UnmappedState::PostInitialConfigure { .. } => {
                    let window = unmapped.window.clone();
                    window.with_state_mut(|state| state.layout_mode.set_client_fullscreen(false));
                    self.pinnacle.configure_window_if_nontiled(&window);
                    window.toplevel().expect("in xdgshell").send_configure();
                }
            }
        }
    }

    fn maximize_request(&mut self, surface: ToplevelSurface) {
        let _span = tracy_client::span!("XdgShellHandler::maximize_request");

        if let Some(window) = self
            .pinnacle
            .window_for_surface(surface.wl_surface())
            .cloned()
        {
            window.with_state_mut(|state| state.need_configure = true);
            self.pinnacle
                .update_window_layout_mode(&window, |mode| mode.set_client_maximized(true));
        } else if let Some(unmapped) = self
            .pinnacle
            .unmapped_window_for_surface_mut(surface.wl_surface())
        {
            match &mut unmapped.state {
                UnmappedState::WaitingForTags { client_requests } => {
                    client_requests.layout_mode = Some(FullscreenOrMaximized::Maximized);
                }
                UnmappedState::WaitingForRules {
                    rules: _,
                    client_requests,
                } => {
                    client_requests.layout_mode = Some(FullscreenOrMaximized::Maximized);
                }
                UnmappedState::PostInitialConfigure {
                    attempt_float_on_map,
                    ..
                } => {
                    let window = unmapped.window.clone();
                    window.with_state_mut(|state| state.layout_mode.set_client_maximized(true));
                    *attempt_float_on_map = false;
                    self.pinnacle.configure_window_if_nontiled(&window);
                    window.toplevel().expect("in xdgshell").send_configure();
                }
            }
        }
    }

    fn unmaximize_request(&mut self, surface: ToplevelSurface) {
        let _span = tracy_client::span!("XdgShellHandler::unmaximize_request");

        if let Some(window) = self
            .pinnacle
            .window_for_surface(surface.wl_surface())
            .cloned()
        {
            window.with_state_mut(|state| state.need_configure = true);

            self.pinnacle
                .update_window_layout_mode(&window, |mode| mode.set_client_maximized(false));
        } else if let Some(unmapped) = self
            .pinnacle
            .unmapped_window_for_surface_mut(surface.wl_surface())
        {
            match &mut unmapped.state {
                UnmappedState::WaitingForTags { client_requests } => {
                    client_requests.layout_mode = Some(FullscreenOrMaximized::Maximized);
                }
                UnmappedState::WaitingForRules {
                    rules: _,
                    client_requests,
                } => {
                    client_requests.layout_mode = Some(FullscreenOrMaximized::Maximized);
                }
                UnmappedState::PostInitialConfigure { .. } => {
                    let window = unmapped.window.clone();
                    window.with_state_mut(|state| state.layout_mode.set_client_maximized(false));
                    self.pinnacle.configure_window_if_nontiled(&window);
                    window.toplevel().expect("in xdgshell").send_configure();
                }
            }
        }
    }

    fn minimize_request(&mut self, _surface: ToplevelSurface) {
        // TODO:
        // if let Some(window) = self.window_for_surface(surface.wl_surface()) {
        //     self.space.unmap_elem(&window);
        // }
    }

    fn app_id_changed(&mut self, surface: ToplevelSurface) {
        let Some(window) = self.pinnacle.window_for_surface(surface.wl_surface()) else {
            return;
        };
        let app_id = window.class().unwrap_or_default();
        window.with_state(|state| {
            if let Some(handle) = state.foreign_toplevel_list_handle.as_ref() {
                handle.send_app_id(&app_id);
                handle.send_done();
            }
        });
    }

    fn title_changed(&mut self, surface: ToplevelSurface) {
        let Some(window) = self
            .pinnacle
            .window_for_surface(surface.wl_surface())
            .cloned()
        else {
            return;
        };

        self.pinnacle
            .signal_state
            .window_title_changed
            .signal(&window);

        let title = window.title().unwrap_or_default();
        window.with_state(|state| {
            if let Some(handle) = state.foreign_toplevel_list_handle.as_ref() {
                handle.send_title(&title);
                handle.send_done();
            }
        });
    }
}
delegate_xdg_shell!(State);
