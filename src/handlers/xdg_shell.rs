use smithay::{
    delegate_xdg_shell,
    desktop::{
        PopupKeyboardGrab, PopupKind, PopupPointerGrab, PopupUngrabStrategy, Window,
        WindowSurfaceType, find_popup_root_surface, layer_map_for_output,
    },
    input::{Seat, pointer::Focus},
    reexports::{
        calloop::Interest,
        wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge,
        wayland_server::{
            Resource,
            protocol::{wl_output::WlOutput, wl_seat::WlSeat},
        },
    },
    utils::{HookId, Serial},
    wayland::{
        compositor::{
            self, BufferAssignment, CompositorHandler, SurfaceAttributes, add_pre_commit_hook,
        },
        shell::xdg::{
            PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
            XdgToplevelSurfaceData,
        },
    },
};
use tracing::{error, field::Empty, trace, trace_span, warn};

use crate::{
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

    fn fullscreen_request(&mut self, surface: ToplevelSurface, _wl_output: Option<WlOutput>) {
        let _span = tracy_client::span!("XdgShellHandler::fullscreen_request");

        // TODO: Respect client output preference

        if let Some(window) = self
            .pinnacle
            .window_for_surface(surface.wl_surface())
            .cloned()
        {
            self.update_window_layout_mode_and_layout(&window, |layout_mode| {
                layout_mode.set_client_fullscreen(true);
            });
        } else if let Some(unmapped) = self
            .pinnacle
            .unmapped_window_for_surface_mut(surface.wl_surface())
        {
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
            self.update_window_layout_mode_and_layout(&window, |layout_mode| {
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
            self.update_window_layout_mode_and_layout(&window, |layout_mode| {
                layout_mode.set_client_maximized(true);
            });
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
            self.update_window_layout_mode_and_layout(&window, |layout_mode| {
                layout_mode.set_client_maximized(false);
            });
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
}
delegate_xdg_shell!(State);

/// Adds a pre-commit hook for mapped toplevels that blocks windows when transactions are pending.
///
/// It also takes over the role of the default dmabuf pre-commit hook, so when adding this
/// be sure to remove the default hook.
//
// Yoinked from niri
pub fn add_mapped_toplevel_pre_commit_hook(toplevel: &ToplevelSurface) -> HookId {
    add_pre_commit_hook::<State, _>(toplevel.wl_surface(), move |state, _dh, surface| {
        let _span = tracy_client::span!("mapped toplevel pre-commit");
        let span =
            trace_span!("toplevel pre-commit", surface = %surface.id(), serial = Empty).entered();

        let Some(window) = state.pinnacle.window_for_surface(surface) else {
            error!("pre-commit hook for mapped surfaces must be removed upon unmapping");
            return;
        };

        let (got_unmapped, dmabuf, commit_serial) = compositor::with_states(surface, |states| {
            let (got_unmapped, dmabuf) = {
                let mut guard = states.cached_state.get::<SurfaceAttributes>();
                match guard.pending().buffer.as_ref() {
                    Some(BufferAssignment::NewBuffer(buffer)) => {
                        let dmabuf = smithay::wayland::dmabuf::get_dmabuf(buffer).cloned().ok();
                        (false, dmabuf)
                    }
                    Some(BufferAssignment::Removed) => (true, None),
                    None => (false, None),
                }
            };

            let role = states
                .data_map
                .get::<XdgToplevelSurfaceData>()
                .unwrap()
                .lock()
                .unwrap();

            (got_unmapped, dmabuf, role.configure_serial)
        });

        let mut transaction_for_dmabuf = None;
        if let Some(serial) = commit_serial {
            if !span.is_disabled() {
                span.record("serial", format!("{serial:?}"));
            }

            trace!("taking pending transaction");
            if let Some(transaction) = window.take_pending_transaction(serial) {
                // Transaction can be already completed if it ran past the deadline.
                if !transaction.is_completed() {
                    let is_last = transaction.is_last();

                    // If this is the last transaction, we don't need to add a separate
                    // notification, because the transaction will complete in our dmabuf blocker
                    // callback, which already calls blocker_cleared(), or by the end of this
                    // function, in which case there would be no blocker in the first place.
                    if !is_last {
                        // Waiting for some other surface; register a notification and add a
                        // transaction blocker.

                        if let Some(client) = surface.client() {
                            transaction.add_notification(
                                state.pinnacle.blocker_cleared_tx.clone(),
                                client.clone(),
                            );
                            compositor::add_blocker(surface, transaction.blocker());
                        }
                    }

                    // Delay dropping (and completing) the transaction until the dmabuf is ready.
                    // If there's no dmabuf, this will be dropped by the end of this pre-commit
                    // hook.
                    transaction_for_dmabuf = Some(transaction);
                }
            }
        } else {
            error!("commit on a mapped surface without a configured serial");
        };

        if let Some((blocker, source)) =
            dmabuf.and_then(|dmabuf| dmabuf.generate_blocker(Interest::READ).ok())
        {
            if let Some(client) = surface.client() {
                let res = state
                    .pinnacle
                    .loop_handle
                    .insert_source(source, move |_, _, state| {
                        // This surface is now ready for the transaction.
                        drop(transaction_for_dmabuf.take());

                        let display_handle = state.pinnacle.display_handle.clone();
                        state
                            .client_compositor_state(&client)
                            .blocker_cleared(state, &display_handle);

                        Ok(())
                    });
                if res.is_ok() {
                    compositor::add_blocker(surface, blocker);
                    trace!("added dmabuf blocker");
                }
            }
        }

        if got_unmapped {
            let Some(output) = window.output(&state.pinnacle) else {
                return;
            };

            state.backend.with_renderer(|renderer| {
                window.capture_snapshot_and_store(
                    renderer,
                    output.current_scale().fractional_scale().into(),
                    1.0,
                );
            });
        } else {
            window.with_state_mut(|state| state.snapshot.take());
        }
    })
}
