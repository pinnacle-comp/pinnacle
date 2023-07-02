// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::{
    error::Error,
    ffi::OsString,
    os::{fd::AsRawFd, unix::net::UnixStream},
    process::Stdio,
    sync::{Arc, Mutex}, path::Path,
};

use crate::{
    api::{
        msg::{Args, CallbackId, Msg, OutgoingMsg, Request, RequestResponse},
        PinnacleSocketSource,
    },
    focus::FocusState,
    window::{window_state::WindowState, WindowProperties}, output::OutputState, tag::{TagState, Tag}, layout::Layout,
};
use calloop::futures::Scheduler;
use futures_lite::AsyncBufReadExt;
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
            Display,
        },
    },
    utils::{Clock, Logical, Monotonic, Point},
    wayland::{
        compositor::{self, CompositorClientState, CompositorState},
        data_device::DataDeviceState,
        dmabuf::DmabufFeedback,
        fractional_scale::FractionalScaleManagerState,
        output::OutputManagerState,
        shell::xdg::{XdgShellState, XdgToplevelSurfaceData},
        shm::ShmState,
        socket::ListeningSocketSource,
        viewporter::ViewporterState,
    },
};

use crate::{backend::Backend, input::InputState};

/// The main state of the application.
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
    pub tag_state: TagState,

    pub popup_manager: PopupManager,

    pub cursor_status: CursorImageStatus,
    pub pointer_location: Point<f64, Logical>,
    pub windows: Vec<Window>,

    pub async_scheduler: Scheduler<()>,
}

