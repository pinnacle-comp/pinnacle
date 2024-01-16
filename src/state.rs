// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    api::{msg::Msg, ApiState},
    backend::Backend,
    config::Config,
    cursor::Cursor,
    focus::FocusState,
    grab::resize_grab::ResizeSurfaceState,
    window::WindowElement,
};
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
        dmabuf::DmabufFeedback,
        fractional_scale::FractionalScaleManagerState,
        output::OutputManagerState,
        selection::data_device::DataDeviceState,
        selection::primary_selection::PrimarySelectionState,
        shell::{wlr_layer::WlrLayerShellState, xdg::XdgShellState},
        shm::ShmState,
        socket::ListeningSocketSource,
        viewporter::ViewporterState,
    },
    xwayland::{X11Wm, XWayland, XWaylandEvent},
};
use std::{cell::RefCell, sync::Arc, time::Duration};
use sysinfo::{ProcessRefreshKind, RefreshKind};
use tokio::sync::mpsc::UnboundedSender;

use crate::input::InputState;

/// The main state of the application.
pub struct State {
    /// Which backend is currently running
    pub backend: Backend,

    /// A loop signal used to stop the compositor
    pub loop_signal: LoopSignal,
    /// A handle to the event loop
    pub loop_handle: LoopHandle<'static, CalloopData>,
    pub display_handle: DisplayHandle,
    pub clock: Clock<Monotonic>,

    pub space: Space<WindowElement>,
    /// The name of the Wayland socket
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

    /// The state of key and mousebinds along with libinput settings
    pub input_state: InputState,
    /// The state holding stuff dealing with the api, like the stream
    pub api_state: ApiState,
    /// Keeps track of the focus stack and focused output
    pub focus_state: FocusState,

    pub popup_manager: PopupManager,

    pub cursor_status: CursorImageStatus,
    pub pointer_location: Point<f64, Logical>,
    pub dnd_icon: Option<WlSurface>,

    /// The main window vec
    pub windows: Vec<WindowElement>,

    pub config: Config,

    // xwayland stuff
    pub xwayland: XWayland,
    pub xwm: Option<X11Wm>,
    pub xdisplay: Option<u32>,

    pub system_processes: sysinfo::System,

    pub config_join_handle: Option<tokio::task::JoinHandle<()>>,
    pub grpc_kill_pinger: Option<UnboundedSender<()>>,
    pub grpc_server_join_handle:
        Option<tokio::task::JoinHandle<Result<(), tonic::transport::Error>>>,
}

