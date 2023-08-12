use std::time::Duration;

use smithay::{
    delegate_xdg_shell,
    desktop::{
        find_popup_root_surface, layer_map_for_output, utils::surface_primary_scanout_output,
        PopupKeyboardGrab, PopupKind, PopupPointerGrab, PopupUngrabStrategy, Window,
        WindowSurfaceType,
    },
    input::{pointer::Focus, Seat},
    output::Output,
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel::{self, ResizeEdge},
        wayland_server::{
            protocol::{wl_output::WlOutput, wl_seat::WlSeat, wl_surface::WlSurface},
            Resource,
        },
    },
    utils::{Serial, SERIAL_COUNTER},
    wayland::{
        compositor::{self, CompositorHandler},
        shell::xdg::{
            Configure, PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler,
            XdgShellState,
        },
    },
};

use crate::{
    backend::Backend,
    focus::FocusTarget,
    state::{State, WithState},
    window::{
        window_state::{LocationRequestState, StatusName},
        WindowElement, BLOCKER_COUNTER,
    },
};

impl<B: Backend> XdgShellHandler for State<B> {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = WindowElement::Wayland(Window::new(surface.clone()));

        window.set_status(StatusName::Tiled);

        window.with_state(|state| {
            state.tags = match (
                &self.focus_state.focused_output,
                self.space.outputs().next(),
            ) {
                (Some(output), _) | (None, Some(output)) => output.with_state(|state| {
                    let output_tags = state.focused_tags().cloned().collect::<Vec<_>>();
                    if !output_tags.is_empty() {
                        output_tags
                    } else if let Some(first_tag) = state.tags.first() {
                        vec![first_tag.clone()]
                    } else {
                        vec![]
                    }
                }),
                (None, None) => vec![],
            };

            tracing::debug!("new window, tags are {:?}", state.tags);
        });

        let windows_on_output = self
            .windows
            .iter()
            .filter(|win| {
                win.with_state(|state| {
                    self.focus_state
                        .focused_output
                        .as_ref()
                        .expect("no focused output")
                        .with_state(|op_state| {
                            op_state
                                .tags
                                .iter()
                                .any(|tag| state.tags.iter().any(|tg| tg == tag))
                        })
                })
            })
            .cloned()
            .collect::<Vec<_>>();

        // note to self: don't reorder this
        // TODO: fix it so that reordering this doesn't break stuff
        self.windows.push(window.clone());
        // self.space.map_element(window.clone(), (0, 0), true);
        if let Some(focused_output) = self.focus_state.focused_output.clone() {
            self.update_windows(&focused_output);
            BLOCKER_COUNTER.store(1, std::sync::atomic::Ordering::SeqCst);
            tracing::debug!(
                "blocker {}",
                BLOCKER_COUNTER.load(std::sync::atomic::Ordering::SeqCst)
            );
            for win in windows_on_output.iter() {
                if let Some(surf) = win.wl_surface() {
                    compositor::add_blocker(&surf, crate::window::WindowBlocker);
                }
            }
            let clone = window.clone();
            self.loop_handle.insert_idle(|data| {
                crate::state::schedule_on_commit(data, vec![clone], move |data| {
                    BLOCKER_COUNTER.store(0, std::sync::atomic::Ordering::SeqCst);
                    tracing::debug!(
                        "blocker {}",
                        BLOCKER_COUNTER.load(std::sync::atomic::Ordering::SeqCst)
                    );
                    for client in windows_on_output
                        .iter()
                        .filter_map(|win| win.wl_surface()?.client())
                    {
                        data.state
                            .client_compositor_state(&client)
                            .blocker_cleared(&mut data.state, &data.display.handle())
                    }
                })
            });
        }
        self.loop_handle.insert_idle(move |data| {
            data.state
                .seat
                .get_keyboard()
                .expect("Seat had no keyboard") // FIXME: actually handle error
                .set_focus(
                    &mut data.state,
                    Some(FocusTarget::Window(window)),
                    SERIAL_COUNTER.next_serial(),
                );
        });
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        tracing::debug!("toplevel destroyed");
        self.windows.retain(|window| {
            window
                .wl_surface()
                .is_some_and(|surf| &surf != surface.wl_surface())
        });
        if let Some(focused_output) = self.focus_state.focused_output.as_ref().cloned() {
            self.update_windows(&focused_output);
        }

