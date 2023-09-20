// SPDX-License-Identifier: GPL-3.0-or-later

mod api_handlers;

use std::{
    cell::RefCell,
    os::{fd::AsRawFd, unix::net::UnixStream},
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    backend::{udev::Udev, winit::Winit, BackendData},
    config::{
        api::msg::{
            window_rules::{WindowRule, WindowRuleCondition},
            CallbackId, Msg,
        },
        ConfigReturn,
    },
    cursor::Cursor,
    focus::FocusState,
    grab::resize_grab::ResizeSurfaceState,
    window::WindowElement,
};
use calloop::{channel::Sender, futures::Scheduler, RegistrationToken};
use smithay::{
    desktop::{PopupManager, Space},
    input::{keyboard::XkbConfig, pointer::CursorImageStatus, Seat, SeatState},
    reexports::{
        calloop::{
            self, channel::Event, generic::Generic, Interest, LoopHandle, LoopSignal, Mode,
            PostAction,
        },
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::wl_surface::WlSurface,
            Display, DisplayHandle,
        },
    },
    utils::{Clock, Logical, Monotonic, Point, Size},
    wayland::{
        compositor::{self, CompositorClientState, CompositorState},
        data_device::DataDeviceState,
        dmabuf::DmabufFeedback,
        fractional_scale::FractionalScaleManagerState,
        output::OutputManagerState,
        primary_selection::PrimarySelectionState,
        shell::{wlr_layer::WlrLayerShellState, xdg::XdgShellState},
        shm::ShmState,
        socket::ListeningSocketSource,
        viewporter::ViewporterState,
    },
    xwayland::{X11Surface, X11Wm, XWayland, XWaylandEvent},
};

use crate::input::InputState;

pub enum Backend {
    Winit(Winit),
    Udev(Udev),
}

impl Backend {
    pub fn seat_name(&self) -> String {
        match self {
            Backend::Winit(winit) => winit.seat_name(),
            Backend::Udev(udev) => udev.seat_name(),
        }
    }

    pub fn early_import(&mut self, surface: &WlSurface) {
        match self {
            Backend::Winit(winit) => winit.early_import(surface),
            Backend::Udev(udev) => udev.early_import(surface),
        }
    }

    /// Returns `true` if the backend is [`Winit`].
    ///
    /// [`Winit`]: Backend::Winit
    #[must_use]
    pub fn is_winit(&self) -> bool {
        matches!(self, Self::Winit(..))
    }
}

/// The main state of the application.
pub struct State {
    pub backend: Backend,

    pub loop_signal: LoopSignal,
    pub loop_handle: LoopHandle<'static, CalloopData>,
    pub display_handle: DisplayHandle,
    pub clock: Clock<Monotonic>,

    pub space: Space<WindowElement>,
    pub move_mode: bool,
    pub socket_name: String,

    pub seat: Seat<State>,

    pub compositor_state: CompositorState,
    pub data_device_state: DataDeviceState,
    pub seat_state: SeatState<Self>,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub xdg_shell_state: XdgShellState,
    pub viewporter_state: ViewporterState,
    pub fractional_scale_manager_state: FractionalScaleManagerState,
    pub primary_selection_state: PrimarySelectionState,
    pub layer_shell_state: WlrLayerShellState,

    pub input_state: InputState,
    pub api_state: ApiState,
    pub focus_state: FocusState,

    pub popup_manager: PopupManager,

    pub cursor_status: CursorImageStatus,
    pub pointer_location: Point<f64, Logical>,
    pub dnd_icon: Option<WlSurface>,

    pub windows: Vec<WindowElement>,
    pub window_rules: Vec<(WindowRuleCondition, WindowRule)>,

    pub async_scheduler: Scheduler<()>,

    // TODO: move into own struct
    // |     basically just clean this mess up
    pub output_callback_ids: Vec<CallbackId>,

    pub xwayland: XWayland,
    pub xwm: Option<X11Wm>,
    pub xdisplay: Option<u32>,
    pub override_redirect_windows: Vec<X11Surface>,
}

