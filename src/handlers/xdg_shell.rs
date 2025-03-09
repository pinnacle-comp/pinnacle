use smithay::{
    delegate_xdg_shell,
    desktop::{
        find_popup_root_surface, layer_map_for_output, PopupKeyboardGrab, PopupKind,
        PopupPointerGrab, PopupUngrabStrategy, Window, WindowSurfaceType,
    },
    input::{pointer::Focus, Seat},
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge,
        wayland_server::{
            protocol::{wl_output::WlOutput, wl_seat::WlSeat, wl_surface::WlSurface},
            DisplayHandle,
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
    window::{window_state::LayoutMode, Unmapped, WindowElement},
};

impl XdgShellHandler for State {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.pinnacle.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let _span = tracy_client::span!("XdgShellHandler::new_toplevel");

        let window = WindowElement::new(Window::new_wayland_window(surface.clone()));

        self.pinnacle.unmapped_windows.push(Unmapped {
            window,
            activation_token_data: None,
            window_rules: Default::default(),
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

        if is_tiled {
            if let Some(output) = output.as_ref() {
                self.capture_snapshots_on_output(output, []);
            }
        }

        self.pinnacle.remove_window(&window, false);

        if let Some(output) = output {
            if is_tiled {
                self.pinnacle.begin_layout_transaction(&output);
                self.pinnacle.request_layout(&output);
            }

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

    fn fullscreen_request(&mut self, surface: ToplevelSurface, _wl_output: Option<WlOutput>) {
        let _span = tracy_client::span!("XdgShellHandler::fullscreen_request");

        // TODO: Respect client output preference

        if let Some(window) = self
            .pinnacle
            .window_for_surface(surface.wl_surface())
            .cloned()
        {
            window.with_state_mut(|state| state.layout_mode.set_fullscreen(true));
            self.update_window_state_and_layout(&window);
        } else if let Some(unmapped) = self
            .pinnacle
            .unmapped_window_for_surface_mut(surface.wl_surface())
        {
            if unmapped.window_rules.layout_mode.is_none() {
                unmapped.window_rules.layout_mode = Some(LayoutMode::fullscreen());
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
            window.with_state_mut(|state| state.layout_mode.set_fullscreen(false));
            self.update_window_state_and_layout(&window);
        }
    }

    fn maximize_request(&mut self, surface: ToplevelSurface) {
        let _span = tracy_client::span!("XdgShellHandler::maximize_request");

        if let Some(window) = self
            .pinnacle
            .window_for_surface(surface.wl_surface())
            .cloned()
        {
            window.with_state_mut(|state| state.layout_mode.set_maximized(true));
            self.update_window_state_and_layout(&window);
        } else if let Some(unmapped) = self
            .pinnacle
            .unmapped_window_for_surface_mut(surface.wl_surface())
        {
            if unmapped.window_rules.layout_mode.is_none() {
                unmapped.window_rules.layout_mode = Some(LayoutMode::maximized());
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
            window.with_state_mut(|state| state.layout_mode.set_maximized(false));
            self.update_window_state_and_layout(&window);
        }
    }

    fn minimize_request(&mut self, _surface: ToplevelSurface) {
        // TODO:
        // if let Some(window) = self.window_for_surface(surface.wl_surface()) {
        //     self.space.unmap_elem(&window);
        // }
    }
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
        let Some(loc) = state.pinnacle.space.element_location(window) else {
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
