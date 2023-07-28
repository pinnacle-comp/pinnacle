// SPDX-License-Identifier: GPL-3.0-or-later

pub mod xwayland;

use std::time::Duration;

use smithay::{
    backend::renderer::utils,
    delegate_compositor, delegate_data_device, delegate_fractional_scale, delegate_output,
    delegate_presentation, delegate_relative_pointer, delegate_seat, delegate_shm,
    delegate_viewporter, delegate_xdg_shell,
    desktop::{
        find_popup_root_surface, utils::surface_primary_scanout_output, PopupKeyboardGrab,
        PopupKind, PopupPointerGrab, PopupUngrabStrategy, Window,
    },
    input::{
        pointer::{CursorImageStatus, Focus},
        Seat, SeatHandler, SeatState,
    },
    reexports::{
        calloop::Interest,
        wayland_protocols::xdg::shell::server::xdg_toplevel::{self, ResizeEdge},
        wayland_server::{
            protocol::{wl_buffer::WlBuffer, wl_seat::WlSeat, wl_surface::WlSurface},
            Client, Resource,
        },
    },
    utils::{Serial, SERIAL_COUNTER},
    wayland::{
        buffer::BufferHandler,
        compositor::{
            self, BufferAssignment, CompositorClientState, CompositorHandler, CompositorState,
            SurfaceAttributes,
        },
        data_device::{
            set_data_device_focus, ClientDndGrabHandler, DataDeviceHandler, DataDeviceState,
            ServerDndGrabHandler,
        },
        dmabuf,
        fractional_scale::{self, FractionalScaleHandler},
        seat::WaylandFocus,
        shell::xdg::{
            Configure, PopupSurface, PositionerState, ToplevelSurface, XdgPopupSurfaceData,
            XdgShellHandler, XdgShellState, XdgToplevelSurfaceData,
        },
        shm::{ShmHandler, ShmState},
    },
    xwayland::{X11Wm, XWaylandClientData},
};

use crate::{
    backend::Backend,
    focus::FocusTarget,
    state::{CalloopData, ClientState, State, WithState},
    window::{window_state::WindowResizeState, WindowBlocker, WindowElement, BLOCKER_COUNTER},
};

impl<B: Backend> BufferHandler for State<B> {
    fn buffer_destroyed(&mut self, _buffer: &WlBuffer) {}
}

impl<B: Backend> CompositorHandler for State<B> {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn new_surface(&mut self, surface: &WlSurface) {
        compositor::add_pre_commit_hook::<Self, _>(surface, |state, _display_handle, surface| {
            let maybe_dmabuf = compositor::with_states(surface, |surface_data| {
                surface_data
                    .cached_state
                    .pending::<SurfaceAttributes>()
                    .buffer
                    .as_ref()
                    .and_then(|assignment| match assignment {
                        BufferAssignment::NewBuffer(buffer) => dmabuf::get_dmabuf(buffer).ok(),
                        _ => None,
                    })
            });
            if let Some(dmabuf) = maybe_dmabuf {
                if let Ok((blocker, source)) = dmabuf.generate_blocker(Interest::READ) {
                    let client = surface
                        .client()
                        .expect("Surface has no client/is no longer alive");
                    let res = state.loop_handle.insert_source(source, move |_, _, data| {
                        data.state
                            .client_compositor_state(&client)
                            .blocker_cleared(&mut data.state, &data.display.handle());
                        Ok(())
                    });
                    if res.is_ok() {
                        compositor::add_blocker(surface, blocker);
                    }
                }
            }
        });
    }

    fn commit(&mut self, surface: &WlSurface) {
        // tracing::debug!("commit");

        X11Wm::commit_hook::<CalloopData<B>>(surface);

        utils::on_commit_buffer_handler::<Self>(surface);
        self.backend_data.early_import(surface);

        if !compositor::is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = compositor::get_parent(&root) {
                root = parent;
            }
            if let Some(WindowElement::Wayland(window)) = self.window_for_surface(surface) {
                window.on_commit();
            }
        };

        self.popup_manager.commit(surface);

        ensure_initial_configure(surface, self);

        crate::grab::resize_grab::handle_commit(self, surface);

