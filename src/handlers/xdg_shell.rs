use smithay::{
    delegate_xdg_shell,
    desktop::{
        find_popup_root_surface, layer_map_for_output, PopupKeyboardGrab, PopupKind,
        PopupPointerGrab, PopupUngrabStrategy, Window, WindowSurfaceType,
    },
    input::{pointer::Focus, Seat},
    output::Output,
    reexports::{
        wayland_protocols::xdg::shell::server::{
            xdg_positioner::{Anchor, ConstraintAdjustment, Gravity},
            xdg_toplevel::{self, ResizeEdge},
        },
        wayland_server::{
            protocol::{wl_output::WlOutput, wl_seat::WlSeat, wl_surface::WlSurface},
            Resource,
        },
    },
    utils::{Logical, Point, Rectangle, Serial, SERIAL_COUNTER},
    wayland::{
        seat::WaylandFocus,
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
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::TiledTop);
            state.states.set(xdg_toplevel::State::TiledBottom);
            state.states.set(xdg_toplevel::State::TiledLeft);
            state.states.set(xdg_toplevel::State::TiledRight);
        });

        let window = WindowElement::new(Window::new_wayland_window(surface.clone()));
        self.new_windows.push(window);
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        tracing::debug!("toplevel destroyed");
        self.windows.retain(|window| {
            window
                .wl_surface()
                .is_some_and(|surf| &surf != surface.wl_surface())
        });

        self.z_index_stack.retain(|window| {
            window
                .wl_surface()
                .is_some_and(|surf| &surf != surface.wl_surface())
        });

        for output in self.space.outputs() {
            output.with_state_mut(|state| {
                state.focus_stack.stack.retain(|window| {
                    window
                        .wl_surface()
                        .is_some_and(|surf| &surf != surface.wl_surface())
                })
            });
        }

        let Some(window) = self.window_for_surface(surface.wl_surface()) else {
            return;
        };

        if let Some(output) = window.output(self) {
            self.request_layout(&output);
            let focus = self
                .focused_window(&output)
                .map(KeyboardFocusTarget::Window);
            if let Some(KeyboardFocusTarget::Window(window)) = &focus {
                tracing::debug!("Focusing on prev win");
                // TODO:
                self.raise_window(window.clone(), true);
                if let Some(toplevel) = window.toplevel() {
                    toplevel.send_configure();
                }
            }
            self.seat
                .get_keyboard()
                .expect("Seat had no keyboard")
                .set_focus(self, focus, SERIAL_COUNTER.next_serial());

            self.schedule_render(&output);
        }
    }

    // this is 500 lines there has to be a shorter way to do this
    fn new_popup(&mut self, surface: PopupSurface, positioner: PositionerState) {
        tracing::info!("XdgShellHandler::new_popup");

        let popup_geo = (|| -> Option<Rectangle<i32, Logical>> {
            let root = find_popup_root_surface(&PopupKind::Xdg(surface.clone())).ok()?;
            let parent = surface.get_parent_surface()?;

            let win = self.window_for_surface(&root)?;
            let win_loc = self.space.element_geometry(&win)?.loc;
            let parent_loc = if root == parent {
                win_loc
            } else {
                match self.popup_manager.find_popup(&parent)? {
                    PopupKind::Xdg(surf) => {
                        surf.with_pending_state(|state| state.geometry.loc) + win_loc
                    }
                    PopupKind::InputMethod(_) => return None,
                }
            };

            let mut output_geo = win
                .output(self)
                .and_then(|op| self.space.output_geometry(&op))?;

            // Make local to parent
            output_geo.loc -= dbg!(parent_loc);
            Some(positioner.get_unconstrained_geometry(output_geo))
        })()
        .unwrap_or_else(|| positioner.get_geometry());

        dbg!(popup_geo);

        surface.with_pending_state(|state| state.geometry = popup_geo);

        if let Err(err) = self.popup_manager.track_popup(PopupKind::from(surface)) {
            tracing::warn!("failed to track popup: {}", err);
        }
    }

    fn popup_destroyed(&mut self, _surface: PopupSurface) {
        // TODO: only schedule on the outputs the popup is on
        for output in self.space.outputs().cloned().collect::<Vec<_>>() {
            self.schedule_render(&output);
        }
    }

    fn move_request(&mut self, surface: ToplevelSurface, seat: WlSeat, serial: Serial) {
        tracing::debug!("move_request_client");
        crate::grab::move_grab::move_request_client(
            self,
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
        const BUTTON_LEFT: u32 = 0x110;
        crate::grab::resize_grab::resize_request_client(
            self,
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
        // TODO: reposition logic

        surface.with_pending_state(|state| {
            state.geometry = positioner.get_geometry();
            state.positioner = positioner;
        });
        surface.send_repositioned(token);
    }

    fn grab(&mut self, surface: PopupSurface, seat: WlSeat, serial: Serial) {
        let seat: Seat<Self> = Seat::from_resource(&seat).expect("couldn't get seat from WlSeat");
        let popup_kind = PopupKind::Xdg(surface);
        if let Some(root) = find_popup_root_surface(&popup_kind).ok().and_then(|root| {
            self.window_for_surface(&root)
                .map(KeyboardFocusTarget::Window)
                .or_else(|| {
                    self.space.outputs().find_map(|op| {
                        layer_map_for_output(op)
                            .layer_for_surface(&root, WindowSurfaceType::TOPLEVEL)
                            .cloned()
                            .map(KeyboardFocusTarget::LayerSurface)
                    })
                })
        }) {
            if let Ok(mut grab) = self
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
                    keyboard.set_grab(PopupKeyboardGrab::new(&grab), serial);
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
                self.window_for_surface(wl_surface)
                    .and_then(|window| self.space.outputs_for_element(&window).first().cloned())
            });

        if let Some(output) = output {
            let Some(geometry) = self.space.output_geometry(&output) else {
                surface.send_configure();
                return;
            };

            let client = self
                .display_handle
                .get_client(wl_surface.id())
                .expect("wl_surface had no client");
            for output in output.client_outputs(&client) {
                wl_output = Some(output);
            }

            surface.with_pending_state(|state| {
                state.states.set(xdg_toplevel::State::Fullscreen);
                state.size = Some(geometry.size);
                state.fullscreen_output = wl_output;
            });

            let Some(window) = self.window_for_surface(wl_surface) else {
                tracing::error!("wl_surface had no window");
                return;
            };

            if !window.with_state(|state| state.fullscreen_or_maximized.is_fullscreen()) {
                window.toggle_fullscreen();
                self.request_layout(&output);
            }
        }

        surface.send_configure();
    }

    fn unfullscreen_request(&mut self, surface: ToplevelSurface) {
        if !surface
            .current_state()
            .states
            .contains(xdg_toplevel::State::Fullscreen)
        {
            return;
        }

        surface.with_pending_state(|state| {
            state.states.unset(xdg_toplevel::State::Fullscreen);
            state.size = None;
            state.fullscreen_output.take();
        });

        surface.send_pending_configure();

        let Some(window) = self.window_for_surface(surface.wl_surface()) else {
            tracing::error!("wl_surface had no window");
            return;
        };

        if window.with_state(|state| state.fullscreen_or_maximized.is_fullscreen()) {
            window.toggle_fullscreen();
            if let Some(output) = window.output(self) {
                self.request_layout(&output);
            }
        }
    }

    fn maximize_request(&mut self, surface: ToplevelSurface) {
        let Some(window) = self.window_for_surface(surface.wl_surface()) else {
            return;
        };

        if !window.with_state(|state| state.fullscreen_or_maximized.is_maximized()) {
            window.toggle_maximized();
        }

        let Some(output) = window.output(self) else { return };
        self.request_layout(&output);
    }

    fn unmaximize_request(&mut self, surface: ToplevelSurface) {
        let Some(window) = self.window_for_surface(surface.wl_surface()) else {
            return;
        };

        if window.with_state(|state| state.fullscreen_or_maximized.is_maximized()) {
            window.toggle_maximized();
        }

        let Some(output) = window.output(self) else { return };
        self.request_layout(&output);
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
