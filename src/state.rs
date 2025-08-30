// SPDX-License-Identifier: GPL-3.0-or-later

#[cfg(feature = "snowcap")]
use crate::protocol::snowcap_decoration::SnowcapDecorationState;
use crate::{
    api::signal::SignalState,
    backend::{
        self, Backend,
        udev::{SurfaceDmabufFeedback, Udev},
        winit::Winit,
    },
    cli::{self, Cli},
    config::Config,
    cursor::CursorState,
    focus::{OutputFocusStack, WindowKeyboardFocusStack, pointer::PointerContents},
    handlers::{
        session_lock::LockState, xdg_activation::XDG_ACTIVATION_TOKEN_TIMEOUT,
        xwayland::XwaylandState,
    },
    layout::LayoutState,
    process::ProcessState,
    protocol::{
        foreign_toplevel::{self, ForeignToplevelManagerState},
        gamma_control::GammaControlManagerState,
        output_management::OutputManagementManagerState,
        output_power_management::OutputPowerManagementState,
        screencopy::ScreencopyManagerState,
    },
    window::{Unmapped, WindowElement, ZIndexElement, rules::WindowRuleState},
};
use smithay::{
    backend::renderer::element::{
        RenderElementState, RenderElementStates, utils::select_dmabuf_feedback,
    },
    desktop::{
        LayerSurface, PopupManager, Space, layer_map_for_output,
        utils::{
            send_dmabuf_feedback_surface_tree, send_frames_surface_tree,
            surface_primary_scanout_output, update_surface_primary_scanout_output,
        },
    },
    input::{Seat, SeatState, keyboard::XkbConfig, pointer::CursorImageStatus},
    output::Output,
    reexports::{
        calloop::{
            Interest, LoopHandle, LoopSignal, Mode, PostAction,
            generic::Generic,
            timer::{TimeoutAction, Timer},
        },
        wayland_protocols::xdg::shell::server::xdg_toplevel::WmCapabilities,
        wayland_protocols_misc::server_decoration::server::org_kde_kwin_server_decoration_manager,
        wayland_server::{
            Client, Display, DisplayHandle,
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::wl_surface::WlSurface,
        },
    },
    utils::{Clock, HookId, Monotonic},
    wayland::{
        compositor::{
            self, CompositorClientState, CompositorHandler, CompositorState, SurfaceData,
            with_surface_tree_downward,
        },
        cursor_shape::CursorShapeManagerState,
        foreign_toplevel_list::ForeignToplevelListState,
        fractional_scale::{FractionalScaleManagerState, with_fractional_scale},
        idle_inhibit::IdleInhibitManagerState,
        idle_notify::IdleNotifierState,
        keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitState,
        output::OutputManagerState,
        pointer_constraints::PointerConstraintsState,
        pointer_gestures::PointerGesturesState,
        relative_pointer::RelativePointerManagerState,
        security_context::SecurityContextState,
        selection::{
            data_device::DataDeviceState, ext_data_control,
            primary_selection::PrimarySelectionState, wlr_data_control,
        },
        session_lock::{LockSurface, SessionLockManagerState},
        shell::{
            kde::decoration::KdeDecorationState,
            wlr_layer::WlrLayerShellState,
            xdg::{XdgShellState, decoration::XdgDecorationState},
        },
        shm::ShmState,
        single_pixel_buffer::SinglePixelBufferState,
        socket::ListeningSocketSource,
        tablet_manager::TabletManagerState,
        viewporter::ViewporterState,
        xdg_activation::XdgActivationState,
        xwayland_keyboard_grab::XWaylandKeyboardGrabState,
        xwayland_shell::XWaylandShellState,
    },
    xwayland::XWaylandClientData,
};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    ffi::OsString,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use sysinfo::{ProcessRefreshKind, RefreshKind};
use tracing::{info, warn};
use xdg::BaseDirectories;

use crate::input::InputState;

#[cfg(feature = "testing")]
use crate::backend::dummy::Dummy;

