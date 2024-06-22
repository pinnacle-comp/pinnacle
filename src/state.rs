// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    api::signal::SignalState,
    backend::{self, udev::Udev, winit::Winit, Backend},
    cli::{self, Cli},
    config::Config,
    cursor::CursorState,
    focus::OutputFocusStack,
    grab::resize_grab::ResizeSurfaceState,
    handlers::session_lock::LockState,
    layout::LayoutState,
    protocol::{
        foreign_toplevel::{self, ForeignToplevelManagerState},
        gamma_control::GammaControlManagerState,
        output_management::OutputManagementManagerState,
        output_power_management::OutputPowerManagementState,
        screencopy::ScreencopyManagerState,
    },
    window::WindowElement,
};
use anyhow::Context;
use indexmap::IndexMap;
use pinnacle_api_defs::pinnacle::v0alpha1::ShutdownWatchResponse;
use smithay::{
    desktop::{PopupManager, Space},
    input::{keyboard::XkbConfig, Seat, SeatState},
    output::Output,
    reexports::{
        calloop::{generic::Generic, Interest, LoopHandle, LoopSignal, Mode, PostAction},
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason, GlobalId},
            protocol::wl_surface::WlSurface,
            Client, Display, DisplayHandle,
        },
    },
    utils::{Clock, Monotonic},
    wayland::{
        compositor::{self, CompositorClientState, CompositorState},
        cursor_shape::CursorShapeManagerState,
        dmabuf::DmabufFeedback,
        fractional_scale::FractionalScaleManagerState,
        idle_notify::IdleNotifierState,
        keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitState,
        output::OutputManagerState,
        pointer_constraints::PointerConstraintsState,
        relative_pointer::RelativePointerManagerState,
        security_context::SecurityContextState,
        selection::{
            data_device::DataDeviceState, primary_selection::PrimarySelectionState,
            wlr_data_control::DataControlState,
        },
        session_lock::SessionLockManagerState,
        shell::{wlr_layer::WlrLayerShellState, xdg::XdgShellState},
        shm::ShmState,
        socket::ListeningSocketSource,
        tablet_manager::TabletManagerState,
        viewporter::ViewporterState,
        xwayland_keyboard_grab::XWaylandKeyboardGrabState,
        xwayland_shell::XWaylandShellState,
    },
    xwayland::{X11Wm, XWaylandClientData},
};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};
use sysinfo::{ProcessRefreshKind, RefreshKind};
use tracing::{info, warn};
use xdg::BaseDirectories;

use crate::input::InputState;

#[cfg(feature = "testing")]
use crate::backend::dummy::Dummy;

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
    pub data_control_state: DataControlState,
    pub screencopy_manager_state: ScreencopyManagerState,
    pub gamma_control_manager_state: GammaControlManagerState,
    pub security_context_state: SecurityContextState,
    pub relative_pointer_manager_state: RelativePointerManagerState,
    pub pointer_constraints_state: PointerConstraintsState,
    pub foreign_toplevel_manager_state: ForeignToplevelManagerState,
    pub session_lock_manager_state: SessionLockManagerState,
    pub xwayland_shell_state: XWaylandShellState,
    pub idle_notifier_state: IdleNotifierState<State>,
    pub output_management_manager_state: OutputManagementManagerState,
    pub output_power_management_state: OutputPowerManagementState,
    pub tablet_manager_state: TabletManagerState,
    pub keyboard_shortcuts_inhibit_state: KeyboardShortcutsInhibitState,
    pub xwayland_keyboard_grab_state: XWaylandKeyboardGrabState,

    pub lock_state: LockState,

    /// The state of key and mousebinds along with libinput settings
    pub input_state: InputState,

    pub output_focus_stack: OutputFocusStack,
    pub z_index_stack: Vec<WindowElement>,

    pub popup_manager: PopupManager,

    pub dnd_icon: Option<WlSurface>,

    /// The main window vec
    pub windows: Vec<WindowElement>,
    /// Windows with no buffer.
    pub unmapped_windows: Vec<WindowElement>,

    pub config: Config,

    // xwayland stuff
    pub xwm: Option<X11Wm>,
    pub xdisplay: Option<u32>,

    pub system_processes: sysinfo::System,

    // Currently only used to keep track of if the server has started
    pub grpc_server_join_handle: Option<tokio::task::JoinHandle<()>>,

    pub xdg_base_dirs: BaseDirectories,

    pub signal_state: SignalState,

    pub layout_state: LayoutState,

    /// A cache of surfaces to their root surface.
    pub root_surface_cache: HashMap<WlSurface, WlSurface>,

    /// WlSurfaces with an attached idle inhibitor.
    pub idle_inhibiting_surfaces: HashSet<WlSurface>,

    pub outputs: IndexMap<Output, Option<GlobalId>>,

    #[cfg(feature = "snowcap")]
    pub snowcap_stop_signal: Option<snowcap::StopSignal>,
    #[cfg(feature = "snowcap")]
    pub snowcap_join_handle: Option<tokio::task::JoinHandle<()>>,

    pub cursor_shape_manager_state: CursorShapeManagerState,
    pub cursor_state: CursorState,
}

