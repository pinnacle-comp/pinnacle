use smithay::{
    delegate_xdg_shell,
    desktop::{
        find_popup_root_surface, layer_map_for_output, PopupKeyboardGrab, PopupKind,
        PopupPointerGrab, PopupUngrabStrategy, Window, WindowSurfaceType,
    },
    input::{pointer::Focus, Seat},
    output::Output,
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel::{self, ResizeEdge},
        wayland_server::{
            protocol::{wl_output::WlOutput, wl_seat::WlSeat, wl_surface::WlSurface},
            DisplayHandle, Resource,
        },
    },
    utils::Serial,
    wayland::{
        compositor::{self, BufferAssignment, SurfaceAttributes},
        shell::xdg::{
            PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
        },
    },
};

use crate::{
    focus::keyboard::KeyboardFocusTarget,
    state::{State, WithState},
    window::WindowElement,
};

impl XdgShellHandler for State {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.pinnacle.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let _span = tracy_client::span!("XdgShellHandler::new_toplevel");

        surface.with_pending_state(|state| {
            // state.size = Some((600, 400).into()); // gets wleird-slow-ack working
            state.states.set(xdg_toplevel::State::TiledTop);
            state.states.set(xdg_toplevel::State::TiledBottom);
            state.states.set(xdg_toplevel::State::TiledLeft);
            state.states.set(xdg_toplevel::State::TiledRight);
        });

        let window = WindowElement::new(Window::new_wayland_window(surface.clone()));
        self.pinnacle.unmapped_windows.push(window);
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        let _span = tracy_client::span!("XdgShellHandler::toplevel_destroyed");

        let Some(window) = self.pinnacle.window_for_surface(surface.wl_surface()) else {
            return;
        };

        let output = window.output(&self.pinnacle);

        if let Some(output) = output.as_ref() {
            self.capture_snapshots_on_output(output, []);
        }

        self.pinnacle.remove_window(&window, false);

        if let Some(output) = output {
            self.pinnacle.begin_layout_transaction(&output);
            self.pinnacle.request_layout(&output);

            self.update_keyboard_focus(&output);
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
        if let Some(root) = find_popup_root_surface(&popup_kind).ok().and_then(|root| {
            self.pinnacle
                .window_for_surface(&root)
                .map(KeyboardFocusTarget::Window)
                .or_else(|| {
                    self.pinnacle.space.outputs().find_map(|op| {
                        layer_map_for_output(op)
                            .layer_for_surface(&root, WindowSurfaceType::TOPLEVEL)
                            .cloned()
                            .map(KeyboardFocusTarget::LayerSurface)
                    })
                })
        }) {
            if let Ok(mut grab) = self
                .pinnacle
                .popup_manager
                .grab_popup(root, popup_kind, &seat, serial)
            {
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
                            || pointer
                                .has_grab(grab.previous_serial().unwrap_or_else(|| grab.serial())))
                    {
                        grab.ungrab(PopupUngrabStrategy::All);
                        return;
                    }
                    pointer.set_grab(self, PopupPointerGrab::new(&grab), serial, Focus::Keep);
                }
            }
        }
    }

    fn fullscreen_request(&mut self, surface: ToplevelSurface, mut wl_output: Option<WlOutput>) {
        let _span = tracy_client::span!("XdgShellHandler::fullscreen_request");

        if !surface
            .current_state()
            .capabilities
            .contains(xdg_toplevel::WmCapabilities::Fullscreen)
        {
            return;
        }

        let wl_surface = surface.wl_surface();
        let output = wl_output
            .as_ref()
            .and_then(Output::from_resource)
            .or_else(|| {
                self.pinnacle
                    .window_for_surface(wl_surface)
                    .and_then(|window| {
                        self.pinnacle
                            .space
                            .outputs_for_element(&window)
                            .first()
                            .cloned()
                    })
            });

        if let Some(output) = output {
            let Some(geometry) = self.pinnacle.space.output_geometry(&output) else {
                surface.send_configure();
                return;
            };

            let client = self
                .pinnacle
                .display_handle
                .get_client(wl_surface.id())
                .expect("wl_surface had no client");
            for output in output.client_outputs(&client) {
                wl_output = Some(output);
            }

            surface.with_pending_state(|state| {
                state.size = Some(geometry.size);
                state.fullscreen_output = wl_output;
            });

            let Some(window) = self.pinnacle.window_for_surface(wl_surface) else {
                return;
            };

            window.with_state_mut(|state| state.window_state.set_fullscreen(true));
            self.update_window_state_and_layout(&window);
        }

        surface.send_configure();
    }

    fn unfullscreen_request(&mut self, surface: ToplevelSurface) {
        let _span = tracy_client::span!("XdgShellHandler::unfullscreen_request");

        surface.with_pending_state(|state| {
            state.fullscreen_output.take();
        });

        let Some(window) = self.pinnacle.window_for_surface(surface.wl_surface()) else {
            return;
        };

        window.with_state_mut(|state| state.window_state.set_fullscreen(false));
        self.update_window_state_and_layout(&window);
    }

    fn maximize_request(&mut self, surface: ToplevelSurface) {
        let _span = tracy_client::span!("XdgShellHandler::maximize_request");

        let Some(window) = self.pinnacle.window_for_surface(surface.wl_surface()) else {
            return;
        };

        window.with_state_mut(|state| state.window_state.set_maximized(true));
        self.update_window_state_and_layout(&window);
    }

    fn unmaximize_request(&mut self, surface: ToplevelSurface) {
        let _span = tracy_client::span!("XdgShellHandler::unmaximize_request");

        let Some(window) = self.pinnacle.window_for_surface(surface.wl_surface()) else {
            return;
        };

        window.with_state_mut(|state| state.window_state.set_maximized(false));
        self.update_window_state_and_layout(&window);
    }

    fn minimize_request(&mut self, _surface: ToplevelSurface) {
        // TODO:
        // if let Some(window) = self.window_for_surface(surface.wl_surface()) {
        //     self.space.unmap_elem(&window);
        // }
    }

    // TODO: impl the rest of the fns in XdgShellHandler
}
delegate_xdg_shell!(State);

pub fn snapshot_pre_commit_hook(
    state: &mut State,
    _display_handle: &DisplayHandle,
    surface: &WlSurface,
) {
    let _span = tracy_client::span!("snapshot_pre_commit_hook");

    let Some(window) = state.pinnacle.window_for_surface(surface) else {
        return;
    };

    let got_unmapped = compositor::with_states(surface, |states| {
        let mut guard = states.cached_state.get::<SurfaceAttributes>();
        let buffer = &guard.pending().buffer;
        matches!(buffer, Some(BufferAssignment::Removed))
    });

    if got_unmapped {
        let Some(output) = window.output(&state.pinnacle) else {
            return;
        };
        let Some(loc) = state.pinnacle.space.element_location(&window) else {
            return;
        };

        let loc = loc - output.current_location();

        state.backend.with_renderer(|renderer| {
            window.capture_snapshot_and_store(
                renderer,
                loc,
                output.current_scale().fractional_scale().into(),
                1.0,
            );
        });
    } else {
        window.with_state_mut(|state| state.snapshot.take());
    }
}
