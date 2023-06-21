use std::{
    error::Error,
    ffi::OsString,
    io::{BufRead, BufReader},
    os::{fd::AsRawFd, unix::net::UnixStream},
    process::Stdio,
    sync::{Arc, Mutex},
};

use crate::{
    api::{
        msg::{Args, CallbackId, Msg, OutgoingMsg},
        PinnacleSocketSource,
    },
    focus::FocusState,
};
use smithay::{
    backend::renderer::element::RenderElementStates,
    desktop::{
        utils::{
            surface_presentation_feedback_flags_from_states, surface_primary_scanout_output,
            OutputPresentationFeedback,
        },
        PopupManager, Space, Window,
    },
    input::{keyboard::XkbConfig, pointer::CursorImageStatus, Seat, SeatState},
    output::Output,
    reexports::{
        calloop::{
            self, channel::Event, generic::Generic, Interest, LoopHandle, LoopSignal, Mode,
            PostAction,
        },
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::wl_surface::WlSurface,
            Display,
        },
    },
    utils::{Clock, Logical, Monotonic, Point},
    wayland::{
        compositor::{CompositorClientState, CompositorState},
        data_device::DataDeviceState,
        dmabuf::DmabufFeedback,
        fractional_scale::FractionalScaleManagerState,
        output::OutputManagerState,
        seat::WaylandFocus,
        shell::xdg::XdgShellState,
        shm::ShmState,
        socket::ListeningSocketSource,
        viewporter::ViewporterState,
    },
};

use crate::{backend::Backend, input::InputState};

pub struct State<B: Backend> {
    pub backend_data: B,

    pub loop_signal: LoopSignal,
    pub loop_handle: LoopHandle<'static, CalloopData<B>>,
    pub clock: Clock<Monotonic>,

    pub space: Space<Window>,
    pub move_mode: bool,
    pub socket_name: String,

    pub seat: Seat<State<B>>,

    pub compositor_state: CompositorState,
    pub data_device_state: DataDeviceState,
    pub seat_state: SeatState<Self>,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub xdg_shell_state: XdgShellState,
    pub viewporter_state: ViewporterState,
    pub fractional_scale_manager_state: FractionalScaleManagerState,
    pub input_state: InputState,
    pub api_state: ApiState,
    pub focus_state: FocusState,

    pub popup_manager: PopupManager,

    pub cursor_status: CursorImageStatus,
    pub pointer_location: Point<f64, Logical>,
}

