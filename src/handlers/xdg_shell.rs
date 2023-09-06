use smithay::{
    delegate_xdg_shell,
    desktop::{
        find_popup_root_surface, layer_map_for_output, space::SpaceElement, PopupKeyboardGrab,
        PopupKind, PopupPointerGrab, PopupUngrabStrategy, Window, WindowSurfaceType,
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
    utils::{Point, Rectangle, Serial, SERIAL_COUNTER},
    wayland::{
        compositor::{self, CompositorHandler},
        shell::xdg::{
            Configure, PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler,
            XdgShellState,
        },
    },
};

use crate::{
    api::msg::window_rules::{self, WindowRule},
    focus::FocusTarget,
    state::{State, WithState},
    window::{
        window_state::{FloatingOrTiled, LocationRequestState},
        WindowElement, BLOCKER_COUNTER,
    },
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

        let window = WindowElement::Wayland(Window::new(surface.clone()));

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

        self.space.map_element(window.clone(), (0, 0), true);

        let win_clone = window.clone();
        self.schedule(
            move |_data| {
                if let WindowElement::Wayland(window) = &win_clone {
                    let initial_configure_sent =
                        compositor::with_states(window.toplevel().wl_surface(), |states| {
                            states
                                .data_map
                                .get::<smithay::wayland::shell::xdg::XdgToplevelSurfaceData>()
                                .expect("XdgToplevelSurfaceData wasn't in surface's data map")
                                .lock()
                                .expect("Failed to lock Mutex<XdgToplevelSurfaceData>")
                                .initial_configure_sent
                        });

                    initial_configure_sent
                } else {
                    true
                }
            },
            |data| {
                for (cond, rule) in data.state.window_rules.iter() {
                    if cond.is_met(&data.state, &window) {
                        let WindowRule {
                            output,
                            tags,
                            floating_or_tiled,
                            fullscreen_or_maximized,
                            size,
                            location,
                        } = rule;

                        if let Some(_output_name) = output {
                            // TODO:
                        }

                        if let Some(tag_ids) = tags {
                            let tags = tag_ids
                                .iter()
                                .filter_map(|tag_id| tag_id.tag(&data.state))
                                .collect::<Vec<_>>();

                            window.with_state(|state| state.tags = tags.clone());
                        }

                        if let Some(floating_or_tiled) = floating_or_tiled {
                            match floating_or_tiled {
                                window_rules::FloatingOrTiled::Floating => {
                                    if window.with_state(|state| state.floating_or_tiled.is_tiled())
                                    {
                                        window.toggle_floating();
                                    }
                                }
                                window_rules::FloatingOrTiled::Tiled => {
                                    if window
                                        .with_state(|state| state.floating_or_tiled.is_floating())
                                    {
                                        window.toggle_floating();
                                    }
                                }
                            }
                        }

                        if let Some(fs_or_max) = fullscreen_or_maximized {
                            window.with_state(|state| state.fullscreen_or_maximized = *fs_or_max);
                        }

                        if let Some((w, h)) = size {
                            let mut window_size = window.geometry().size;
                            window_size.w = u32::from(*w) as i32;
                            window_size.h = u32::from(*h) as i32;

                            match window.with_state(|state| state.floating_or_tiled) {
                                FloatingOrTiled::Floating(mut rect) => {
                                    rect.size = (u32::from(*w) as i32, u32::from(*h) as i32).into();
                                    window.with_state(|state| {
                                        state.floating_or_tiled = FloatingOrTiled::Floating(rect)
                                    });
                                }
                                FloatingOrTiled::Tiled(mut rect) => {
                                    if let Some(rect) = rect.as_mut() {
                                        rect.size =
                                            (u32::from(*w) as i32, u32::from(*h) as i32).into();
                                    }
                                    window.with_state(|state| {
                                        state.floating_or_tiled = FloatingOrTiled::Tiled(rect)
                                    });
                                }
                            }
                        }

                        if let Some(loc) = location {
                            match window.with_state(|state| state.floating_or_tiled) {
                                FloatingOrTiled::Floating(mut rect) => {
                                    rect.loc = (*loc).into();
                                    window.with_state(|state| {
                                        state.floating_or_tiled = FloatingOrTiled::Floating(rect)
                                    });
                                    data.state.space.map_element(window.clone(), *loc, false);
                                }
                                FloatingOrTiled::Tiled(rect) => {
                                    // If the window is tiled, don't set the size. Instead, set
                                    // what the size will be when it gets set to floating.
                                    let rect = rect.unwrap_or_else(|| {
                                        let size = window.geometry().size;
                                        Rectangle::from_loc_and_size(Point::from(*loc), size)
                                    });

                                    window.with_state(|state| {
                                        state.floating_or_tiled = FloatingOrTiled::Tiled(Some(rect))
                                    });
                                }
                            }
                        }
                    }
                }

                if let Some(focused_output) = data.state.focus_state.focused_output.clone() {
                    data.state.update_windows(&focused_output);
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
                    data.state.loop_handle.insert_idle(|data| {
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
                data.state.loop_handle.insert_idle(move |data| {
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
            },
        );
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

            if !window.with_state(|state| state.fullscreen_or_maximized.is_fullscreen()) {
                window.toggle_fullscreen();
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
        }
    }

    fn maximize_request(&mut self, surface: ToplevelSurface) {
        let Some(window) = self.window_for_surface(surface.wl_surface()) else {
            return;
        };

        if !window.with_state(|state| state.fullscreen_or_maximized.is_maximized()) {
            window.toggle_maximized();
        }
        // TODO: might need to update_windows here
    }

    fn unmaximize_request(&mut self, surface: ToplevelSurface) {
        let Some(window) = self.window_for_surface(surface.wl_surface()) else {
            return;
        };

        if window.with_state(|state| state.fullscreen_or_maximized.is_maximized()) {
            window.toggle_maximized();
        }
    }

    // fn minimize_request(&mut self, surface: ToplevelSurface) {
    //     if let Some(window) = self.window_for_surface(surface.wl_surface()) {
    //         self.space.unmap_elem(&window);
    //     }
    // }

    // TODO: impl the rest of the fns in XdgShellHandler
}
delegate_xdg_shell!(State);