impl<B: Backend> State<B> {
    /// Create the main [`State`].
    ///
    /// This will set the WAYLAND_DISPLAY environment variable, insert Wayland necessary sources
    /// into the event loop, and run an implementation of the config API (currently Lua).
    pub fn init(
        backend_data: B,
        display: &mut Display<Self>,
        loop_signal: LoopSignal,
        loop_handle: LoopHandle<'static, CalloopData<B>>,
    ) -> Result<Self, Box<dyn Error>> {
        let socket = ListeningSocketSource::new_auto()?;
        let socket_name = socket.socket_name().to_os_string();

        std::env::set_var("WAYLAND_DISPLAY", socket_name.clone());

        // Opening a new process will use up a few file descriptors, around 10 for Alacritty, for
        // example. Because of this, opening up only around 100 processes would exhaust the file
        // descriptor limit on my system (Arch btw) and cause a "Too many open files" crash.
        //
        // To fix this, I just set the limit to be higher. As Pinnacle is the whole graphical
        // environment, I *think* this is ok.
        if let Err(err) = smithay::reexports::nix::sys::resource::setrlimit(
                    smithay::reexports::nix::sys::resource::Resource::RLIMIT_NOFILE,
                    65536,
                    65536 * 2,
                ) {
            tracing::error!("Could not raise fd limit: errno {err}");
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
        loop_handle.insert_source(rx_channel, |msg, _, data| match msg {
            Event::Msg(msg) => {
                // TODO: move this into its own function
                // TODO: no like seriously this is getting a bit unwieldy
                // TODO: no like rustfmt literally refuses to format the code below
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
                            .insert((modifiers.into(), key), callback_id);
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
                        data.state.handle_spawn(command, callback_id);
                    }

                    Msg::SetWindowSize { window_id, size } => {
                        let Some(window) = data.state.space.elements().find(|&win| {
                            WindowState::with_state(win, |state| state.id == window_id)
                        }) else { return; };

                        // TODO: tiled vs floating
                        window.toplevel().with_pending_state(|state| {
                            state.size = Some(size.into());
                        });
                        window.toplevel().send_pending_configure();
                    }
                    Msg::MoveToTag { tag_id } => todo!(),
                    Msg::ToggleTag { tag_id } => {
                        let windows = OutputState::with(
                            data
                                .state
                                .focus_state
                                .focused_output
                                .as_ref()
                                .unwrap(), // TODO: handle error
                            |state| {
                                match state.focused_tags.get_mut(&tag_id) {
                                    Some(id) => {
                                        *id = !*id;
                                        tracing::debug!("toggled tag {tag_id:?} {}", if *id { "on" } else { "off" });
                                    }
                                    None => {
                                        state.focused_tags.insert(tag_id.clone(), true);
                                        tracing::debug!("toggled tag {tag_id:?} on");
                                    }
                                }
                                // re-layout
                                for window in data.state.space.elements().cloned().collect::<Vec<_>>() {
                                    let should_render = WindowState::with_state(&window, |win_state| {
                                        for tag_id in win_state.tags.iter() {
                                            if *state.focused_tags.get(tag_id).unwrap_or(&false) {
                                                return true;
                                            }
                                        }
                                        false
                                    });
                                    if !should_render {
                                        data.state.space.unmap_elem(&window);
                                    }
                                }

                                data.state.windows.iter().filter(|&win| {
                                    WindowState::with_state(win, |win_state| {
                                        for tag_id in win_state.tags.iter() {
                                            if *state.focused_tags.get(tag_id).unwrap_or(&false) {
                                                return true;
                                            }
                                        }
                                        false
                                    })
                                }).cloned().collect::<Vec<_>>()
                            }
                        );

                        tracing::info!("Laying out {} windows", windows.len());
                        
                        Layout::master_stack(&mut data.state, windows, crate::layout::Direction::Left);
                    },
                    Msg::SwitchToTag { tag_id } => {
                        let windows = OutputState::with(data
                            .state
                            .focus_state
                            .focused_output
                            .as_ref()
                            .unwrap(), 
                            |state| {
                                for (_, active) in state.focused_tags.iter_mut() {
                                    *active = false;
                                }
                                if let Some(active) = state.focused_tags.get_mut(&tag_id) {
                                    *active = true;
                                } else {
                                    state.focused_tags.insert(tag_id.clone(), true);
                                }

                                // TODO: extract into fn, same with the one up there
                                for window in data.state.space.elements().cloned().collect::<Vec<_>>() {
                                    let should_render = WindowState::with_state(&window, |win_state| {
                                        for tag_id in win_state.tags.iter() {
                                            if *state.focused_tags.get(tag_id).unwrap_or(&false) {
                                                return true;
                                            }
                                        }
                                        false
                                    });
                                    if !should_render {
                                        data.state.space.unmap_elem(&window);
                                    }
                                }

                                data.state.windows.iter().filter(|&win| {
                                    WindowState::with_state(win, |win_state| {
                                        for tag_id in win_state.tags.iter() {
                                            if *state.focused_tags.get(tag_id).unwrap_or(&false) {
                                                return true;
                                            }
                                        }
                                        false
                                    })
                                }).cloned().collect::<Vec<_>>()
                            }
                        );

                        Layout::master_stack(&mut data.state, windows, crate::layout::Direction::Left);
                    }
                    Msg::AddTags { tags } => {
                        data
                            .state
                            .tag_state
                            .tags
                            .extend(
                                tags
                                    .into_iter()
                                    .map(|tag| Tag { id: tag, windows: vec![] })
                            );
                    },
                    Msg::RemoveTags { tags } => {
                        data.state.tag_state.tags.retain(|tag| !tags.contains(&tag.id));
                    },

                    Msg::Quit => {
                        data.state.loop_signal.stop();
                    }

                    Msg::Request(request) => match request {
                        Request::GetWindowByAppId { id, app_id } => todo!(),
                        Request::GetWindowByTitle { id, title } => todo!(),
                        Request::GetWindowByFocus { id } => {
                            let Some(current_focus) = data.state.focus_state.current_focus() else { return; };
                            let (app_id, title) = compositor::with_states(
                                current_focus.toplevel().wl_surface(), 
                                |states| {
                                    let lock = states.
                                        data_map
                                        .get::<XdgToplevelSurfaceData>()
                                        .expect("XdgToplevelSurfaceData doesn't exist")
                                        .lock()
                                        .expect("Couldn't lock XdgToplevelSurfaceData");
                                    (lock.app_id.clone(), lock.title.clone())
                                }
                            );
                            let (window_id, floating) = WindowState::with_state(&current_focus, |state| {
                                (state.id, state.floating.is_floating())
                            });
                            // TODO: unwrap
                            let location = data.state.space.element_location(&current_focus).unwrap(); 
                            let props = WindowProperties {
                                id: window_id,
                                app_id,
                                title,
                                size: current_focus.geometry().size.into(),
                                location: location.into(),
                                floating,
                            };
                            let stream = data.state.api_state.stream.as_ref().expect("Stream doesn't exist");
                            let mut stream = stream.lock().expect("Couldn't lock stream");
                            crate::api::send_to_client(
                                &mut stream, 
                                &OutgoingMsg::RequestResponse { 
                                    request_id: id, 
                                    response: RequestResponse::Window { window: props }
                                }
                            )
                            .expect("Send to client failed");
                        },
                        Request::GetAllWindows { id } => {
                            let window_props = data.state.space.elements().map(|win| {

                                let (app_id, title) = compositor::with_states(
                                    win.toplevel().wl_surface(), 
                                    |states| {
                                        let lock = states.
                                            data_map
                                            .get::<XdgToplevelSurfaceData>()
                                            .expect("XdgToplevelSurfaceData doesn't exist")
                                            .lock()
                                            .expect("Couldn't lock XdgToplevelSurfaceData");
                                        (lock.app_id.clone(), lock.title.clone())
                                    }
                                );
                                let (window_id, floating) = WindowState::with_state(win, |state| {
                                    (state.id, state.floating.is_floating())
                                });
                                // TODO: unwrap
                                let location = data.state.space.element_location(win).expect("Window location doesn't exist"); 
                                WindowProperties {
                                    id: window_id,
                                    app_id,
                                    title,
                                    size: win.geometry().size.into(),
                                    location: location.into(),
                                    floating,
                                }
                            }).collect::<Vec<_>>();

                            // FIXME: figure out what to do if error
                            let stream = data.state.api_state.stream.as_ref().expect("Stream doesn't exist");
                            let mut stream = stream.lock().expect("Couldn't lock stream");
                            crate::api::send_to_client(
                                &mut stream, 
                                &OutgoingMsg::RequestResponse { 
                                    request_id: id, 
                                    response: RequestResponse::GetAllWindows { windows: window_props },
                                }
                            )
                            .expect("Couldn't send to client");
                        }
                    },
                };
            }
            Event::Closed => todo!(),
        })?;

