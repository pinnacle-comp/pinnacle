// SPDX-License-Identifier: GPL-3.0-or-later

mod xdg_shell;
mod xwayland;

use std::os::fd::OwnedFd;

use smithay::{
    backend::renderer::utils,
    delegate_compositor, delegate_data_device, delegate_fractional_scale, delegate_layer_shell,
    delegate_output, delegate_presentation, delegate_primary_selection, delegate_relative_pointer,
    delegate_seat, delegate_shm, delegate_viewporter,
    desktop::{self, find_popup_root_surface, layer_map_for_output, PopupKind, WindowSurfaceType},
    input::{pointer::CursorImageStatus, Seat, SeatHandler, SeatState},
    output::Output,
    reexports::{
        calloop::Interest,
        wayland_server::{
            protocol::{
                wl_buffer::WlBuffer, wl_data_source::WlDataSource, wl_output::WlOutput,
                wl_surface::WlSurface,
            },
            Client, Resource,
        },
    },
    wayland::{
        buffer::BufferHandler,
        compositor::{
            self, BufferAssignment, CompositorClientState, CompositorHandler, CompositorState,
            SurfaceAttributes,
        },
        dmabuf,
        fractional_scale::{self, FractionalScaleHandler},
        seat::WaylandFocus,
        selection::data_device::{
            set_data_device_focus, ClientDndGrabHandler, DataDeviceHandler, DataDeviceState,
            ServerDndGrabHandler,
        },
        selection::{
            primary_selection::{
                set_primary_focus, PrimarySelectionHandler, PrimarySelectionState,
            },
            SelectionHandler, SelectionSource, SelectionTarget,
        },
        shell::{
            wlr_layer::{self, Layer, LayerSurfaceData, WlrLayerShellHandler, WlrLayerShellState},
            xdg::{XdgPopupSurfaceData, XdgToplevelSurfaceData},
        },
        shm::{ShmHandler, ShmState},
    },
    xwayland::{X11Wm, XWaylandClientData},
};

use crate::{
    focus::FocusTarget,
    state::{CalloopData, ClientState, State, WithState},
    window::{window_state::LocationRequestState, WindowElement},
};

impl BufferHandler for State {
    fn buffer_destroyed(&mut self, _buffer: &WlBuffer) {}
}

impl CompositorHandler for State {
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
                            .blocker_cleared(&mut data.state, &data.display_handle);
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
        // tracing::debug!("commit on surface {surface:?}");
        X11Wm::commit_hook::<CalloopData>(surface);

        utils::on_commit_buffer_handler::<Self>(surface);
        self.backend.early_import(surface);

        let mut root = surface.clone();
        while let Some(parent) = compositor::get_parent(&root) {
            root = parent;
        }

        if !compositor::is_sync_subsurface(surface) {
            if let Some(win @ WindowElement::Wayland(window)) = &self.window_for_surface(&root) {
                // tracing::debug!("window commit thing {:?}", win.class());
                window.on_commit();
                win.with_state(|state| {
                    if let LocationRequestState::Acknowledged(new_pos) = state.loc_request_state {
                        tracing::debug!("Mapping Acknowledged window");
                        state.loc_request_state = LocationRequestState::Idle;
                        self.space.map_element(win.clone(), new_pos, false);
                    }
                });
            }
        };

        self.popup_manager.commit(surface);

        ensure_initial_configure(surface, self);

        crate::grab::resize_grab::handle_commit(self, surface);

        // `surface` is a root window
        let Some(output) = self
            .window_for_surface(surface)
            .and_then(|win| win.output(self))
            .or_else(|| {
                // `surface` is a descendant of a root window
                self.window_for_surface(&root)
                    .and_then(|win| win.output(self))
            })
            .or_else(|| {
                // `surface` is a popup
                self.popup_manager
                    .find_popup(surface)
                    .and_then(|popup| find_popup_root_surface(&popup).ok())
                    .and_then(|surf| self.window_for_surface(&surf))
                    .and_then(|win| win.output(self))
            })
            .or_else(|| {
                // `surface` is a layer surface
                self.space
                    .outputs()
                    .find(|op| {
                        let layer_map = layer_map_for_output(op);
                        layer_map
                            .layer_for_surface(surface, WindowSurfaceType::ALL)
                            .is_some()
                    })
                    .cloned()
            })
        // TODO: cursor surface and dnd icon
        else {
            return;
        };

        self.schedule_render(&output);
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
delegate_compositor!(State);

fn ensure_initial_configure(surface: &WlSurface, state: &mut State) {
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
        let PopupKind::Xdg(popup) = &popup else { return };
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
        return;
    }

    if let Some(output) = state.space.outputs().find(|op| {
        let map = layer_map_for_output(op);
        map.layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
            .is_some()
    }) {
        let initial_configure_sent = compositor::with_states(surface, |states| {
            states
                .data_map
                .get::<LayerSurfaceData>()
                .expect("no LayerSurfaceData")
                .lock()
                .expect("failed to lock data")
                .initial_configure_sent
        });

        let mut map = layer_map_for_output(output);

        map.arrange();

        if !initial_configure_sent {
            map.layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
                .expect("no layer for surface")
                .layer_surface()
                .send_configure();
        }
    }
}