impl<B: Backend> State<B> {
    pub fn init(
        backend_data: B,
        display: &mut Display<Self>,
        loop_signal: LoopSignal,
        loop_handle: LoopHandle<'static, CalloopData<B>>,
    ) -> Result<Self, Box<dyn Error>> {
        let socket = ListeningSocketSource::new_auto()?;
        let socket_name = socket.socket_name().to_os_string();

        std::env::set_var("WAYLAND_DISPLAY", socket_name.clone());

        loop_handle.insert_source(socket, |stream, _metadata, data| {
            data.display
                .handle()
                .insert_client(stream, Arc::new(ClientState::default()))
                .unwrap();
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
        loop_handle.insert_source(rx_channel, |msg, _, data| match msg {
            Event::Msg(msg) => {
                // TODO: move this into its own function
                match msg {
                    Msg::SetKeybind {
                        key,
                        modifiers,
                        callback_id,
                    } => {
                        tracing::info!("set keybind: {:?}, {}", modifiers, key);
                        data.state
                            .input_state
                            .keybinds
                            .insert((modifiers.into(), key), callback_id.0);
                    }
                    Msg::SetMousebind { button } => todo!(),
                    Msg::CloseWindow { client_id } => {
                        // TODO: client_id
                        tracing::info!("CloseWindow {:?}", client_id);
                        if let Some(window) = data.state.focus_state.current_focus() {
                            window.toplevel().send_close();
                        }
                    }
                    Msg::ToggleFloating { client_id } => {
                        // TODO: add client_ids
                        if let Some(window) = data.state.focus_state.current_focus() {
                            crate::window::toggle_floating(&mut data.state, &window);
                        }
                    }

                    Msg::Spawn {
                        command,
                        callback_id,
                    } => {
                        let mut command = command.into_iter().peekable();
                        if command.peek().is_none() {
                            // TODO: notify that command was nothing
                            return;
                        }

                        // TODO: may need to set env for WAYLAND_DISPLAY
                        let mut child =
                            std::process::Command::new(OsString::from(command.next().unwrap()))
                                .env("WAYLAND_DISPLAY", data.state.socket_name.clone())
                                .stdin(if callback_id.is_some() {
                                    Stdio::piped()
                                } else {
                                    // piping to null because foot won't open without a callback_id
                                    // otherwise
                                    Stdio::null()
                                })
                                .stdout(if callback_id.is_some() {
                                    Stdio::piped()
                                } else {
                                    Stdio::null()
                                })
                                .stderr(if callback_id.is_some() {
                                    Stdio::piped()
                                } else {
                                    Stdio::null()
                                })
                                .args(command)
                                .spawn()
                                .unwrap(); // TODO: handle unwrap

                        // TODO: find a way to make this hellish code look better, deal with unwraps
                        if let Some(callback_id) = callback_id {
                            let stdout = child.stdout.take();
                            let stderr = child.stderr.take();
                            let stream_out = data.state.api_state.stream.as_ref().unwrap().clone();
                            let stream_err = stream_out.clone();
                            let stream_exit = stream_out.clone();

                            if let Some(stdout) = stdout {
                                std::thread::spawn(move || {
                                    // TODO: maybe find a way to make this async?
                                    let mut reader = BufReader::new(stdout);
                                    loop {
                                        let mut buf = String::new();
                                        match reader.read_line(&mut buf) {
                                            Ok(0) => break, // stream closed
                                            Ok(_) => {
                                                let mut stream = stream_out.lock().unwrap();
                                                crate::api::send_to_client(
                                                    &mut stream,
                                                    &OutgoingMsg::CallCallback {
                                                        callback_id,
                                                        args: Some(Args::Spawn {
                                                            stdout: Some(
                                                                buf.trim_end_matches('\n')
                                                                    .to_string(),
                                                            ),
                                                            stderr: None,
                                                            exit_code: None,
                                                            exit_msg: None,
                                                        }),
                                                    },
                                                )
                                                .unwrap();
                                            }
                                            Err(err) => {
                                                tracing::error!("child read err: {err}");
                                                break;
                                            }
                                        }
                                    }
                                });
                            }
                            if let Some(stderr) = stderr {
                                std::thread::spawn(move || {
                                    let mut reader = BufReader::new(stderr);
                                    loop {
                                        let mut buf = String::new();
                                        match reader.read_line(&mut buf) {
                                            Ok(0) => break, // stream closed
                                            Ok(_) => {
                                                let mut stream = stream_err.lock().unwrap();
                                                crate::api::send_to_client(
                                                    &mut stream,
                                                    &OutgoingMsg::CallCallback {
                                                        callback_id,
                                                        args: Some(Args::Spawn {
                                                            stdout: None,
                                                            stderr: Some(
                                                                buf.trim_end_matches('\n')
                                                                    .to_string(),
                                                            ),
                                                            exit_code: None,
                                                            exit_msg: None,
                                                        }),
                                                    },
                                                )
                                                .unwrap();
                                            }
                                            Err(err) => {
                                                tracing::error!("child read err: {err}");
                                                break;
                                            }
                                        }
                                    }
                                });
                            }
                            std::thread::spawn(move || match child.wait() {
                                Ok(exit_status) => {
                                    let mut stream = stream_exit.lock().unwrap();
                                    crate::api::send_to_client(
                                        &mut stream,
                                        &OutgoingMsg::CallCallback {
                                            callback_id,
                                            args: Some(Args::Spawn {
                                                stdout: None,
                                                stderr: None,
                                                exit_code: exit_status.code(),
                                                exit_msg: Some(exit_status.to_string()),
                                            }),
                                        },
                                    )
                                    .unwrap()
                                }
                                Err(err) => {
                                    tracing::warn!("child wait() err: {err}");
                                }
                            });
                        }
                    }
                    Msg::SpawnShell {
                        shell,
                        command,
                        callback_id,
                    } => todo!(),
                    Msg::Quit => {
                        data.state.loop_signal.stop();
                    }
                };
            }
            Event::Closed => todo!(),
        })?;

        // We want to replace the client if a new one pops up
        // INFO: this source try_clone()s the stream
        loop_handle.insert_source(PinnacleSocketSource::new(tx_channel)?, |stream, _, data| {
            if let Some(old_stream) = data
                .state
                .api_state
                .stream
                .replace(Arc::new(Mutex::new(stream)))
            {
                old_stream
                    .lock()
                    .unwrap()
                    .shutdown(std::net::Shutdown::Both)
                    .unwrap();
            }
        })?;

        // TODO: move all this into the lua api
        let config_path = std::env::var("PINNACLE_CONFIG").unwrap_or_else(|_| {
            let mut default_path =
                std::env::var("XDG_CONFIG_HOME").unwrap_or("~/.config".to_string());
            default_path.push_str("/pinnacle/init.lua");
            default_path
        });

        let lua_path = std::env::var("LUA_PATH").expect("Lua is not installed!");
        let mut local_lua_path = std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .to_string();
        local_lua_path.push_str("/api/lua"); // TODO: get from crate root and do dynamically
        let new_lua_path =
            format!("{local_lua_path}/?.lua;{local_lua_path}/?/init.lua;{local_lua_path}/lib/?.lua;{local_lua_path}/lib/?/init.lua;{lua_path}");

        let lua_cpath = std::env::var("LUA_CPATH").expect("Lua is not installed!");
        let new_lua_cpath = format!("{local_lua_path}/lib/?.so;{lua_cpath}");

        std::process::Command::new("lua5.4")
            .arg(config_path)
            .env("LUA_PATH", new_lua_path)
            .env("LUA_CPATH", new_lua_cpath)
            .spawn()
            .unwrap();

        let display_handle = display.handle();
        let mut seat_state = SeatState::new();
        let mut seat = seat_state.new_wl_seat(&display_handle, backend_data.seat_name());
        seat.add_pointer();
        seat.add_keyboard(XkbConfig::default(), 200, 25)?;

        Ok(Self {
            backend_data,
            loop_signal,
            loop_handle,
            clock: Clock::<Monotonic>::new()?,
            compositor_state: CompositorState::new::<Self>(&display_handle),
            data_device_state: DataDeviceState::new::<Self>(&display_handle),
            seat_state,
            pointer_location: (0.0, 0.0).into(),
            shm_state: ShmState::new::<Self>(&display_handle, vec![]),
            space: Space::<Window>::default(),
            cursor_status: CursorImageStatus::Default,
            output_manager_state: OutputManagerState::new_with_xdg_output::<Self>(&display_handle),
            xdg_shell_state: XdgShellState::new::<Self>(&display_handle),
            viewporter_state: ViewporterState::new::<Self>(&display_handle),
            fractional_scale_manager_state: FractionalScaleManagerState::new::<Self>(
                &display_handle,
            ),
            input_state: InputState::new(),
            api_state: ApiState::new(),
            focus_state: FocusState::new(),

            seat,

            move_mode: false,
            socket_name: socket_name.to_string_lossy().to_string(),

            popup_manager: PopupManager::default(),
        })
    }

    /// Returns the [Window] associated with a given [WlSurface].
    pub fn window_for_surface(&self, surface: &WlSurface) -> Option<Window> {
        self.space
            .elements()
            .find(|window| window.wl_surface().map(|s| s == *surface).unwrap_or(false))
            .cloned()
    }
}

pub struct CalloopData<B: Backend> {
    pub display: Display<State<B>>,
    pub state: State<B>,
}

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}
impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}

    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}

    // fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {}
}