        if let Some(window) = self.window_for_surface(surface) {
            window.with_state(|state| {
                if let WindowResizeState::Acknowledged(new_pos) = state.resize_state {
                    state.resize_state = WindowResizeState::Idle;
                    if window.is_x11() {
                        tracing::error!("DID SOMETHING WITH x11 WINDOW HERE");
                        // if !surface.is_override_redirect() {
                        //     surface.set_mapped(true).expect("failed to map x11 win");
                        // }
                    }
                    self.space.map_element(window.clone(), new_pos, false);
                }
            });
        }
        // let states = self
        //     .windows
        //     .iter()
        //     .map(|win| win.with_state(|state| state.resize_state.clone()))
        //     .collect::<Vec<_>>();
        // tracing::debug!("states: {states:?}");
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        if let Some(state) = client.get_data::<XWaylandClientData>() {
            return &state.compositor_state;
        }
        if let Some(state) = client.get_data::<ClientState>() {
            return &state.compositor_state;
        }
        panic!("Unknown client data type");
    }
}
delegate_compositor!(@<B: Backend> State<B>);

fn ensure_initial_configure<B: Backend>(surface: &WlSurface, state: &mut State<B>) {
    if let Some(window) = state.window_for_surface(surface) {
        if let WindowElement::Wayland(window) = &window {
            let initial_configure_sent = compositor::with_states(surface, |states| {
                states
                    .data_map
                    .get::<XdgToplevelSurfaceData>()
                    .expect("XdgToplevelSurfaceData wasn't in surface's data map")
                    .lock()
                    .expect("Failed to lock Mutex<XdgToplevelSurfaceData>")
                    .initial_configure_sent
            });

            if !initial_configure_sent {
                tracing::debug!("Initial configure");
                window.toplevel().send_configure();
            }
        }
        return;
    }

    if let Some(popup) = state.popup_manager.find_popup(surface) {
        let PopupKind::Xdg(popup) = &popup;
        let initial_configure_sent = compositor::with_states(surface, |states| {
            states
                .data_map
                .get::<XdgPopupSurfaceData>()
                .expect("XdgPopupSurfaceData wasn't in popup's data map")
                .lock()
                .expect("Failed to lock Mutex<XdgPopupSurfaceData>")
                .initial_configure_sent
        });
        if !initial_configure_sent {
            popup
                .send_configure()
                .expect("popup initial configure failed");
        }
    }
    // TODO: layer map thingys
}

impl<B: Backend> ClientDndGrabHandler for State<B> {}
impl<B: Backend> ServerDndGrabHandler for State<B> {}

impl<B: Backend> DataDeviceHandler for State<B> {
    type SelectionUserData = ();

    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}
delegate_data_device!(@<B: Backend> State<B>);

impl<B: Backend> SeatHandler for State<B> {
    type KeyboardFocus = FocusTarget;
    type PointerFocus = FocusTarget;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn cursor_image(&mut self, _seat: &Seat<Self>, image: CursorImageStatus) {
        // tracing::info!("new cursor image: {:?}", image);
        self.cursor_status = image;
    }

    fn focus_changed(&mut self, seat: &Seat<Self>, focused: Option<&Self::KeyboardFocus>) {
        if let Some(focus) = focused.and_then(|focus| focus.wl_surface()) {
            if let Some(window) = self.window_for_surface(&focus) {
                self.focus_state.set_focus(window);
                // let focus = focused.and_then(|surf| self.display_handle.get_client(surf.id()).ok());
                // set_data_device_focus(&self.display_handle, seat, focus);
            }
        }
    }
}
delegate_seat!(@<B: Backend> State<B>);

impl<B: Backend> ShmHandler for State<B> {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}
delegate_shm!(@<B: Backend> State<B>);

impl<B: Backend> XdgShellHandler for State<B> {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = WindowElement::Wayland(Window::new(surface));

        {
            let WindowElement::Wayland(window) = &window else { unreachable!() };
            window.toplevel().with_pending_state(|tl_state| {
                tl_state.states.set(xdg_toplevel::State::TiledTop);
                tl_state.states.set(xdg_toplevel::State::TiledBottom);
                tl_state.states.set(xdg_toplevel::State::TiledLeft);
                tl_state.states.set(xdg_toplevel::State::TiledRight);
            });
        }

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
                        .unwrap()
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

