// SPDX-License-Identifier: GPL-3.0-or-later

pub mod decoration;
mod foreign_toplevel;
pub mod idle;
pub mod session_lock;
pub mod window;
mod xdg_shell;
mod xwayland;

use std::{collections::HashMap, mem, os::fd::OwnedFd, sync::Arc};

use smithay::{
    backend::{
        input::TabletToolDescriptor,
        renderer::utils::{self, with_renderer_surface_state},
    },
    delegate_compositor, delegate_cursor_shape, delegate_data_control, delegate_data_device,
    delegate_fractional_scale, delegate_keyboard_shortcuts_inhibit, delegate_layer_shell,
    delegate_output, delegate_pointer_constraints, delegate_pointer_gestures,
    delegate_presentation, delegate_primary_selection, delegate_relative_pointer, delegate_seat,
    delegate_security_context, delegate_shm, delegate_single_pixel_buffer, delegate_tablet_manager,
    delegate_viewporter, delegate_xdg_activation, delegate_xwayland_keyboard_grab,
    delegate_xwayland_shell,
    desktop::{
        self, find_popup_root_surface, get_popup_toplevel_coords, layer_map_for_output,
        space::SpaceElement, PopupKind, PopupManager, WindowSurfaceType,
    },
    input::{
        keyboard::LedState,
        pointer::{CursorImageStatus, PointerHandle},
        Seat, SeatHandler, SeatState,
    },
    output::{Mode, Output, Scale},
    reexports::{
        calloop::Interest,
        wayland_protocols::xdg::shell::server::xdg_positioner::ConstraintAdjustment,
        wayland_server::{
            protocol::{
                wl_buffer::WlBuffer, wl_data_source::WlDataSource, wl_output::WlOutput,
                wl_surface::WlSurface,
            },
            Client, Resource,
        },
    },
    utils::{Logical, Point, Rectangle},
    wayland::{
        buffer::BufferHandler,
        compositor::{
            self, add_pre_commit_hook, BufferAssignment, CompositorClientState, CompositorHandler,
            CompositorState, SurfaceAttributes,
        },
        dmabuf,
        fractional_scale::{self, FractionalScaleHandler},
        keyboard_shortcuts_inhibit::{
            KeyboardShortcutsInhibitHandler, KeyboardShortcutsInhibitState,
            KeyboardShortcutsInhibitor,
        },
        output::OutputHandler,
        pointer_constraints::{with_pointer_constraint, PointerConstraintsHandler},
        seat::WaylandFocus,
        security_context::{
            SecurityContext, SecurityContextHandler, SecurityContextListenerSource,
        },
        selection::{
            data_device::{
                set_data_device_focus, ClientDndGrabHandler, DataDeviceHandler, DataDeviceState,
                ServerDndGrabHandler,
            },
            primary_selection::{
                set_primary_focus, PrimarySelectionHandler, PrimarySelectionState,
            },
            wlr_data_control::{DataControlHandler, DataControlState},
            SelectionHandler, SelectionSource, SelectionTarget,
        },
        shell::{
            wlr_layer::{self, Layer, LayerSurfaceData, WlrLayerShellHandler, WlrLayerShellState},
            xdg::{PopupSurface, SurfaceCachedState, XdgPopupSurfaceData, XdgToplevelSurfaceData},
        },
        shm::{ShmHandler, ShmState},
        tablet_manager::TabletSeatHandler,
        xdg_activation::{
            XdgActivationHandler, XdgActivationState, XdgActivationToken, XdgActivationTokenData,
        },
        xwayland_keyboard_grab::XWaylandKeyboardGrabHandler,
        xwayland_shell::{XWaylandShellHandler, XWaylandShellState},
    },
    xwayland::XWaylandClientData,
};
use tracing::{debug, error, trace, warn};

use crate::{
    backend::Backend,
    delegate_gamma_control, delegate_output_management, delegate_output_power_management,
    delegate_screencopy,
    focus::{keyboard::KeyboardFocusTarget, pointer::PointerFocusTarget},
    handlers::xdg_shell::snapshot_pre_commit_hook,
    output::OutputMode,
    protocol::{
        gamma_control::{GammaControlHandler, GammaControlManagerState},
        output_management::{
            OutputConfiguration, OutputManagementHandler, OutputManagementManagerState,
        },
        output_power_management::{OutputPowerManagementHandler, OutputPowerManagementState},
        screencopy::{Screencopy, ScreencopyHandler},
    },
    state::{ClientState, Pinnacle, State, WithState},
};