// We'll try to send frame callbacks at least once a second. We'll make a timer that fires once a
// second, so with the worst timing the maximum interval between two frame callbacks for a surface
// should be ~1.995 seconds.
const FRAME_CALLBACK_THROTTLE: Option<Duration> = Some(Duration::from_millis(995));

/// The main state of the application.
pub struct State {
    /// Which backend is currently running
    pub backend: Backend,
    pub pinnacle: Pinnacle,
}

pub struct Pinnacle {
    /// A loop signal used to stop the compositor
    pub loop_signal: LoopSignal,
    /// A handle to the event loop
    pub loop_handle: LoopHandle<'static, State>,
    pub display_handle: DisplayHandle,
    pub clock: Clock<Monotonic>,
    pub socket_name: OsString,

    pub space: Space<WindowElement>,

    pub seat: Seat<State>,

    pub compositor_state: CompositorState,
    pub data_device_state: DataDeviceState,
    pub seat_state: SeatState<State>,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub xdg_shell_state: XdgShellState,
    pub viewporter_state: ViewporterState,
    pub fractional_scale_manager_state: FractionalScaleManagerState,
    pub primary_selection_state: PrimarySelectionState,
    pub layer_shell_state: WlrLayerShellState,
    pub wlr_data_control_state: wlr_data_control::DataControlState,
    pub ext_data_control_state: ext_data_control::DataControlState,
    pub screencopy_manager_state: ScreencopyManagerState,
    pub gamma_control_manager_state: GammaControlManagerState,
    pub security_context_state: SecurityContextState,
    pub relative_pointer_manager_state: RelativePointerManagerState,
    pub pointer_constraints_state: PointerConstraintsState,
    pub foreign_toplevel_manager_state: ForeignToplevelManagerState,
    pub session_lock_manager_state: SessionLockManagerState,
    pub xwayland_shell_state: XWaylandShellState,
    pub idle_notifier_state: IdleNotifierState<State>,
    pub idle_inhibit_manager_state: IdleInhibitManagerState,
    pub output_management_manager_state: OutputManagementManagerState,
    pub output_power_management_state: OutputPowerManagementState,
    pub tablet_manager_state: TabletManagerState,
    pub keyboard_shortcuts_inhibit_state: KeyboardShortcutsInhibitState,
    pub xwayland_keyboard_grab_state: XWaylandKeyboardGrabState,
    pub xdg_activation_state: XdgActivationState,
    pub xdg_decoration_state: XdgDecorationState,
    pub kde_decoration_state: KdeDecorationState,
    pub pointer_gestures_state: PointerGesturesState,
    pub single_pixel_buffer_state: SinglePixelBufferState,
    pub foreign_toplevel_list_state: ForeignToplevelListState,
    #[cfg(feature = "snowcap")]
    pub snowcap_decoration_state: SnowcapDecorationState,

    pub lock_state: LockState,

    /// The state of key and mousebinds along with libinput settings
    pub input_state: InputState,

    pub outputs: Vec<Output>,
    pub output_focus_stack: OutputFocusStack,

    /// The z-index stack, bottom to top
    pub z_index_stack: Vec<ZIndexElement>,

    pub popup_manager: PopupManager,

    pub dnd_icon: Option<WlSurface>,

    /// The main window vec
    pub windows: Vec<WindowElement>,
    /// Windows with no buffer attached
    pub unmapped_windows: Vec<Unmapped>,
    pub keyboard_focus_stack: WindowKeyboardFocusStack,
    pub on_demand_layer_focus: Option<LayerSurface>,
    pub lock_surface_focus: Option<LockSurface>,

    pub config: Config,

    pub xwayland_state: Option<XwaylandState>,

    pub process_state: ProcessState,

    // Currently only used to keep track of if the server has started
    pub grpc_server_join_handle: Option<tokio::task::JoinHandle<()>>,

    pub xdg_base_dirs: BaseDirectories,

    pub signal_state: SignalState,

    pub layout_state: LayoutState,

    pub window_rule_state: WindowRuleState,

    /// A cache of surfaces to their root surface.
    pub root_surface_cache: HashMap<WlSurface, WlSurface>,