impl ClientDndGrabHandler for State {
    fn started(
        &mut self,
        _source: Option<WlDataSource>,
        icon: Option<WlSurface>,
        _seat: Seat<Self>,
    ) {
        self.dnd_icon = icon;
    }

    fn dropped(&mut self, _seat: Seat<Self>) {
        self.dnd_icon = None;
    }
}

impl ServerDndGrabHandler for State {}

impl SelectionHandler for State {
    type SelectionUserData = ();

    fn new_selection(
        &mut self,
        ty: SelectionTarget,
        source: Option<SelectionSource>,
        _seat: Seat<Self>,
    ) {
        if let Some(xwm) = self.xwm.as_mut() {
            if let Err(err) = xwm.new_selection(ty, source.map(|source| source.mime_types())) {
                tracing::warn!(?err, ?ty, "Failed to set Xwayland selection");
            }
        }
    }

    fn send_selection(
        &mut self,
        ty: SelectionTarget,
        mime_type: String,
        fd: OwnedFd,
        _seat: Seat<Self>,
        _user_data: &(),
    ) {
        if let Some(xwm) = self.xwm.as_mut() {
            if let Err(err) = xwm.send_selection(ty, mime_type, fd, self.loop_handle.clone()) {
                tracing::warn!(?err, "Failed to send primary (X11 -> Wayland)");
            }
        }
    }
}

impl DataDeviceHandler for State {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}
delegate_data_device!(State);

impl PrimarySelectionHandler for State {
    fn primary_selection_state(&self) -> &PrimarySelectionState {
        &self.primary_selection_state
    }
}
delegate_primary_selection!(State);

impl SeatHandler for State {
    type KeyboardFocus = FocusTarget;
    type PointerFocus = FocusTarget;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn cursor_image(&mut self, _seat: &Seat<Self>, image: CursorImageStatus) {
        self.cursor_status = image;
    }

    fn focus_changed(&mut self, seat: &Seat<Self>, focused: Option<&Self::KeyboardFocus>) {
        if let Some(win) =
            focused.and_then(|focused| self.window_for_surface(&focused.wl_surface()?))
        {
            if let WindowElement::Wayland(win) = &win {
                win.set_activated(true);
            }
            self.focus_state.set_focus(win);
        }
        let focus_client = focused.and_then(|foc_target| {
            self.display_handle
                .get_client(foc_target.wl_surface()?.id())
                .ok()
        });
        set_data_device_focus(&self.display_handle, seat, focus_client.clone());
        set_primary_focus(&self.display_handle, seat, focus_client);
    }
}
delegate_seat!(State);

impl ShmHandler for State {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}
delegate_shm!(State);

delegate_output!(State);

delegate_viewporter!(State);

impl FractionalScaleHandler for State {
    fn new_fractional_scale(&mut self, surface: WlSurface) {
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
                desktop::utils::surface_primary_scanout_output(&surface, states)
                    .or_else(|| {
                        if root != surface {
                            compositor::with_states(&root, |states| {
                                desktop::utils::surface_primary_scanout_output(&root, states)
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

delegate_fractional_scale!(State);

delegate_relative_pointer!(State);

delegate_presentation!(State);

impl WlrLayerShellHandler for State {
    fn shell_state(&mut self) -> &mut WlrLayerShellState {
        &mut self.layer_shell_state
    }

    fn new_layer_surface(
        &mut self,
        surface: wlr_layer::LayerSurface,
        output: Option<WlOutput>,
        _layer: Layer,
        namespace: String,
    ) {
        tracing::debug!("New layer surface");
        let output = output
            .as_ref()
            .and_then(Output::from_resource)
            .or_else(|| self.space.outputs().next().cloned());

        let Some(output) = output else {
            tracing::error!("New layer surface, but there was no output to map it on");
            return;
        };

        let mut map = layer_map_for_output(&output);
        map.map_layer(&desktop::LayerSurface::new(surface, namespace))
            .expect("failed to map layer surface");
        drop(map); // wow i really love refcells haha

        self.loop_handle.insert_idle(move |data| {
            data.state.update_windows(&output);
        });
    }

    fn layer_destroyed(&mut self, surface: wlr_layer::LayerSurface) {
        let mut output: Option<Output> = None;
        if let Some((mut map, layer, op)) = self.space.outputs().find_map(|o| {
            let map = layer_map_for_output(o);
            let layer = map
                .layers()
                .find(|&layer| layer.layer_surface() == &surface)
                .cloned();
            layer.map(|layer| (map, layer, o))
        }) {
            map.unmap_layer(&layer);
            output = Some(op.clone());
        }

        if let Some(output) = output {
            self.loop_handle.insert_idle(move |data| {
                data.state.update_windows(&output);
            });
        }
    }
}
delegate_layer_shell!(State);
