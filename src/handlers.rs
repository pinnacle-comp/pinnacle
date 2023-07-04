// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use smithay::{
    backend::renderer::utils,
    delegate_compositor, delegate_data_device, delegate_fractional_scale, delegate_output,
    delegate_presentation, delegate_relative_pointer, delegate_seat, delegate_shm,
    delegate_viewporter, delegate_xdg_shell,
    desktop::{
        find_popup_root_surface, PopupKeyboardGrab, PopupKind, PopupPointerGrab,
        PopupUngrabStrategy, Window,
    },
    input::{
        pointer::{CursorImageStatus, Focus},
        Seat, SeatHandler, SeatState,
    },
    reexports::{
        calloop::Interest,
        wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge,
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
            ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler,
        },
        dmabuf,
        fractional_scale::{self, FractionalScaleHandler},
        shell::xdg::{
            Configure, PopupSurface, PositionerState, ToplevelSurface, XdgPopupSurfaceData,
            XdgShellHandler, XdgShellState, XdgToplevelSurfaceData,
        },
        shm::{ShmHandler, ShmState},
    },
};

use crate::{
    backend::Backend,
    layout::Layout,
    output::OutputState,
    state::{ClientState, State},
    window::window_state::{WindowResizeState, WindowState},
};

impl<B: Backend> BufferHandler for State<B> {
    fn buffer_destroyed(&mut self, _buffer: &WlBuffer) {}
}

impl<B: Backend> CompositorHandler for State<B> {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn new_surface(&mut self, surface: &WlSurface) {
        // yanked straight from anvil
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
        tracing::debug!("commit");

        utils::on_commit_buffer_handler::<Self>(surface);

        if !compositor::is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = compositor::get_parent(&root) {
                root = parent;
            }
            if let Some(window) = self.window_for_surface(surface) {
                window.on_commit();
            }
        };

        self.popup_manager.commit(surface);

        ensure_initial_configure(surface, self);

        crate::grab::resize_grab::handle_commit(self, surface);

        if let Some(window) = self.window_for_surface(surface) {
            WindowState::with_state(&window, |state| {
                if let WindowResizeState::WaitingForCommit(new_pos) = state.resize_state {
                    state.resize_state = WindowResizeState::Idle;
                    self.space.map_element(window.clone(), new_pos, false);
                }
            });
        }
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client
            .get_data::<ClientState>()
            .expect("ClientState wasn't in client's data map")
            .compositor_state
    }
}
delegate_compositor!(@<B: Backend> State<B>);

fn ensure_initial_configure<B: Backend>(surface: &WlSurface, state: &mut State<B>) {
    if let Some(window) = state.window_for_surface(surface) {
        let initial_configure_sent = compositor::with_states(surface, |states| {
            states
                .data_map
                .get::<XdgToplevelSurfaceData>()
                .expect("XdgToplevelSurfaceData wasn't in surface's data map")
                .lock()
                .expect("Failed to lock Mutex<XdgToplevelSurfaceData>")
                .initial_configure_sent
        });
        // println!("initial_configure_sent is {}", initial_configure_sent);

        if !initial_configure_sent {
            tracing::debug!("Initial configure");
            window.toplevel().send_configure();
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
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn cursor_image(&mut self, _seat: &Seat<Self>, image: CursorImageStatus) {
        // tracing::info!("new cursor image: {:?}", image);
        self.cursor_status = image;
    }

    fn focus_changed(&mut self, _seat: &Seat<Self>, focused: Option<&Self::KeyboardFocus>) {
        if let Some(wl_surface) = focused {
            if let Some(window) = self.window_for_surface(wl_surface) {
                self.focus_state.set_focus(window);
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
        let window = Window::new(surface);

        WindowState::with_state(&window, |state| {
            state.tags = if let Some(focused_output) = &self.focus_state.focused_output {
                OutputState::with(focused_output, |state| {
                    state
                        .focused_tags
                        .iter()
                        .filter_map(|(id, active)| active.then_some(id.clone()))
                        .collect()
                })
            } else if let Some(first_tag) = self.tag_state.tags.first() {
                vec![first_tag.id.clone()]
            } else {
                vec![]
            };
            tracing::debug!("new window, tags are {:?}", state.tags);
        });

        self.windows.push(window.clone());
        self.space.map_element(window.clone(), (0, 0), true);
        self.loop_handle.insert_idle(move |data| {
            data.state
                .seat
                .get_keyboard()
                .expect("Seat had no keyboard") // FIXME: actually handle error
                .set_focus(
                    &mut data.state,
                    Some(window.toplevel().wl_surface().clone()),
                    SERIAL_COUNTER.next_serial(),
                );
        });

        let windows: Vec<Window> = self.space.elements().cloned().collect();

        self.loop_handle.insert_idle(|data| {
            tracing::debug!("Layout master_stack");
            Layout::master_stack(&mut data.state, windows, crate::layout::Direction::Left);
        });
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        tracing::debug!("toplevel destroyed");
        self.windows.retain(|window| window.toplevel() != &surface);
        let mut windows: Vec<Window> = self.space.elements().cloned().collect();
        windows.retain(|window| window.toplevel() != &surface);
        Layout::master_stack(self, windows, crate::layout::Direction::Left);
        let focus = self
            .focus_state
            .current_focus()
            .map(|win| win.toplevel().wl_surface().clone());
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
            &surface,
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
            if let Ok(mut grab) = self.popup_manager.grab_popup(
                root.toplevel().wl_surface().clone(),
                popup_kind,
                &seat,
                serial,
            ) {
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
        tracing::debug!("start of ack_configure");
        if let Some(window) = self.window_for_surface(&surface) {
            tracing::debug!("found window for surface");
            WindowState::with_state(&window, |state| {
                if let WindowResizeState::WaitingForAck(serial, new_loc) = state.resize_state {
                    match &configure {
                        Configure::Toplevel(configure) => {
                            if configure.serial >= serial {
                                tracing::debug!("acked configure, new loc is {:?}", new_loc);
                                state.resize_state = WindowResizeState::WaitingForCommit(new_loc);
                            }
                        }
                        Configure::Popup(_) => todo!(),
                    }
                }
            });

            // HACK: If a window is currently going through something that generates a bunch of
            // |     commits, like an animation, unmapping it while it's doing that has a chance
            // |     to cause any send_configures to not trigger a commit. I'm not sure if this is because of
            // |     the way I've implemented things or if it's something else. Because of me
            // |     mapping the element in commit, this means that the window won't reappear on a tag
            // |     change. The code below is a workaround until I can figure it out.
            if !self.space.elements().any(|win| win == &window) {
                WindowState::with_state(&window, |state| {
                    if let WindowResizeState::WaitingForCommit(new_loc) = state.resize_state {
                        tracing::debug!("remapping window");
                        let win = window.clone();
                        self.loop_handle.insert_idle(move |data| {
                            data.state.space.map_element(win, new_loc, false);
                        });
                    }
                });
            }
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