        // We want to replace the client if a new one pops up
        // TODO: there should only ever be one client working at a time, and creating a new client
        // |     when one is already running should be impossible.
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
                    .expect("Couldn't lock old stream")
                    .shutdown(std::net::Shutdown::Both)
                    .expect("Couldn't shutdown old stream");
            }
        })?;

        let (executor, sched) = calloop::futures::executor::<()>().expect("Couldn't create executor");
        loop_handle.insert_source(executor, |_, _, _| {})?;

        // TODO: move all this into the lua api
        let config_path = std::env::var("PINNACLE_CONFIG").unwrap_or_else(|_| {
            let mut default_path =
                std::env::var("XDG_CONFIG_HOME").unwrap_or("~/.config".to_string());
            default_path.push_str("/pinnacle/init.lua");
            default_path
        });

        if Path::new(&config_path).exists() {
            let lua_path = std::env::var("LUA_PATH").expect("Lua is not installed!");
            let mut local_lua_path = std::env::current_dir()
                .expect("Couldn't get current dir")
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
                .expect("Could not start config process");
        } else {
            tracing::error!("Could not find {}", config_path);
        }


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
            tag_state: TagState::new(),

            seat,

            move_mode: false,
            socket_name: socket_name.to_string_lossy().to_string(),

            popup_manager: PopupManager::default(),

            async_scheduler: sched,

            windows: vec![],
        })
    }

    pub fn handle_spawn(&self, command: Vec<String>, callback_id: Option<CallbackId>) {
        let mut command = command.into_iter();
        let Some(program) = command.next() else {
            // TODO: notify that command was nothing
            return;
        };

        let program = OsString::from(program);
        let Ok(mut child) = async_process::Command::new(&program)
            .env("WAYLAND_DISPLAY", self.socket_name.clone())
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
        else {
            // TODO: notify user that program doesn't exist
            tracing::warn!("tried to run {}, but it doesn't exist", program.to_string_lossy());
            return;
        };

        if let Some(callback_id) = callback_id {
            let stdout = child.stdout.take();
            let stderr = child.stderr.take();
            let stream_out = self.api_state.stream.as_ref().expect("Stream doesn't exist").clone();
            let stream_err = stream_out.clone();
            let stream_exit = stream_out.clone();

            if let Some(stdout) = stdout {
                let future = async move {
                    // TODO: use BufReader::new().lines()
                    let mut reader = futures_lite::io::BufReader::new(stdout);
                    loop {
                        let mut buf = String::new();
                        match reader.read_line(&mut buf).await {
                            Ok(0) => break,
                            Ok(_) => {
                                let mut stream = stream_out.lock().expect("Couldn't lock stream");
                                crate::api::send_to_client(
                                    &mut stream,
                                    &OutgoingMsg::CallCallback {
                                        callback_id,
                                        args: Some(Args::Spawn {
                                            stdout: Some(buf.trim_end_matches('\n').to_string()),
                                            stderr: None,
                                            exit_code: None,
                                            exit_msg: None,
                                        }),
                                    },
                                )
                                .expect("Send to client failed"); // TODO: notify instead of crash
                            }
                            Err(err) => {
                                tracing::warn!("child read err: {err}");
                                break;
                            },
                        }
                    }
                };

                // This is not important enough to crash on error, so just print the error instead
                if let Err(err) = self.async_scheduler.schedule(future) {
                    tracing::error!("Failed to schedule future: {err}");
                }
            }
            if let Some(stderr) = stderr {
                let future = async move {
                    let mut reader = futures_lite::io::BufReader::new(stderr);
                    loop {
                        let mut buf = String::new();
                        match reader.read_line(&mut buf).await {
                            Ok(0) => break,
                            Ok(_) => {
                                let mut stream = stream_err.lock().expect("Couldn't lock stream");
                                crate::api::send_to_client(
                                    &mut stream,
                                    &OutgoingMsg::CallCallback {
                                        callback_id,
                                        args: Some(Args::Spawn {
                                            stdout: None,
                                            stderr: Some(buf.trim_end_matches('\n').to_string()),
                                            exit_code: None,
                                            exit_msg: None,
                                        }),
                                    },
                                )
                                .expect("Send to client failed"); // TODO: notify instead of crash
                            }
                            Err(err) => {
                                tracing::warn!("child read err: {err}");
                                break;
                            },
                        }
                    }
                };
                if let Err(err) = self.async_scheduler.schedule(future) {
                    tracing::error!("Failed to schedule future: {err}");
                }
            }

            let future = async move {
                match child.status().await {
                    Ok(exit_status) => {
                        let mut stream = stream_exit.lock().expect("Couldn't lock stream");
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
                        .expect("Send to client failed"); // TODO: notify instead of crash
                    }
                    Err(err) => {
                        tracing::warn!("child wait() err: {err}");
                    }
                }
            };
            if let Err(err) = self.async_scheduler.schedule(future) {
                tracing::error!("Failed to schedule future: {err}");
            }
        }
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
}

#[derive(Debug, Copy, Clone)]
pub struct SurfaceDmabufFeedback<'a> {
    pub render_feedback: &'a DmabufFeedback,
    pub scanout_feedback: &'a DmabufFeedback,
}

// TODO: docs
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

/// State containing the config API's stream.
#[derive(Default)]
pub struct ApiState {
    pub stream: Option<Arc<Mutex<UnixStream>>>,
}

impl ApiState {
    pub fn new() -> Self {
        Default::default()
    }
}