impl State {
    pub fn on_event_loop_cycle_completion(&mut self) {
        self.pinnacle.fixup_z_layering();
        self.pinnacle.space.refresh();
        self.pinnacle.popup_manager.cleanup();
        self.update_pointer_focus();
        foreign_toplevel::refresh(self);
        self.pinnacle.refresh_idle_inhibit();

        if let Backend::Winit(winit) = &mut self.backend {
            winit.render_if_scheduled(&mut self.pinnacle);
        }

        #[cfg(feature = "snowcap")]
        if self
            .pinnacle
            .snowcap_join_handle
            .as_ref()
            .is_some_and(|handle| handle.is_finished())
        {
            // If Snowcap is dead, the config has most likely crashed or will crash if it's used.
            // The embedded config will also fail to start.
            // We'll panic here just so people aren't stuck in the compositor.
            panic!("snowcap has exited");
        }

        // FIXME: Don't poll this every cycle
        for output in self.pinnacle.space.outputs().cloned().collect::<Vec<_>>() {
            output.with_state_mut(|state| {
                if state
                    .layout_transaction
                    .as_ref()
                    .is_some_and(|ts| ts.ready())
                {
                    self.schedule_render(&output);
                }
            });
        }

        self.pinnacle
            .display_handle
            .flush_clients()
            .expect("failed to flush client buffers");
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
    ) -> anyhow::Result<Self> {
        let socket = ListeningSocketSource::new_auto()?;
        let socket_name = socket.socket_name().to_os_string();

        info!(
            "Setting WAYLAND_DISPLAY to {}",
            socket_name.to_string_lossy()
        );
        std::env::set_var("WAYLAND_DISPLAY", socket_name);

        loop_handle.insert_source(socket, |stream, _metadata, state| {
            state
                .pinnacle
                .display_handle
                .insert_client(stream, Arc::new(ClientState::default()))
                .expect("Could not insert client into loop handle");
        })?;

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

        let data_control_state = DataControlState::new::<State, _>(
            &display_handle,
            Some(&primary_selection_state),
            filter_restricted_client,
        );

        let pinnacle = Pinnacle {
            loop_signal,
            loop_handle: loop_handle.clone(),
            display_handle: display_handle.clone(),
            clock: Clock::<Monotonic>::new(),
            compositor_state: CompositorState::new::<State>(&display_handle),
            data_device_state: DataDeviceState::new::<State>(&display_handle),
            seat_state,
            shm_state: ShmState::new::<State>(&display_handle, vec![]),
            space: Space::<WindowElement>::default(),
            output_manager_state: OutputManagerState::new_with_xdg_output::<State>(&display_handle),
            xdg_shell_state: XdgShellState::new::<State>(&display_handle),
            viewporter_state: ViewporterState::new::<State>(&display_handle),
            fractional_scale_manager_state: FractionalScaleManagerState::new::<State>(
                &display_handle,
            ),
            primary_selection_state,
            layer_shell_state: WlrLayerShellState::new_with_filter::<State, _>(
                &display_handle,
                filter_restricted_client,
            ),
            data_control_state,
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

            lock_state: LockState::default(),

            input_state: InputState::new(),

            output_focus_stack: OutputFocusStack::default(),
            z_index_stack: Vec::new(),

            config: Config::new(config_dir, cli),

            seat,

            dnd_icon: None,

            popup_manager: PopupManager::default(),

            windows: Vec::new(),
            unmapped_windows: Vec::new(),

            xwm: None,
            xdisplay: None,

            system_processes: sysinfo::System::new_with_specifics(
                RefreshKind::new().with_processes(ProcessRefreshKind::new()),
            ),

            grpc_server_join_handle: None,

            xdg_base_dirs: BaseDirectories::with_prefix("pinnacle")
                .context("couldn't create xdg BaseDirectories")?,

            signal_state: SignalState::default(),

            layout_state: LayoutState::default(),

            root_surface_cache: HashMap::new(),

            idle_inhibiting_surfaces: HashSet::new(),

            outputs: IndexMap::new(),

            #[cfg(feature = "snowcap")]
            snowcap_stop_signal: None,
            #[cfg(feature = "snowcap")]
            snowcap_join_handle: None,

            cursor_shape_manager_state: CursorShapeManagerState::new::<State>(&display_handle),
            cursor_state: CursorState::new(),
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
        if let Some(shutdown_sender) = self.config.shutdown_sender.take() {
            if let Err(err) = shutdown_sender.send(Ok(ShutdownWatchResponse {})) {
                warn!("Failed to send shutdown signal to config: {err}");
            }
        }

        #[cfg(feature = "snowcap")]
        if let Some(stop_signal) = self.snowcap_stop_signal.take() {
            stop_signal.stop();
        }
    }
}

impl State {
    pub fn new(
        backend: cli::Backend,
        loop_handle: LoopHandle<'static, State>,
        loop_signal: LoopSignal,
        config_dir: PathBuf,
        cli: Option<Cli>,
    ) -> anyhow::Result<Self> {
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
                )?;
                let udev = (uninit_udev.init)(&mut pinnacle)?;
                (backend::Backend::Udev(udev), pinnacle)
            }
            #[cfg(feature = "testing")]
            cli::Backend::Dummy => {
                let uninit_dummy = Dummy::try_new(display.handle());
                let mut pinnacle = Pinnacle::new(
                    display,
                    loop_signal,
                    loop_handle,
                    uninit_dummy.seat_name,
                    config_dir,
                    cli,
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

#[derive(Debug, Copy, Clone)]
pub struct SurfaceDmabufFeedback<'a> {
    pub render_feedback: &'a DmabufFeedback,
    pub scanout_feedback: &'a DmabufFeedback,
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

#[derive(Default, Debug)]
pub struct WlSurfaceState {
    pub resize_state: ResizeSurfaceState,
}

impl WithState for WlSurface {
    type State = WlSurfaceState;

    fn with_state<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&Self::State) -> T,
    {
        compositor::with_states(self, |states| {
            let state = states
                .data_map
                .get_or_insert(RefCell::<Self::State>::default);

            func(&state.borrow())
        })
    }

    fn with_state_mut<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&mut Self::State) -> T,
    {
        compositor::with_states(self, |states| {
            let state = states
                .data_map
                .get_or_insert(RefCell::<Self::State>::default);

            func(&mut state.borrow_mut())
        })
    }
}