    /// WlSurfaces with an attached idle inhibitor.
    pub idle_inhibiting_surfaces: HashSet<WlSurface>,

    #[cfg(feature = "snowcap")]
    pub snowcap_handle: Option<snowcap::SnowcapHandle>,
    #[cfg(feature = "snowcap")]
    pub snowcap_join_handle: Option<tokio::task::JoinHandle<()>>,

    pub cursor_shape_manager_state: CursorShapeManagerState,
    pub cursor_state: CursorState,

    pub pointer_contents: PointerContents,

    pub blocker_cleared_tx: std::sync::mpsc::Sender<Client>,
    pub blocker_cleared_rx: std::sync::mpsc::Receiver<Client>,

    pub dmabuf_hooks: HashMap<WlSurface, HookId>,
}

#[cfg(feature = "snowcap")]
impl Drop for Pinnacle {
    fn drop(&mut self) {
        if let Some(signal) = self.snowcap_handle.take() {
            signal.stop();
        }
    }
}

impl State {
    pub fn on_event_loop_cycle_completion(&mut self) {
        let _span = tracy_client::span!("State::on_event_loop_cycle_completion");

        self.notify_blocker_cleared();
        self.update_layout();

        self.update_keyboard_focus();
        self.pinnacle.fixup_z_layering();
        self.pinnacle.space.refresh();
        self.pinnacle.update_window_tags();
        self.pinnacle.cursor_state.cleanup();
        self.pinnacle.popup_manager.cleanup();
        self.update_pointer_focus();
        foreign_toplevel::refresh(self);
        self.pinnacle.refresh_idle_inhibit();

        self.backend.render_scheduled_outputs(&mut self.pinnacle);

        #[cfg(feature = "snowcap")]
        if self
            .pinnacle
            .snowcap_join_handle
            .as_ref()
            .is_some_and(|handle| handle.is_finished())
        {
            // If Snowcap is dead, the config has most likely crashed or will crash if it's used.
            // The embedded config will also fail to start.
            // We'll exit here so people aren't stuck in the compositor.
            self.pinnacle.shutdown();
        }

        self.pinnacle
            .display_handle
            .flush_clients()
            .expect("failed to flush client buffers");
    }

    fn notify_blocker_cleared(&mut self) {
        let dh = self.pinnacle.display_handle.clone();
        while let Ok(client) = self.pinnacle.blocker_cleared_rx.try_recv() {
            self.client_compositor_state(&client)
                .blocker_cleared(self, &dh);
        }
    }
}

/// Filters clients that are restricted by the security context
fn filter_restricted_client(client: &Client) -> bool {
    if let Some(state) = client.get_data::<ClientState>() {
        return !state.is_restricted;
    }
    if client.get_data::<XWaylandClientData>().is_some() {
        return true;
    }
    panic!("Unknown client data type");
}