impl BufferHandler for State {
    fn buffer_destroyed(&mut self, _buffer: &WlBuffer) {}
}

impl CompositorHandler for State {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.pinnacle.compositor_state
    }

    fn new_surface(&mut self, surface: &WlSurface) {
        compositor::add_pre_commit_hook::<Self, _>(surface, |state, _display_handle, surface| {
            let maybe_dmabuf = compositor::with_states(surface, |surface_data| {
                surface_data
                    .cached_state
                    .get::<SurfaceAttributes>()
                    .pending()
                    .buffer
                    .as_ref()
                    .and_then(|assignment| match assignment {
                        BufferAssignment::NewBuffer(buffer) => {
                            dmabuf::get_dmabuf(buffer).cloned().ok()
                        }
                        _ => None,
                    })
            });
            if let Some(dmabuf) = maybe_dmabuf {
                if let Ok((blocker, source)) = dmabuf.generate_blocker(Interest::READ) {
                    if let Some(client) = surface.client() {
                        let res =
                            state
                                .pinnacle
                                .loop_handle
                                .insert_source(source, move |_, _, state| {
                                    state.client_compositor_state(&client).blocker_cleared(
                                        state,
                                        &state.pinnacle.display_handle.clone(),
                                    );
                                    Ok(())
                                });
                        if res.is_ok() {
                            compositor::add_blocker(surface, blocker);
                        }
                    }
                }
            }
        });
    }

    fn commit(&mut self, surface: &WlSurface) {
        // tracing::info!("commit on surface {surface:?}");

        utils::on_commit_buffer_handler::<State>(surface);

        self.backend.early_import(surface);

        if compositor::is_sync_subsurface(surface) {
            return;
        }

        let mut root = surface.clone();
        while let Some(parent) = compositor::get_parent(&root) {
            root = parent;
        }

        self.pinnacle
            .root_surface_cache
            .insert(surface.clone(), root.clone());

        if let Some(window) = self.pinnacle.window_for_surface(&root) {
            window.mark_serial_as_committed();
            window.on_commit();
        }

        // TODO: maps here, is that good?
        self.pinnacle.move_surface_if_resized(surface);

        // Root surface commit
        if surface == &root {
            // Unmapped window commit
            if let Some(unmapped_window) = self.pinnacle.unmapped_window_for_surface(surface) {
                let Some(is_mapped) =
                    with_renderer_surface_state(surface, |state| state.buffer().is_some())
                else {
                    unreachable!("on_commit_buffer_handler was called previously");
                };

                // Unmapped window has become mapped
                if is_mapped {
                    unmapped_window.on_commit();

                    if let Some(toplevel) = unmapped_window.toplevel() {
                        let hook_id =
                            add_pre_commit_hook(toplevel.wl_surface(), snapshot_pre_commit_hook);

                        unmapped_window
                            .with_state_mut(|state| state.snapshot_hook_id = Some(hook_id));
                    }

                    let output = self.pinnacle.focused_output().cloned();

                    if let Some(output) = output.as_ref() {
                        output.with_state_mut(|state| {
                            state.focus_stack.set_focus(unmapped_window.clone())
                        });

                        self.capture_snapshots_on_output(output, []);
                    }

                    unmapped_window.with_state_mut(|state| {
                        if state.floating_size.is_none() {
                            state.floating_size = Some(unmapped_window.geometry().size);
                        }
                    });

                    // Float windows if necessary
                    if let Some(toplevel) = unmapped_window.toplevel() {
                        let has_parent = toplevel.parent().is_some();
                        let (min_size, max_size) =
                            compositor::with_states(toplevel.wl_surface(), |states| {
                                let mut guard = states.cached_state.get::<SurfaceCachedState>();
                                let state = guard.current();
                                (state.min_size, state.max_size)
                            });

                        let requests_constrained_size = min_size.w > 0
                            && min_size.h > 0
                            && (min_size.w == max_size.w || min_size.h == max_size.h);

                        let should_float = has_parent || requests_constrained_size;

                        if should_float {
                            unmapped_window.with_state_mut(|state| {
                                state.window_state.set_floating(true);
                            });
                        }
                    }

                    self.pinnacle.update_window_state(&unmapped_window);

                    self.pinnacle
                        .unmapped_windows
                        .retain(|win| win != unmapped_window);
                    self.pinnacle.windows.push(unmapped_window.clone());

                    self.pinnacle.raise_window(unmapped_window.clone(), true);

                    if let Some(focused_output) = output {
                        if unmapped_window.is_on_active_tag() {
                            self.update_keyboard_focus(&focused_output);

                            if unmapped_window.with_state(|state| state.window_state.is_floating())
                            {
                                // TODO: make this sync with commit
                                let loc = unmapped_window
                                    .with_state(|state| state.floating_loc)
                                    .unwrap();
                                self.pinnacle.space.map_element(
                                    unmapped_window.clone(),
                                    loc.to_i32_round(),
                                    true,
                                );
                                unmapped_window
                                    .toplevel()
                                    .expect("unreachable")
                                    .send_pending_configure();
                            } else {
                                self.pinnacle.begin_layout_transaction(&focused_output);
                                self.pinnacle.request_layout(&focused_output);
                            }

                            // It seems wlcs needs immediate frame sends for client tests to work
                            #[cfg(feature = "testing")]
                            unmapped_window.send_frame(
                                &focused_output,
                                self.pinnacle.clock.now(),
                                Some(std::time::Duration::ZERO),
                                |_, _| None,
                            );
                        }
                    }
                } else {
                    if let Some(output) = self.pinnacle.focused_output().cloned() {
                        self.pinnacle
                            .place_window_on_output(&unmapped_window, &output);
                    }
                    self.pinnacle.apply_window_rules(&unmapped_window);

                    // TODO: may be able to update_window_state here instead
                    if unmapped_window.with_state(|state| state.window_state.is_floating()) {
                        if let Some(size) = unmapped_window.with_state(|state| state.floating_size)
                        {
                            if let Some(toplevel) = unmapped_window.toplevel() {
                                toplevel.with_pending_state(|state| state.size = Some(size));
                            }
                        }
                    }
                    // Still unmapped
                    unmapped_window.on_commit();
                    self.pinnacle.ensure_initial_configure(surface);
                }

                return;
            }

            // Window surface commit
            if let Some(window) = self.pinnacle.window_for_surface(surface) {
                if window.is_wayland() {
                    let Some(is_mapped) =
                        with_renderer_surface_state(surface, |state| state.buffer().is_some())
                    else {
                        unreachable!("on_commit_buffer_handler was called previously");
                    };

                    window.on_commit();

                    // Toplevel has become unmapped,
                    // see https://wayland.app/protocols/xdg-shell#xdg_toplevel
                    if !is_mapped {
                        if let Some(hook_id) =
                            window.with_state_mut(|state| state.snapshot_hook_id.take())
                        {
                            compositor::remove_pre_commit_hook(surface, hook_id);
                        }

                        let output = window.output(&self.pinnacle);

                        if let Some(output) = output.as_ref() {
                            self.capture_snapshots_on_output(output, []);
                            self.pinnacle.begin_layout_transaction(output);
                        }

                        self.pinnacle.remove_window(&window, true);

                        if let Some(output) = output {
                            self.update_keyboard_focus(&output);
                            self.pinnacle.request_layout(&output);
                        }
                    }

                    // Update reactive popups
                    for (popup, _) in PopupManager::popups_for_surface(surface) {
                        if let PopupKind::Xdg(popup) = popup {
                            if popup.with_pending_state(|state| state.positioner.reactive) {
                                self.pinnacle.position_popup(&popup);
                                if let Err(err) = popup.send_pending_configure() {
                                    warn!("Failed to configure reactive popup: {err}");
                                }
                            }
                        }
                    }
                }
            }
        }

        // TODO: split this up and don't call every commit
        self.pinnacle.ensure_initial_configure(surface);

        self.pinnacle.popup_manager.commit(surface);

        let outputs = if let Some(window) = self.pinnacle.window_for_surface(surface) {
            self.pinnacle.space.outputs_for_element(&window) // surface is a window
        } else if let Some(window) = self.pinnacle.window_for_surface(&root) {
            self.pinnacle.space.outputs_for_element(&window) // surface's root is a window
        } else if let Some(ref popup @ PopupKind::Xdg(ref surf)) =
            self.pinnacle.popup_manager.find_popup(surface)
        {
            let size = surf.with_pending_state(|state| state.geometry.size);
            let loc = find_popup_root_surface(popup)
                .ok()
                .and_then(|surf| self.pinnacle.window_for_surface(&surf))
                .and_then(|win| self.pinnacle.space.element_location(&win));

            if let Some(loc) = loc {
                let geo = Rectangle::from_loc_and_size(loc, size);
                let outputs = self
                    .pinnacle
                    .space
                    .outputs()
                    .filter_map(|output| {
                        let op_geo = self.pinnacle.space.output_geometry(output);
                        op_geo.and_then(|op_geo| op_geo.overlaps_or_touches(geo).then_some(output))
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                outputs
            } else {
                let layer_output = find_popup_root_surface(popup)
                    .ok()
                    .and_then(|surf| {
                        self.pinnacle.space.outputs().find(|op| {
                            let map = layer_map_for_output(op);
                            map.layer_for_surface(&surf, WindowSurfaceType::TOPLEVEL)
                                .is_some()
                        })
                    })
                    .cloned();
                layer_output.map(|op| vec![op]).unwrap_or_default()
            }
        } else if let Some(output) = self
            .pinnacle
            .space
            .outputs()
            .find(|op| {
                let layer_map = layer_map_for_output(op);
                layer_map
                    .layer_for_surface(surface, WindowSurfaceType::ALL)
                    .is_some()
            })
            .cloned()
        {
            vec![output] // surface is a layer surface
        } else if matches!(self.pinnacle.cursor_state.cursor_image(), CursorImageStatus::Surface(s) if s == surface)
        {
            // This is a cursor surface
            // TODO: granular
            self.pinnacle.space.outputs().cloned().collect()
        } else if self.pinnacle.dnd_icon.as_ref() == Some(surface) {
            // This is a dnd icon
            // TODO: granular
            self.pinnacle.space.outputs().cloned().collect()
        } else if let Some(output) = self
            .pinnacle
            .space
            .outputs()
            .find(|op| {
                op.with_state(|state| {
                    state
                        .lock_surface
                        .as_ref()
                        .is_some_and(|lock| lock.wl_surface() == surface)
                })
            })
            .cloned()
        {
            vec![output] // surface is a lock surface
        } else {
            return;
        };

        for output in outputs {
            self.schedule_render(&output);
        }
    }

    fn destroyed(&mut self, surface: &WlSurface) {
        let Some(root_surface) = self.pinnacle.root_surface_cache.get(surface) else {
            return;
        };
        let Some(window) = self.pinnacle.window_for_surface(root_surface) else {
            return;
        };
        let Some(output) = window.output(&self.pinnacle) else {
            return;
        };
        let Some(loc) = self.pinnacle.space.element_location(&window) else {
            return;
        };

        let loc = loc - output.current_location();

        self.backend.with_renderer(|renderer| {
            window.capture_snapshot_and_store(
                renderer,
                loc,
                output.current_scale().fractional_scale().into(),
                1.0,
            );
        });

        self.pinnacle
            .root_surface_cache
            .retain(|surf, root| surf != surface && root != surface);
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

impl Pinnacle {
    fn ensure_initial_configure(&mut self, surface: &WlSurface) {
        if let Some(window) = self.unmapped_window_for_surface(surface) {
            if let Some(toplevel) = window.toplevel() {
                let initial_configure_sent = compositor::with_states(surface, |states| {
                    states
                        .data_map
                        .get::<XdgToplevelSurfaceData>()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .initial_configure_sent
                });

                if !initial_configure_sent {
                    tracing::debug!("Initial configure on wl_surface {:?}", surface.id());
                    toplevel.send_configure();
                }
            }
            return;
        }

        if let Some(popup) = self.popup_manager.find_popup(surface) {
            let PopupKind::Xdg(popup) = &popup else { return };
            let initial_configure_sent = compositor::with_states(surface, |states| {
                states
                    .data_map
                    .get::<XdgPopupSurfaceData>()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .initial_configure_sent
            });
            if !initial_configure_sent {
                popup.send_configure().expect(
                    "sent configure for popup that doesn't allow multiple or is nonreactive",
                );
            }
            return;
        }

        let output = self
            .space
            .outputs()
            .find(|op| {
                let map = layer_map_for_output(op);
                map.layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
                    .is_some()
            })
            .cloned();

        if let Some(output) = output {
            if layer_map_for_output(&output).arrange() {
                self.request_layout(&output);
            }

            let initial_configure_sent = compositor::with_states(surface, |states| {
                states
                    .data_map
                    .get::<LayerSurfaceData>()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .initial_configure_sent
            });

            if !initial_configure_sent {
                layer_map_for_output(&output)
                    .layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
                    .expect("no layer for surface")
                    .layer_surface()
                    .send_configure();
            }
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
        self.pinnacle.dnd_icon = icon;
    }

    fn dropped(&mut self, _target: Option<WlSurface>, _validated: bool, _seat: Seat<Self>) {
        self.pinnacle.dnd_icon = None;
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
        if let Some(xwm) = self.pinnacle.xwm.as_mut() {
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
        if let Some(xwm) = self.pinnacle.xwm.as_mut() {
            if let Err(err) =
                xwm.send_selection(ty, mime_type, fd, self.pinnacle.loop_handle.clone())
            {
                tracing::warn!(?err, "Failed to send primary (X11 -> Wayland)");
            }
        }
    }
}

impl DataDeviceHandler for State {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.pinnacle.data_device_state
    }
}
delegate_data_device!(State);

impl PrimarySelectionHandler for State {
    fn primary_selection_state(&self) -> &PrimarySelectionState {
        &self.pinnacle.primary_selection_state
    }
}
delegate_primary_selection!(State);

impl DataControlHandler for State {
    fn data_control_state(&self) -> &DataControlState {
        &self.pinnacle.data_control_state
    }
}
delegate_data_control!(State);

impl SeatHandler for State {
    type KeyboardFocus = KeyboardFocusTarget;
    type PointerFocus = PointerFocusTarget;
    type TouchFocus = PointerFocusTarget;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.pinnacle.seat_state
    }

    fn cursor_image(&mut self, _seat: &Seat<Self>, image: CursorImageStatus) {
        self.pinnacle.cursor_state.set_cursor_image(image);
    }

    fn focus_changed(&mut self, seat: &Seat<Self>, focused: Option<&Self::KeyboardFocus>) {
        let focus_client = focused.and_then(|foc_target| {
            self.pinnacle
                .display_handle
                .get_client(foc_target.wl_surface()?.id())
                .ok()
        });
        set_data_device_focus(&self.pinnacle.display_handle, seat, focus_client.clone());
        set_primary_focus(&self.pinnacle.display_handle, seat, focus_client);
    }

    fn led_state_changed(&mut self, _seat: &Seat<Self>, led_state: LedState) {
        for device in self.pinnacle.input_state.libinput_devices.iter_mut() {
            device.led_update(led_state.into());
        }
    }
}
delegate_seat!(State);

impl ShmHandler for State {
    fn shm_state(&self) -> &ShmState {
        &self.pinnacle.shm_state
    }
}
delegate_shm!(State);

impl OutputHandler for State {
    fn output_bound(&mut self, output: Output, wl_output: WlOutput) {
        crate::protocol::foreign_toplevel::on_output_bound(self, &output, &wl_output);
    }
}
delegate_output!(State);

delegate_viewporter!(State);

impl FractionalScaleHandler for State {
    fn new_fractional_scale(&mut self, surface: WlSurface) {
        // comment yanked from anvil
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
                                        self.pinnacle.window_for_surface(&root).and_then(|window| {
                                            self.pinnacle
                                                .space
                                                .outputs_for_element(&window)
                                                .first()
                                                .cloned()
                                        })
                                    })
                            })
                        } else {
                            self.pinnacle.window_for_surface(&root).and_then(|window| {
                                self.pinnacle
                                    .space
                                    .outputs_for_element(&window)
                                    .first()
                                    .cloned()
                            })
                        }
                    })
                    .or_else(|| self.pinnacle.space.outputs().next().cloned());
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
        &mut self.pinnacle.layer_shell_state
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
            .or_else(|| self.pinnacle.focused_output().cloned());

        let Some(output) = output else {
            error!("New layer surface, but there was no output to map it on");
            return;
        };

        if let Err(err) =
            layer_map_for_output(&output).map_layer(&desktop::LayerSurface::new(surface, namespace))
        {
            error!("Failed to map layer surface: {err}");
        };
    }

    fn layer_destroyed(&mut self, surface: wlr_layer::LayerSurface) {
        let mut output: Option<Output> = None;
        if let Some((mut map, layer, op)) = self.pinnacle.space.outputs().find_map(|o| {
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

        let focused_output = self.pinnacle.focused_output().cloned();

        if let Some(focused_output) = focused_output {
            self.update_keyboard_focus(&focused_output);
        }

        if let Some(output) = output {
            self.pinnacle.loop_handle.insert_idle(move |state| {
                state.pinnacle.request_layout(&output);
            });
        }
    }

    fn new_popup(&mut self, _parent: wlr_layer::LayerSurface, popup: PopupSurface) {
        trace!("WlrLayerShellHandler::new_popup");
        self.pinnacle.position_popup(&popup);
    }
}
delegate_layer_shell!(State);

impl ScreencopyHandler for State {
    fn frame(&mut self, frame: Screencopy) {
        let output = frame.output().clone();
        if !frame.with_damage() {
            self.schedule_render(&output);
        }
        output.with_state_mut(|state| state.screencopy.replace(frame));
    }
}
delegate_screencopy!(State);

impl GammaControlHandler for State {
    fn gamma_control_manager_state(&mut self) -> &mut GammaControlManagerState {
        &mut self.pinnacle.gamma_control_manager_state
    }

    fn get_gamma_size(&mut self, output: &Output) -> Option<u32> {
        let Backend::Udev(udev) = &self.backend else {
            return None;
        };

        match udev.gamma_size(output) {
            Ok(0) => None, // Setting gamma is not supported
            Ok(size) => Some(size),
            Err(err) => {
                warn!(
                    "Failed to get gamma size for output {}: {err}",
                    output.name()
                );
                None
            }
        }
    }

    fn set_gamma(&mut self, output: &Output, gammas: [&[u16]; 3]) -> bool {
        let Backend::Udev(udev) = &mut self.backend else {
            warn!("Setting gamma is not supported on the winit backend");
            return false;
        };

        match udev.set_gamma(output, Some(gammas)) {
            Ok(_) => true,
            Err(err) => {
                warn!("Failed to set gamma for output {}: {err}", output.name());
                false
            }
        }
    }

    fn gamma_control_destroyed(&mut self, output: &Output) {
        let Backend::Udev(udev) = &mut self.backend else {
            warn!("Resetting gamma is not supported on the winit backend");
            return;
        };

        if let Err(err) = udev.set_gamma(output, None) {
            warn!("Failed to set gamma for output {}: {err}", output.name());
        }
    }
}
delegate_gamma_control!(State);

impl SecurityContextHandler for State {
    fn context_created(&mut self, source: SecurityContextListenerSource, context: SecurityContext) {
        self.pinnacle
            .loop_handle
            .insert_source(source, move |client, _, state| {
                let client_state = Arc::new(ClientState {
                    is_restricted: true,
                    ..Default::default()
                });

                if let Err(err) = state
                    .pinnacle
                    .display_handle
                    .insert_client(client, client_state)
                {
                    warn!("Failed to insert a restricted client: {err}");
                } else {
                    trace!("Inserted a restricted client, context={context:?}");
                }
            })
            .expect("Failed to insert security context listener source into event loop");
    }
}
delegate_security_context!(State);

impl PointerConstraintsHandler for State {
    fn new_constraint(&mut self, _surface: &WlSurface, pointer: &PointerHandle<Self>) {
        self.pinnacle
            .maybe_activate_pointer_constraint(pointer.current_location());
    }

    fn cursor_position_hint(
        &mut self,
        surface: &WlSurface,
        pointer: &PointerHandle<Self>,
        location: Point<f64, Logical>,
    ) {
        if with_pointer_constraint(surface, pointer, |constraint| {
            constraint.is_some_and(|c| c.is_active())
        }) {
            // TODO: cache the current focus's location so you don't have to get it on demand
            // through the current pointer location
            let Some((current_focus, current_focus_loc)) = self
                .pinnacle
                .pointer_focus_target_under(pointer.current_location())
            else {
                return;
            };

            if current_focus.wl_surface().as_deref() != Some(surface) {
                return;
            }

            pointer.set_location(current_focus_loc + location);
        }
    }
}
delegate_pointer_constraints!(State);

impl XWaylandShellHandler for State {
    fn xwayland_shell_state(&mut self) -> &mut XWaylandShellState {
        &mut self.pinnacle.xwayland_shell_state
    }
}
delegate_xwayland_shell!(State);

impl OutputManagementHandler for State {
    fn output_management_manager_state(&mut self) -> &mut OutputManagementManagerState {
        &mut self.pinnacle.output_management_manager_state
    }

    fn apply_configuration(&mut self, config: HashMap<Output, OutputConfiguration>) -> bool {
        for (output, config) in config {
            match config {
                OutputConfiguration::Disabled => {
                    self.pinnacle.set_output_enabled(&output, false);
                    // TODO: split
                    self.backend
                        .set_output_powered(&output, &self.pinnacle.loop_handle, false);
                }
                OutputConfiguration::Enabled {
                    mode,
                    position,
                    transform,
                    scale,
                    adaptive_sync: _,
                } => {
                    self.pinnacle.set_output_enabled(&output, true);
                    // TODO: split
                    self.backend
                        .set_output_powered(&output, &self.pinnacle.loop_handle, true);

                    self.capture_snapshots_on_output(&output, []);

                    let mode = mode.map(|(size, refresh)| {
                        if let Some(refresh) = refresh {
                            Mode {
                                size,
                                refresh: refresh.get() as i32,
                            }
                        } else {
                            output
                                .with_state(|state| {
                                    state
                                        .modes
                                        .iter()
                                        .filter(|mode| mode.size == size)
                                        .max_by_key(|mode| mode.refresh)
                                        .copied()
                                })
                                .unwrap_or(Mode {
                                    size,
                                    refresh: 60_000,
                                })
                        }
                    });

                    self.pinnacle.change_output_state(
                        &mut self.backend,
                        &output,
                        mode.map(OutputMode::Smithay),
                        transform,
                        scale.map(Scale::Fractional),
                        position,
                    );

                    self.pinnacle.begin_layout_transaction(&output);
                    self.pinnacle.request_layout(&output);

                    self.schedule_render(&output);
                }
            }
        }
        self.pinnacle
            .output_management_manager_state
            .update::<State>();
        true
    }

    fn test_configuration(&mut self, config: HashMap<Output, OutputConfiguration>) -> bool {
        debug!(?config);
        true
    }
}
delegate_output_management!(State);

impl OutputPowerManagementHandler for State {
    fn output_power_management_state(&mut self) -> &mut OutputPowerManagementState {
        &mut self.pinnacle.output_power_management_state
    }

    fn set_mode(&mut self, output: &Output, powered: bool) {
        self.backend
            .set_output_powered(output, &self.pinnacle.loop_handle, powered);

        if powered {
            self.schedule_render(output);
        }
    }
}
delegate_output_power_management!(State);

impl TabletSeatHandler for State {
    fn tablet_tool_image(&mut self, tool: &TabletToolDescriptor, image: CursorImageStatus) {
        // TODO:
        let _ = tool;
        let _ = image;
    }
}
delegate_tablet_manager!(State);

delegate_cursor_shape!(State);

impl KeyboardShortcutsInhibitHandler for State {
    fn keyboard_shortcuts_inhibit_state(&mut self) -> &mut KeyboardShortcutsInhibitState {
        &mut self.pinnacle.keyboard_shortcuts_inhibit_state
    }

    fn new_inhibitor(&mut self, inhibitor: KeyboardShortcutsInhibitor) {
        // TODO: Some way to not unconditionally activate the inhibitor
        inhibitor.activate();
    }
}
delegate_keyboard_shortcuts_inhibit!(State);

impl XWaylandKeyboardGrabHandler for State {
    fn keyboard_focus_for_xsurface(&self, surface: &WlSurface) -> Option<Self::KeyboardFocus> {
        self.pinnacle
            .window_for_surface(surface)
            .map(KeyboardFocusTarget::from)
    }
}
delegate_xwayland_keyboard_grab!(State);

enum ActivationContext {
    FocusIfPossible,
    UrgentOnly,
}

impl XdgActivationHandler for State {
    fn activation_state(&mut self) -> &mut XdgActivationState {
        &mut self.pinnacle.xdg_activation_state
    }

    fn token_created(&mut self, token: XdgActivationToken, data: XdgActivationTokenData) -> bool {
        let Some((serial, seat)) = data.serial else {
            data.user_data
                .insert_if_missing(|| ActivationContext::UrgentOnly);
            debug!(
                ?token,
                "xdg-activation: created urgent-only token for missing seat/serial"
            );
            return true;
        };

        let Some(seat) = Seat::<State>::from_resource(&seat) else {
            data.user_data
                .insert_if_missing(|| ActivationContext::UrgentOnly);
            debug!(
                ?token,
                "xdg-activation: created urgent-only token for unknown seat"
            );
            return true;
        };

        let keyboard = seat.get_keyboard().unwrap();

        let valid = keyboard
            .last_enter()
            .is_some_and(|last_enter| serial.is_no_older_than(&last_enter));

        if valid {
            data.user_data
                .insert_if_missing(|| ActivationContext::FocusIfPossible);
            debug!(?token, "xdg-activation: created focus-if-possible token");
        } else {
            debug!(?token, "xdg-activation: invalid token");
        }

        valid
    }

    fn request_activation(
        &mut self,
        _token: XdgActivationToken,
        token_data: XdgActivationTokenData,
        surface: WlSurface,
    ) {
        let Some(context) = token_data.user_data.get::<ActivationContext>() else {
            debug!("xdg-activation: request without context");
            return;
        };

        let Some(window) = self.pinnacle.window_for_surface(&surface) else {
            debug!("xdg-activation: no window for request");
            return;
        };

        match context {
            ActivationContext::FocusIfPossible => {
                if window.is_on_active_tag() {
                    // TODO: add a holder for pending activations like cosmic-comp

                    let Some(output) = window.output(&self.pinnacle) else {
                        // TODO: make "no tags" be all tags on an output
                        debug!("xdg-activation: focus-if-possible request on window but it had no tags");
                        return;
                    };

                    self.pinnacle.raise_window(window.clone(), true);

                    output.with_state_mut(|state| {
                        state.focus_stack.set_focus(window);
                    });

                    self.update_keyboard_focus(&output);

                    self.schedule_render(&output);
                }
            }
            ActivationContext::UrgentOnly => {
                // TODO: add urgent state to windows, use in a focus border/taskbar flash
            }
        }
    }
}
delegate_xdg_activation!(State);

delegate_pointer_gestures!(State);

delegate_single_pixel_buffer!(State);

impl Pinnacle {
    fn position_popup(&self, popup: &PopupSurface) {
        trace!("State::position_popup");
        let Ok(root) = find_popup_root_surface(&PopupKind::Xdg(popup.clone())) else {
            return;
        };

        let mut positioner = popup.with_pending_state(|state| mem::take(&mut state.positioner));

        let popup_geo = (|| -> Option<Rectangle<i32, Logical>> {
            let parent = popup.get_parent_surface()?;

            if parent == root {
                // Slide toplevel popup x's instead of flipping; this mimics Awesome
                positioner
                    .constraint_adjustment
                    .remove(ConstraintAdjustment::FlipX);
            }

            let (root_global_loc, output) = if let Some(win) = self.window_for_surface(&root) {
                let win_geo = self.space.element_geometry(&win)?;
                (win_geo.loc, self.focused_output()?.clone())
            } else {
                self.space.outputs().find_map(|op| {
                    let layer_map = layer_map_for_output(op);
                    let layer = layer_map.layer_for_surface(&root, WindowSurfaceType::TOPLEVEL)?;
                    let output_loc = self.space.output_geometry(op)?.loc;
                    Some((
                        layer_map.layer_geometry(layer)?.loc + output_loc,
                        op.clone(),
                    ))
                })?
            };

            let parent_global_loc = if root == parent {
                root_global_loc
            } else {
                root_global_loc + get_popup_toplevel_coords(&PopupKind::Xdg(popup.clone()))
            };

            let mut output_geo = self.space.output_geometry(&output)?;

            // Make local to parent
            output_geo.loc -= parent_global_loc;
            Some(positioner.get_unconstrained_geometry(output_geo))
        })()
        .unwrap_or_else(|| positioner.get_geometry());

        popup.with_pending_state(|state| {
            state.geometry = popup_geo;
            state.positioner = positioner;
        });
    }

    // From Niri
    /// Attempt to activate any pointer constraint on the pointer focus at `new_pos`.
    pub fn maybe_activate_pointer_constraint(&self, new_pos: Point<f64, Logical>) {
        let Some((surface, surface_loc)) = self.pointer_focus_target_under(new_pos) else {
            return;
        };
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        let Some(surface) = surface.wl_surface() else {
            return;
        };
        with_pointer_constraint(&surface, &pointer, |constraint| {
            let Some(constraint) = constraint else { return };

            if constraint.is_active() {
                return;
            }

            // Constraint does not apply if not within region.
            if let Some(region) = constraint.region() {
                let new_pos_surface_local = new_pos - surface_loc;
                if !region.contains(new_pos_surface_local.to_i32_round()) {
                    return;
                }
            }

            constraint.activate();
        });
    }
}