        self.windows.push(window.clone());
        // self.space.map_element(window.clone(), (0, 0), true);
        if let Some(focused_output) = self.focus_state.focused_output.clone() {
            focused_output.with_state(|state| {
                let first_tag = state.focused_tags().next();
                if let Some(first_tag) = first_tag {
                    first_tag.layout().layout(
                        self.windows.clone(),
                        state.focused_tags().cloned().collect(),
                        self,
                        &focused_output,
                    );
                }
            });
            BLOCKER_COUNTER.store(1, std::sync::atomic::Ordering::SeqCst);
            tracing::debug!(
                "blocker {}",
                BLOCKER_COUNTER.load(std::sync::atomic::Ordering::SeqCst)
            );
            for win in windows_on_output.iter() {
                if let Some(surf) = win.wl_surface() {
                    compositor::add_blocker(&surf, WindowBlocker);
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
            focused_output.with_state(|state| {
                let first_tag = state.focused_tags().next();
                if let Some(first_tag) = first_tag {
                    first_tag.layout().layout(
                        self.windows.clone(),
                        state.focused_tags().cloned().collect(),
                        self,
                        &focused_output,
                    );
                }
            });
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
        crate::xdg::request::move_request(
            self,
            &surface,
            &Seat::from_resource(&seat).expect("Couldn't get seat from WlSeat"),
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
        crate::xdg::request::resize_request(
            self,
            surface.wl_surface(),
            &Seat::from_resource(&seat).expect("Couldn't get seat from WlSeat"),
            serial,
            edges,
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
        if let Some(root) = find_popup_root_surface(&popup_kind)
            .ok()
            .and_then(|root| self.window_for_surface(&root))
        {
            if let Ok(mut grab) =
                self.popup_manager
                    .grab_popup(FocusTarget::Window(root), popup_kind, &seat, serial)
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
                if let WindowResizeState::Requested(serial, new_loc) = state.resize_state {
                    match &configure {
                        Configure::Toplevel(configure) => {
                            if configure.serial >= serial {
                                // tracing::debug!("acked configure, new loc is {:?}", new_loc);
                                state.resize_state = WindowResizeState::Acknowledged(new_loc);
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

    // fn minimize_request(&mut self, surface: ToplevelSurface) {
    //     if let Some(window) = self.window_for_surface(surface.wl_surface()) {
    //         self.space.unmap_elem(&window);
    //     }
    // }

    // TODO: impl the rest of the fns in XdgShellHandler
}
delegate_xdg_shell!(@<B: Backend> State<B>);

delegate_output!(@<B: Backend> State<B>);

delegate_viewporter!(@<B: Backend> State<B>);

impl<B: Backend> FractionalScaleHandler for State<B> {
    fn new_fractional_scale(&mut self, surface: WlSurface) {
        // ripped straight from anvil

        // Here we can set the initial fractional scale
        //
        // First we look if the surface already has a primary scan-out output, if not
        // we test if the surface is a subsurface and try to use the primary scan-out output
        // of the root surface. If the root also has no primary scan-out output we just try
        // to use the first output of the toplevel.
        // If the surface is the root we also try to use the first output of the toplevel.
        //
        // If all the above tests do not lead to a output we just use the first output
        // of the space (which in case of anvil will also be the output a toplevel will
        // initially be placed on)
        let mut root = surface.clone();
        while let Some(parent) = compositor::get_parent(&root) {
            root = parent;
        }

        compositor::with_states(&surface, |states| {
            let primary_scanout_output =
                smithay::desktop::utils::surface_primary_scanout_output(&surface, states)
                    .or_else(|| {
                        if root != surface {
                            compositor::with_states(&root, |states| {
                                smithay::desktop::utils::surface_primary_scanout_output(
                                    &root, states,
                                )
                                .or_else(|| {
                                    self.window_for_surface(&root).and_then(|window| {
                                        self.space.outputs_for_element(&window).first().cloned()
                                    })
                                })
                            })
                        } else {
                            self.window_for_surface(&root).and_then(|window| {
                                self.space.outputs_for_element(&window).first().cloned()
                            })
                        }
                    })
                    .or_else(|| self.space.outputs().next().cloned());
            if let Some(output) = primary_scanout_output {
                fractional_scale::with_fractional_scale(states, |fractional_scale| {
                    fractional_scale.set_preferred_scale(output.current_scale().fractional_scale());
                });
            }
        });
    }
}

delegate_fractional_scale!(@<B: Backend> State<B>);

delegate_relative_pointer!(@<B: Backend> State<B>);

delegate_presentation!(@<B: Backend> State<B>);