impl Pinnacle {
    pub fn new(
        display: Display<State>,
        loop_signal: LoopSignal,
        loop_handle: LoopHandle<'static, State>,
        seat_name: String,
        config_dir: PathBuf,
        cli: Option<Cli>,
        create_socket: bool,
    ) -> anyhow::Result<Self> {
        let _span = tracy_client::span!("Pinnacle::new");

        let socket_name = if create_socket {
            let socket = ListeningSocketSource::new_auto()?;
            let socket_name = socket.socket_name().to_os_string();

            loop_handle.insert_source(socket, |stream, _metadata, state| {
                state
                    .pinnacle
                    .display_handle
                    .insert_client(stream, Arc::new(ClientState::default()))
                    .expect("Could not insert client into loop handle");
            })?;
            socket_name
        } else {
            OsString::from("funny-socket-name-here")
        };

        let display_handle = display.handle();

        loop_handle.insert_source(
            Generic::new(display, Interest::READ, Mode::Level),
            |_readiness, display, state| {
                // Safety: we don't drop the display
                unsafe {
                    display
                        .get_mut()
                        .dispatch_clients(state)
                        .expect("failed to dispatch clients");
                }
                Ok(PostAction::Continue)
            },
        )?;

        let mut seat_state = SeatState::new();

        let mut seat = seat_state.new_wl_seat(&display_handle, seat_name);
        seat.add_pointer();

        seat.add_keyboard(XkbConfig::default(), 500, 25)?;

        let primary_selection_state = PrimarySelectionState::new::<State>(&display_handle);

        let wlr_data_control_state = wlr_data_control::DataControlState::new::<State, _>(
            &display_handle,
            Some(&primary_selection_state),
            filter_restricted_client,
        );
        let ext_data_control_state = ext_data_control::DataControlState::new::<State, _>(
            &display_handle,
            Some(&primary_selection_state),
            filter_restricted_client,
        );

        loop_handle
            .insert_source(Timer::immediate(), |_, _, state| {
                state
                    .pinnacle
                    .xdg_activation_state
                    .retain_tokens(|_, data| {
                        data.timestamp.elapsed() < XDG_ACTIVATION_TOKEN_TIMEOUT
                    });
                TimeoutAction::ToDuration(XDG_ACTIVATION_TOKEN_TIMEOUT)
            })
            .map_err(|err| {
                anyhow::anyhow!("failed to insert xdg activation token cleanup source: {err}")
            })?;

        let (blocker_cleared_tx, blocker_cleared_rx) = std::sync::mpsc::channel();

        let pinnacle = Pinnacle {
            loop_signal,
            loop_handle: loop_handle.clone(),
            display_handle: display_handle.clone(),
            clock: Clock::<Monotonic>::new(),
            socket_name,

            compositor_state: CompositorState::new::<State>(&display_handle),
            data_device_state: DataDeviceState::new::<State>(&display_handle),
            seat_state,
            shm_state: ShmState::new::<State>(&display_handle, vec![]),
            space: Space::<WindowElement>::default(),
            output_manager_state: OutputManagerState::new_with_xdg_output::<State>(&display_handle),
            xdg_shell_state: XdgShellState::new_with_capabilities::<State>(
                &display_handle,
                [WmCapabilities::Fullscreen, WmCapabilities::Maximize],
            ),
            viewporter_state: ViewporterState::new::<State>(&display_handle),
            fractional_scale_manager_state: FractionalScaleManagerState::new::<State>(
                &display_handle,
            ),
            primary_selection_state,
            layer_shell_state: WlrLayerShellState::new_with_filter::<State, _>(
                &display_handle,
                filter_restricted_client,
            ),
            wlr_data_control_state,
            ext_data_control_state,
            screencopy_manager_state: ScreencopyManagerState::new::<State, _>(
                &display_handle,
                filter_restricted_client,
            ),
            gamma_control_manager_state: GammaControlManagerState::new::<State, _>(
                &display_handle,
                filter_restricted_client,
            ),
            security_context_state: SecurityContextState::new::<State, _>(
                &display_handle,
                filter_restricted_client,
            ),
            relative_pointer_manager_state: RelativePointerManagerState::new::<State>(
                &display_handle,
            ),
            pointer_constraints_state: PointerConstraintsState::new::<State>(&display_handle),
            foreign_toplevel_manager_state: ForeignToplevelManagerState::new::<State, _>(
                &display_handle,
                filter_restricted_client,
            ),
            session_lock_manager_state: SessionLockManagerState::new::<State, _>(
                &display_handle,
                filter_restricted_client,
            ),
            xwayland_shell_state: XWaylandShellState::new::<State>(&display_handle),
            idle_notifier_state: IdleNotifierState::new(&display_handle, loop_handle),
            idle_inhibit_manager_state: IdleInhibitManagerState::new::<State>(&display_handle),
            output_management_manager_state: OutputManagementManagerState::new::<State, _>(
                &display_handle,
                filter_restricted_client,
            ),
            output_power_management_state: OutputPowerManagementState::new::<State, _>(
                &display_handle,
                filter_restricted_client,
            ),
            tablet_manager_state: TabletManagerState::new::<State>(&display_handle),
            keyboard_shortcuts_inhibit_state: KeyboardShortcutsInhibitState::new::<State>(
                &display_handle,
            ),
            xwayland_keyboard_grab_state: XWaylandKeyboardGrabState::new::<State>(&display_handle),
            xdg_activation_state: XdgActivationState::new::<State>(&display_handle),
            xdg_decoration_state: XdgDecorationState::new::<State>(&display_handle),
            kde_decoration_state: KdeDecorationState::new::<State>(
                &display_handle,
                org_kde_kwin_server_decoration_manager::Mode::Client,
            ),
            pointer_gestures_state: PointerGesturesState::new::<State>(&display_handle),
            single_pixel_buffer_state: SinglePixelBufferState::new::<State>(&display_handle),
            foreign_toplevel_list_state: ForeignToplevelListState::new::<State>(&display_handle),
            #[cfg(feature = "snowcap")]
            snowcap_decoration_state: SnowcapDecorationState::new::<State>(&display_handle),

            lock_state: LockState::default(),

            input_state: InputState::new(),

            output_focus_stack: OutputFocusStack::default(),
            z_index_stack: Vec::new(),

            config: Config::new(config_dir, cli),

            seat,

            dnd_icon: None,

            popup_manager: PopupManager::default(),

            windows: Vec::new(),
            unmapped_windows: Default::default(),
            keyboard_focus_stack: WindowKeyboardFocusStack::default(),
            on_demand_layer_focus: None,
            lock_surface_focus: None,

            xwayland_state: None,

            process_state: ProcessState::new(sysinfo::System::new_with_specifics(
                RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing()),
            )),

            grpc_server_join_handle: None,

            xdg_base_dirs: BaseDirectories::with_prefix("pinnacle"),

            signal_state: SignalState::default(),

            layout_state: LayoutState::default(),

            window_rule_state: WindowRuleState::default(),

            root_surface_cache: HashMap::new(),

            idle_inhibiting_surfaces: HashSet::new(),

            outputs: Default::default(),

            #[cfg(feature = "snowcap")]
            snowcap_handle: None,
            #[cfg(feature = "snowcap")]
            snowcap_join_handle: None,

            cursor_shape_manager_state: CursorShapeManagerState::new::<State>(&display_handle),
            cursor_state: CursorState::new(),

            pointer_contents: Default::default(),

            blocker_cleared_tx,
            blocker_cleared_rx,

            dmabuf_hooks: Default::default(),
        };