impl State {
    pub fn init(
        backend: Backend,
        display: &mut Display<Self>,
        loop_signal: LoopSignal,
        loop_handle: LoopHandle<'static, CalloopData>,
    ) -> anyhow::Result<Self> {
        let socket = ListeningSocketSource::new_auto()?;
        let socket_name = socket.socket_name().to_os_string();

        std::env::set_var("WAYLAND_DISPLAY", socket_name.clone());
        tracing::info!(
            "Set WAYLAND_DISPLAY to {}",
            socket_name.clone().to_string_lossy()
        );

        // Opening a new process will use up a few file descriptors, around 10 for Alacritty, for
        // example. Because of this, opening up only around 100 processes would exhaust the file
        // descriptor limit on my system (Arch btw) and cause a "Too many open files" crash.
        //
        // To fix this, I just set the limit to be higher. As Pinnacle is the whole graphical
        // environment, I *think* this is ok.
        tracing::info!("Trying to raise file descriptor limit...");
        if let Err(err) = smithay::reexports::nix::sys::resource::setrlimit(
            smithay::reexports::nix::sys::resource::Resource::RLIMIT_NOFILE,
            65536,
            65536 * 2,
        ) {
            tracing::error!("Could not raise fd limit: errno {err}");
        } else {
            tracing::info!("Fd raise success!");
        }

        loop_handle.insert_source(socket, |stream, _metadata, data| {
            data.display
                .handle()
                .insert_client(stream, Arc::new(ClientState::default()))
                .expect("Could not insert client into loop handle");
        })?;

        loop_handle.insert_source(
            Generic::new(
                display.backend().poll_fd().as_raw_fd(),
                Interest::READ,
                Mode::Level,
            ),
            |_readiness, _metadata, data| {
                data.display.dispatch_clients(&mut data.state)?;
                Ok(PostAction::Continue)
            },
        )?;

        let (tx_channel, rx_channel) = calloop::channel::channel::<Msg>();

        let ConfigReturn {
            reload_keybind,
            kill_keybind,
            config_child_handle,
            socket_source,
        } = crate::config::start_config(tx_channel.clone())?;

        let socket_token_ret = loop_handle.insert_source(socket_source, |stream, _, data| {
            if let Some(old_stream) = data
                .state
                .api_state
                .stream
                .replace(Arc::new(Mutex::new(stream)))
            {
                old_stream
                    .lock()
                    .expect("Couldn't lock old stream")
                    .shutdown(std::net::Shutdown::Both)
                    .expect("Couldn't shutdown old stream");
            }
        });

        let socket_token = match socket_token_ret {
            Ok(token) => token,
            Err(err) => {
                anyhow::bail!("Failed to insert socket source into event loop: {err}");
            }
        };

        let (executor, sched) =
            calloop::futures::executor::<()>().expect("Couldn't create executor");
        if let Err(err) = loop_handle.insert_source(executor, |_, _, _| {}) {
            anyhow::bail!("Failed to insert async executor into event loop: {err}");
        }

        let display_handle = display.handle();
        let mut seat_state = SeatState::new();

        let mut seat = seat_state.new_wl_seat(&display_handle, backend.seat_name());
        seat.add_pointer();
        seat.add_keyboard(XkbConfig::default(), 500, 25)?;

        loop_handle.insert_idle(|data| {
            data.state
                .loop_handle
                .insert_source(rx_channel, |msg, _, data| match msg {
                    Event::Msg(msg) => data.state.handle_msg(msg),
                    Event::Closed => todo!(),
                })
                .expect("failed to insert rx_channel into loop");
        });

        let xwayland = {
            let (xwayland, channel) = XWayland::new(&display_handle);
            let clone = display_handle.clone();
            tracing::debug!("inserting into loop");
            let res = loop_handle.insert_source(channel, move |event, _, data| match event {
                XWaylandEvent::Ready {
                    connection,
                    client,
                    client_fd: _,
                    display,
                } => {
                    let mut wm = X11Wm::start_wm(
                        data.state.loop_handle.clone(),
                        clone.clone(),
                        connection,
                        client,
                    )
                    .expect("failed to attach x11wm");

                    let cursor = Cursor::load();
                    let image = cursor.get_image(1, Duration::ZERO);
                    wm.set_cursor(
                        &image.pixels_rgba,
                        Size::from((image.width as u16, image.height as u16)),
                        Point::from((image.xhot as u16, image.yhot as u16)),
                    )
                    .expect("failed to set xwayland default cursor");

                    tracing::debug!("setting xwm and xdisplay");

                    data.state.xwm = Some(wm);
                    data.state.xdisplay = Some(display);
                }
                XWaylandEvent::Exited => {
                    data.state.xwm.take();
                }
            });
            if let Err(err) = res {
                tracing::error!("Failed to insert XWayland source into loop: {err}");
            }
            xwayland
        };
        tracing::debug!("xwayland set up");

        Ok(Self {
            backend,
            loop_signal,
            loop_handle,
            display_handle: display_handle.clone(),
            clock: Clock::<Monotonic>::new()?,
            compositor_state: CompositorState::new::<Self>(&display_handle),
            data_device_state: DataDeviceState::new::<Self>(&display_handle),
            seat_state,
            pointer_location: (0.0, 0.0).into(),
            shm_state: ShmState::new::<Self>(&display_handle, vec![]),
            space: Space::<WindowElement>::default(),
            cursor_status: CursorImageStatus::Default,
            output_manager_state: OutputManagerState::new_with_xdg_output::<Self>(&display_handle),
            xdg_shell_state: XdgShellState::new::<Self>(&display_handle),
            viewporter_state: ViewporterState::new::<Self>(&display_handle),
            fractional_scale_manager_state: FractionalScaleManagerState::new::<Self>(
                &display_handle,
            ),
            primary_selection_state: PrimarySelectionState::new::<Self>(&display_handle),
            layer_shell_state: WlrLayerShellState::new::<Self>(&display_handle),

            input_state: InputState::new(reload_keybind, kill_keybind),
            api_state: ApiState {
                stream: None,
                socket_token,
                tx_channel,
                config_process: config_child_handle,
            },
            focus_state: FocusState::new(),

            seat,

            dnd_icon: None,

            move_mode: false,
            socket_name: socket_name.to_string_lossy().to_string(),

            popup_manager: PopupManager::default(),

            async_scheduler: sched,

            windows: vec![],
            window_rules: vec![],
            output_callback_ids: vec![],

            xwayland,
            xwm: None,
            xdisplay: None,
            override_redirect_windows: vec![],
        })
    }