#[derive(Debug, Copy, Clone)]
pub struct SurfaceDmabufFeedback<'a> {
    pub render_feedback: &'a DmabufFeedback,
    pub scanout_feedback: &'a DmabufFeedback,
}

pub fn take_presentation_feedback(
    output: &Output,
    space: &Space<Window>,
    render_element_states: &RenderElementStates,
) -> OutputPresentationFeedback {
    let mut output_presentation_feedback = OutputPresentationFeedback::new(output);

    space.elements().for_each(|window| {
        if space.outputs_for_element(window).contains(output) {
            window.take_presentation_feedback(
                &mut output_presentation_feedback,
                surface_primary_scanout_output,
                |surface, _| {
                    surface_presentation_feedback_flags_from_states(surface, render_element_states)
                },
            );
        }
    });
    // let map = smithay::desktop::layer_map_for_output(output);
    // for layer_surface in map.layers() {
    //     layer_surface.take_presentation_feedback(
    //         &mut output_presentation_feedback,
    //         surface_primary_scanout_output,
    //         |surface, _| {
    //             surface_presentation_feedback_flags_from_states(surface, render_element_states)
    //         },
    //     );
    // }

    output_presentation_feedback
}

#[derive(Default)]
pub struct ApiState {
    pub stream: Option<Arc<Mutex<UnixStream>>>,
}

impl ApiState {
    pub fn new() -> Self {
        Default::default()
    }
}