        Ok(pinnacle)
    }

    /// Schedule `run` to run when `condition` returns true.
    ///
    /// This will continually reschedule `run` in the event loop if `condition` returns false.
    pub fn schedule<F1, F2>(&self, condition: F1, run: F2)
    where
        F1: Fn(&mut State) -> bool + 'static,
        F2: FnOnce(&mut State) + 'static,
    {
        self.loop_handle.insert_idle(|state| {
            if !condition(state) {
                state.pinnacle.schedule(condition, run);
            } else {
                run(state);
            }
        });
    }

    pub fn shutdown(&mut self) {
        info!("Shutting down Pinnacle");
        self.loop_signal.stop();
        self.loop_signal.wakeup();
        if let Some(join_handle) = self.config.config_join_handle.take() {
            join_handle.abort();
        }
        if let Some(shutdown_sender) = self.config.keepalive_sender.take()
            && shutdown_sender.send(()).is_err()
        {
            warn!("failed to send shutdown signal to config");
        }

        #[cfg(feature = "snowcap")]
        if let Some(stop_signal) = self.snowcap_handle.take() {
            stop_signal.stop();
        }
    }

    pub fn send_frame_callbacks(&self, output: &Output, sequence: Option<FrameCallbackSequence>) {
        let _span = tracy_client::span!("Pinnacle::send_frame_callbacks");

        let should_send = |surface: &WlSurface, states: &SurfaceData| {
            // Do the standard primary scanout output check. For pointer surfaces it deduplicates
            // the frame callbacks across potentially multiple outputs, and for regular windows and
            // layer-shell surfaces it avoids sending frame callbacks to invisible surfaces.
            let current_primary_output = surface_primary_scanout_output(surface, states);

            if current_primary_output.as_ref() != Some(output) {
                return None;
            }

            let Some(sequence) = sequence else {
                return Some(output.clone());
            };

            // Next, check the throttling status.
            let frame_throttling_state = states
                .data_map
                .get_or_insert(SurfaceFrameThrottlingState::default);
            let mut last_sent_at = frame_throttling_state.last_sent_at.borrow_mut();

            let mut send = true;

            // If we already sent a frame callback to this surface this output refresh
            // cycle, don't send one again to prevent empty-damage commit busy loops.
            if let Some((last_output, last_sequence)) = &*last_sent_at
                && last_output == output
                && *last_sequence == sequence
            {
                send = false;
            }

            if send {
                *last_sent_at = Some((output.clone(), sequence));
                Some(output.clone())
            } else {
                None
            }
        };

        let now = self.clock.now();

        for window in self.space.elements_for_output(output) {
            window.send_frame(output, now, FRAME_CALLBACK_THROTTLE, should_send);

            #[cfg(feature = "snowcap")]
            window.with_state(|state| {
                for deco in state.decoration_surfaces.iter() {
                    deco.send_frame(output, now, FRAME_CALLBACK_THROTTLE, should_send);
                }
            });
        }

        for layer in layer_map_for_output(output).layers() {
            layer.send_frame(output, now, FRAME_CALLBACK_THROTTLE, should_send);
        }

        if let Some(lock_surface) = output.with_state(|state| state.lock_surface.clone()) {
            send_frames_surface_tree(
                lock_surface.wl_surface(),
                output,
                now,
                FRAME_CALLBACK_THROTTLE,
                should_send,
            );
        }

        if let Some(dnd) = self.dnd_icon.as_ref() {
            send_frames_surface_tree(dnd, output, now, FRAME_CALLBACK_THROTTLE, should_send);
        }

        if let CursorImageStatus::Surface(surface) = self.cursor_state.cursor_image() {
            send_frames_surface_tree(surface, output, now, FRAME_CALLBACK_THROTTLE, should_send);
        }
    }

    /// Returns a custom primary scanout output comparison function that picks the output with
    /// a larger visible area, as well as checks if the returned output actually
    /// exists. If it doesn't, it returns the new output.
    ///
    /// This is needed because when turning a monitor off and on, windows will *still* have the old
    /// output as the primary scanout output. For whatever reason, clones of that now-defunct
    /// output still exist somewhere, causing the default compare function to choose it over the
    /// new output for the monitor. This is a workaround for that.
    ///
    /// FIXME: audit every place that an output is stored and possibly convert it to a weakoutput
    fn primary_scanout_output_compare(
        &self,
    ) -> impl for<'a> Fn(
        &'a Output,
        &'a RenderElementState,
        &'a Output,
        &'a RenderElementState,
    ) -> &'a Output
    + '_ {
        |current_output, current_state, next_output, next_state| {
            let new_op = if next_state.visible_area > current_state.visible_area {
                next_output
            } else {
                current_output
            };

            if self.outputs.contains(new_op) {
                new_op
            } else {
                next_output
            }
        }
    }

    pub fn update_primary_scanout_output(
        &self,
        output: &Output,
        render_element_states: &RenderElementStates,
    ) {
        let _span = tracy_client::span!("Pinnacle::update_primary_scanout_output");

        for window in self.space.elements() {
            window.with_surfaces(|surface, states| {
                let primary_scanout_output = update_surface_primary_scanout_output(
                    surface,
                    output,
                    states,
                    render_element_states,
                    self.primary_scanout_output_compare(),
                );

                if let Some(output) = primary_scanout_output {
                    with_fractional_scale(states, |fraction_scale| {
                        fraction_scale
                            .set_preferred_scale(output.current_scale().fractional_scale());
                    });
                }
            });

            #[cfg(feature = "snowcap")]
            window.with_state(|state| {
                for deco in state.decoration_surfaces.iter() {
                    deco.with_surfaces(|surface, states| {
                        let primary_scanout_output = update_surface_primary_scanout_output(
                            surface,
                            output,
                            states,
                            render_element_states,
                            self.primary_scanout_output_compare(),
                        );

                        if let Some(output) = primary_scanout_output {
                            with_fractional_scale(states, |fraction_scale| {
                                fraction_scale
                                    .set_preferred_scale(output.current_scale().fractional_scale());
                            });
                        }
                    });
                }
            });
        }

        let map = layer_map_for_output(output);
        for layer_surface in map.layers() {
            layer_surface.with_surfaces(|surface, states| {
                let primary_scanout_output = update_surface_primary_scanout_output(
                    surface,
                    output,
                    states,
                    render_element_states,
                    self.primary_scanout_output_compare(),
                );

                if let Some(output) = primary_scanout_output {
                    with_fractional_scale(states, |fraction_scale| {
                        fraction_scale
                            .set_preferred_scale(output.current_scale().fractional_scale());
                    });
                }
            });
        }

        if let Some(lock_surface) = output.with_state(|state| state.lock_surface.clone()) {
            with_surface_tree_downward(
                lock_surface.wl_surface(),
                (),
                |_, _, _| compositor::TraversalAction::DoChildren(()),
                |surface, states, _| {
                    update_surface_primary_scanout_output(
                        surface,
                        output,
                        states,
                        render_element_states,
                        self.primary_scanout_output_compare(),
                    );
                },
                |_, _, _| true,
            );
        }

        if let Some(dnd) = self.dnd_icon.as_ref() {
            with_surface_tree_downward(
                dnd,
                (),
                |_, _, _| compositor::TraversalAction::DoChildren(()),
                |surface, states, _| {
                    update_surface_primary_scanout_output(
                        surface,
                        output,
                        states,
                        render_element_states,
                        self.primary_scanout_output_compare(),
                    );
                },
                |_, _, _| true,
            );
        }

        if let CursorImageStatus::Surface(surface) = self.cursor_state.cursor_image() {
            with_surface_tree_downward(
                surface,
                (),
                |_, _, _| compositor::TraversalAction::DoChildren(()),
                |surface, states, _| {
                    update_surface_primary_scanout_output(
                        surface,
                        output,
                        states,
                        render_element_states,
                        self.primary_scanout_output_compare(),
                    );
                },
                |_, _, _| true,
            );
        }
    }

    pub fn send_dmabuf_feedback(
        &self,
        output: &Output,
        feedback: &SurfaceDmabufFeedback,
        render_element_states: &RenderElementStates,
    ) {
        let _span = tracy_client::span!("Pinnacle::send_dmabuf_feedback");

        for window in self.space.elements() {
            if self.space.outputs_for_element(window).contains(output) {
                window.send_dmabuf_feedback(
                    output,
                    surface_primary_scanout_output,
                    |surface, _| {
                        select_dmabuf_feedback(
                            surface,
                            render_element_states,
                            &feedback.render_feedback,
                            &feedback.scanout_feedback,
                        )
                    },
                );

                // FIXME: get the actual overlap
                #[cfg(feature = "snowcap")]
                window.with_state(|state| {
                    for deco in state.decoration_surfaces.iter() {
                        deco.send_dmabuf_feedback(
                            output,
                            surface_primary_scanout_output,
                            |surface, _| {
                                select_dmabuf_feedback(
                                    surface,
                                    render_element_states,
                                    &feedback.render_feedback,
                                    &feedback.scanout_feedback,
                                )
                            },
                        );
                    }
                });
            }
        }

        let map = layer_map_for_output(output);
        for layer_surface in map.layers() {
            layer_surface.send_dmabuf_feedback(
                output,
                surface_primary_scanout_output,
                |surface, _| {
                    select_dmabuf_feedback(
                        surface,
                        render_element_states,
                        &feedback.render_feedback,
                        &feedback.scanout_feedback,
                    )
                },
            );
        }

        if let Some(lock_surface) = output.with_state(|state| state.lock_surface.clone()) {
            send_dmabuf_feedback_surface_tree(
                lock_surface.wl_surface(),
                output,
                surface_primary_scanout_output,
                |surface, _| {
                    select_dmabuf_feedback(
                        surface,
                        render_element_states,
                        &feedback.render_feedback,
                        &feedback.scanout_feedback,
                    )
                },
            );
        }

        if let Some(dnd) = self.dnd_icon.as_ref() {
            send_dmabuf_feedback_surface_tree(
                dnd,
                output,
                surface_primary_scanout_output,
                |surface, _| {
                    select_dmabuf_feedback(
                        surface,
                        render_element_states,
                        &feedback.render_feedback,
                        &feedback.scanout_feedback,
                    )
                },
            );
        }

        if let CursorImageStatus::Surface(surface) = self.cursor_state.cursor_image() {
            send_dmabuf_feedback_surface_tree(
                surface,
                output,
                surface_primary_scanout_output,
                |surface, _| {
                    select_dmabuf_feedback(
                        surface,
                        render_element_states,
                        &feedback.render_feedback,
                        &feedback.scanout_feedback,
                    )
                },
            );
        }
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct FrameCallbackSequence(u32);

impl FrameCallbackSequence {
    pub fn increment(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

#[derive(Default)]
struct SurfaceFrameThrottlingState {
    last_sent_at: RefCell<Option<(Output, FrameCallbackSequence)>>,
}

impl State {
    pub fn new(
        backend: cli::Backend,
        loop_handle: LoopHandle<'static, State>,
        loop_signal: LoopSignal,
        config_dir: PathBuf,
        cli: Option<Cli>,
        create_socket: bool,
    ) -> anyhow::Result<Self> {
        let _span = tracy_client::span!("State::new");

        let display = Display::<State>::new()?;

        let (backend, pinnacle) = match backend {
            cli::Backend::Winit => {
                info!("Starting winit backend");
                let uninit_winit = Winit::try_new(display.handle())?;
                let mut pinnacle = Pinnacle::new(
                    display,
                    loop_signal,
                    loop_handle,
                    uninit_winit.seat_name,
                    config_dir,
                    cli,
                    create_socket,
                )?;
                let winit = (uninit_winit.init)(&mut pinnacle)?;
                (backend::Backend::Winit(winit), pinnacle)
            }
            cli::Backend::Udev => {
                info!("Starting udev backend");
                let uninit_udev = Udev::try_new(display.handle())?;
                let mut pinnacle = Pinnacle::new(
                    display,
                    loop_signal,
                    loop_handle,
                    uninit_udev.seat_name,
                    config_dir,
                    cli,
                    create_socket,
                )?;
                let udev = (uninit_udev.init)(&mut pinnacle)?;
                (backend::Backend::Udev(udev), pinnacle)
            }
            #[cfg(feature = "testing")]
            cli::Backend::Dummy => {
                let uninit_dummy = Dummy::try_new();
                let mut pinnacle = Pinnacle::new(
                    display,
                    loop_signal,
                    loop_handle,
                    uninit_dummy.seat_name,
                    config_dir,
                    cli,
                    create_socket,
                )?;
                let dummy = (uninit_dummy.init)(&mut pinnacle)?;
                (backend::Backend::Dummy(dummy), pinnacle)
            }
        };

        Ok(Self { backend, pinnacle })
    }
}

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
    /// True, if the client may NOT access restricted protocols
    pub is_restricted: bool,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}

    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

/// A trait meant to be used in types with a [`UserDataMap`][smithay::utils::user_data::UserDataMap]
/// to get user-defined state.
pub trait WithState {
    /// The user-defined state
    type State;

    /// Access data map state.
    ///
    /// RefCell Safety: This function will panic if called within [`with_state_mut`][Self::with_state_mut].
    fn with_state<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&Self::State) -> T;

    /// Access data map state mutably.
    ///
    /// RefCell Safety: This function will panic if called within itself or
    /// [`with_state`][Self::with_state].
    fn with_state_mut<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&mut Self::State) -> T;
}