    /// Schedule `run` to run when `condition` returns true.
    ///
    /// This will continually reschedule `run` in the event loop if `condition` returns false.
    pub fn schedule<F1, F2>(&self, condition: F1, run: F2)
    where
        F1: Fn(&mut CalloopData) -> bool + 'static,
        F2: FnOnce(&mut CalloopData) + 'static,
    {
        self.loop_handle.insert_idle(|data| {
            Self::schedule_inner(data, condition, run);
        });
    }

    /// Schedule something to be done when `condition` returns true.
    fn schedule_inner<F1, F2>(data: &mut CalloopData, condition: F1, run: F2)
    where
        F1: Fn(&mut CalloopData) -> bool + 'static,
        F2: FnOnce(&mut CalloopData) + 'static,
    {
        if !condition(data) {
            data.state.loop_handle.insert_idle(|data| {
                Self::schedule_inner(data, condition, run);
            });
            return;
        }

        run(data);
    }
}

pub struct CalloopData {
    pub display: Display<State>,
    pub state: State,
}

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
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

pub struct ApiState {
    // TODO: this may not need to be in an arc mutex because of the move to async
    /// The stream API messages are being sent through
    pub stream: Option<Arc<Mutex<UnixStream>>>,
    /// A token used to remove the socket source from the event loop on config restart
    pub socket_token: RegistrationToken,
    /// The sending channel used to send API messages from the socket source to a handler
    pub tx_channel: Sender<Msg>,
    pub config_process: async_process::Child,
}

pub trait WithState {
    type State;

    /// Access data map state.
    ///
    /// RefCell Safety: This function will panic if called within itself.
    fn with_state<F, T>(&self, func: F) -> T
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