        // let mut windows: Vec<Window> = self.space.elements().cloned().collect();
        // windows.retain(|window| window.toplevel() != &surface);
        // Layouts::master_stack(self, windows, crate::layout::Direction::Left);
        let focus = self.focus_state.current_focus().map(FocusTarget::Window);
        self.seat
            .get_keyboard()
            .expect("Seat had no keyboard")
            .set_focus(self, focus, SERIAL_COUNTER.next_serial());
    }

    fn new_popup(&mut self, surface: PopupSurface, _positioner: PositionerState) {
        if let Err(err) = self.popup_manager.track_popup(PopupKind::from(surface)) {
            tracing::warn!("failed to track popup: {}", err);
        }
    }

    fn move_request(&mut self, surface: ToplevelSurface, seat: WlSeat, serial: Serial) {
        tracing::debug!("move_request_client");
        const BUTTON_LEFT: u32 = 0x110; // We assume the left mouse button is used
        crate::grab::move_grab::move_request_client(
            self,
            surface.wl_surface(),
            &Seat::from_resource(&seat).expect("Couldn't get seat from WlSeat"),
            serial,
            BUTTON_LEFT,
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
            &Seat::from_resource(&seat).expect("Couldn't get seat from WlSeat"),
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
        surface.with_pending_state(|state| {
            state.geometry = positioner.get_geometry();
            state.positioner = positioner;
        });
        surface.send_repositioned(token);
    }

    fn grab(&mut self, surface: PopupSurface, seat: WlSeat, serial: Serial) {
        let seat: Seat<Self> = Seat::from_resource(&seat).expect("Couldn't get seat from WlSeat");
        let popup_kind = PopupKind::Xdg(surface);
        if let Some(root) = find_popup_root_surface(&popup_kind).ok().and_then(|root| {
            self.window_for_surface(&root)
                .map(FocusTarget::Window)
                .or_else(|| {
                    self.space.outputs().find_map(|op| {
                        layer_map_for_output(op)
                            .layer_for_surface(&root, WindowSurfaceType::TOPLEVEL)
                            .cloned()
                            .map(FocusTarget::LayerSurface)
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

    fn ack_configure(&mut self, surface: WlSurface, configure: Configure) {
        if let Some(window) = self.window_for_surface(&surface) {
            window.with_state(|state| {
                if let LocationRequestState::Requested(serial, new_loc) = state.loc_request_state {
                    match &configure {
                        Configure::Toplevel(configure) => {
                            if configure.serial >= serial {
                                // tracing::debug!("acked configure, new loc is {:?}", new_loc);
                                state.loc_request_state =
                                    LocationRequestState::Acknowledged(new_loc);
                                if let Some(focused_output) =
                                    self.focus_state.focused_output.clone()
                                {
                                    window.send_frame(
                                        &focused_output,
                                        self.clock.now(),
                                        Some(Duration::ZERO),
                                        surface_primary_scanout_output,
                                    );
                                }
                            }
                        }
                        Configure::Popup(_) => todo!(),
                    }
                }
            });
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
                    .and_then(|window| self.space.outputs_for_element(&window).get(0).cloned())
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

            window.with_state(|state| {
                let tags = state.tags.iter();
                for tag in tags {
                    // tag.set_fullscreen_window(window.clone());
                }
            });
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

        if let Some(window) = self.window_for_surface(surface.wl_surface()) {
            window.with_state(|state| {
                for tag in state.tags.iter() {
                    // tag.set_fullscreen_window(None);
                }
            });
        }

        surface.send_pending_configure();
    }

    // fn minimize_request(&mut self, surface: ToplevelSurface) {
    //     if let Some(window) = self.window_for_surface(surface.wl_surface()) {
    //         self.space.unmap_elem(&window);
    //     }
    // }

    // TODO: impl the rest of the fns in XdgShellHandler
}
delegate_xdg_shell!(@<B: Backend> State<B>);