impl State {
    /// Creates the central state and starts the config and xwayland
    pub fn init(
        backend: Backend,
        display: Display<Self>,
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
        if let Err(err) = nix::sys::resource::setrlimit(
            nix::sys::resource::Resource::RLIMIT_NOFILE,
            65536,
            65536 * 2,
        ) {
            tracing::error!("Could not raise fd limit: errno {err}");
        } else {
            tracing::info!("Fd raise success!");
        }

        loop_handle.insert_source(socket, |stream, _metadata, data| {
            data.display_handle
                .insert_client(stream, Arc::new(ClientState::default()))
                .expect("Could not insert client into loop handle");
        })?;

        let display_handle = display.handle();

        loop_handle.insert_source(
            Generic::new(display, Interest::READ, Mode::Level),
            |_readiness, display, data| {
                // Safety: we don't drop the display
                unsafe {
                    display
                        .get_mut()
                        .dispatch_clients(&mut data.state)
                        .expect("failed to dispatch clients");
                }
                Ok(PostAction::Continue)
            },
        )?;

        let (tx_channel, rx_channel) = calloop::channel::channel::<Msg>();

        loop_handle.insert_idle(|data| {
            if let Err(err) = data.state.start_config(crate::config::get_config_dir()) {
                panic!("failed to start config: {err}");
            }
        });

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

                    std::env::set_var("DISPLAY", format!(":{display}"));
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

        // gRPC stuff

        // std::env::set_var(
        //     "PINNACLE_PROTO_DIR",
        //     XDG_BASE_DIRS.get_data_file("protobuf"),
        // );
        //
        // let (grpc_sender, grpc_receiver) =
        //     calloop::channel::channel::<Box<dyn FnOnce(&mut Self) + Send>>();
        //
        // loop_handle.insert_idle(|data| {
        //     data.state
        //         .loop_handle
        //         .insert_source(grpc_receiver, |msg, _, data| match msg {
        //             Event::Msg(f) => f(&mut data.state),
        //             Event::Closed => panic!("grpc receiver was closed"),
        //         })
        //         .expect("failed to insert grpc_receiver into loop");
        // });
        //
        // let pinnacle_service = PinnacleService {
        //     sender: grpc_sender.clone(),
        // };
        // let input_service = InputService {
        //     sender: grpc_sender.clone(),
        // };
        // let process_service = ProcessService {
        //     sender: grpc_sender.clone(),
        // };
        // let tag_service = TagService {
        //     sender: grpc_sender.clone(),
        // };
        // let output_service = OutputService {
        //     sender: grpc_sender.clone(),
        // };
        // let window_service = WindowService {
        //     sender: grpc_sender.clone(),
        // };
        //
        // let refl_service = tonic_reflection::server::Builder::configure()
        //     .register_encoded_file_descriptor_set(pinnacle_api_defs::FILE_DESCRIPTOR_SET)
        //     .build()?;
        //
        // let p = std::path::Path::new("/tmp/pinnacle/grpc.sock");
        // let _ = std::fs::remove_file(p);
        // std::fs::create_dir_all(p.parent().unwrap())?;
        // let uds = tokio::net::UnixListener::bind(p)?;
        // let uds_stream = tokio_stream::wrappers::UnixListenerStream::new(uds);
        //
        // let grpc_server = tonic::transport::Server::builder()
        //     .add_service(refl_service)
        //     .add_service(PinnacleServiceServer::new(pinnacle_service))
        //     .add_service(InputServiceServer::new(input_service))
        //     .add_service(ProcessServiceServer::new(process_service))
        //     .add_service(TagServiceServer::new(tag_service))
        //     .add_service(OutputServiceServer::new(output_service))
        //     .add_service(WindowServiceServer::new(window_service));
        //
        // let (ping_tx, mut ping_rx) = tokio::sync::mpsc::unbounded_channel::<()>();
        //
        // let ping_rx_future = async move {
        //     ping_rx.recv().await;
        // };
        //
        // let ping_tx_clone = ping_tx.clone();
        // let loop_signal_clone = loop_signal.clone();
        // tokio::spawn(async move {
        //     tokio::signal::ctrl_c()
        //         .await
        //         .expect("failed to listen for ctrl-c");
        //     ping_tx_clone
        //         .send(())
        //         .expect("failed to send grpc shutdown ping");
        //     loop_signal_clone.stop();
        // });

        let state = Self {
            backend,
            loop_signal,
            loop_handle,
            display_handle: display_handle.clone(),
            clock: Clock::<Monotonic>::new(),
            compositor_state: CompositorState::new::<Self>(&display_handle),
            data_device_state: DataDeviceState::new::<Self>(&display_handle),
            seat_state,
            pointer_location: (0.0, 0.0).into(),
            shm_state: ShmState::new::<Self>(&display_handle, vec![]),
            space: Space::<WindowElement>::default(),
            cursor_status: CursorImageStatus::default_named(),
            output_manager_state: OutputManagerState::new_with_xdg_output::<Self>(&display_handle),
            xdg_shell_state: XdgShellState::new::<Self>(&display_handle),
            viewporter_state: ViewporterState::new::<Self>(&display_handle),
            fractional_scale_manager_state: FractionalScaleManagerState::new::<Self>(
                &display_handle,
            ),
            primary_selection_state: PrimarySelectionState::new::<Self>(&display_handle),
            layer_shell_state: WlrLayerShellState::new::<Self>(&display_handle),

            input_state: InputState::new(),
            api_state: ApiState {
                stream: None,
                socket_token: None,
                tx_channel,
            },
            focus_state: FocusState::new(),

            config: Config::default(),

            seat,

            dnd_icon: None,

            socket_name: socket_name.to_string_lossy().to_string(),

            popup_manager: PopupManager::default(),

            windows: vec![],

            xwayland,
            xwm: None,
            xdisplay: None,

            system_processes: sysinfo::System::new_with_specifics(
                RefreshKind::new().with_processes(ProcessRefreshKind::new()),
            ),

            config_join_handle: None,
            grpc_kill_pinger: None,
            grpc_server_join_handle: None,
        };

        // state.schedule(
        //     |data| data.state.xdisplay.is_some(),
        //     move |_| {
        //         tokio::spawn(async move {
        //             grpc_server
        //                 .serve_with_incoming_shutdown(uds_stream, ping_rx_future)
        //                 .await
        //                 .expect("failed to serve grpc");
        //         });
        //     },
        // );

        Ok(state)
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

    pub fn shutdown(&self) {
        tracing::info!("Shutting down Pinnacle");
        if let Some(pinger) = self.grpc_kill_pinger.as_ref() {
            pinger.send(()).expect("failed to send grpc shutdown ping");
        }
        self.loop_signal.stop();
    }
}

pub struct CalloopData {
    pub display_handle: DisplayHandle,
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

/// A trait meant to be used in types with a [`UserDataMap`][smithay::utils::user_data::UserDataMap]
/// to get user-defined state.
pub trait WithState {
    /// The user-defined state
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
