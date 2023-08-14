// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    cell::RefCell,
    error::Error,
    ffi::OsString,
    os::{fd::AsRawFd, unix::net::UnixStream},
    path::PathBuf,
    process::Stdio,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    api::{
        msg::{Args, CallbackId, Msg, OutgoingMsg, Request, RequestId, RequestResponse},
        PinnacleSocketSource,
    },
    cursor::Cursor,
    focus::FocusState,
    grab::resize_grab::ResizeSurfaceState,
    tag::Tag,
    window::{window_state::LocationRequestState, WindowElement},
};
use calloop::futures::Scheduler;
use futures_lite::AsyncBufReadExt;
use smithay::{
    backend::renderer::element::RenderElementStates,
    desktop::{
        space::SpaceElement,
        utils::{
            surface_presentation_feedback_flags_from_states, surface_primary_scanout_output,
            OutputPresentationFeedback,
        },
        PopupManager, Space,
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
            Display, DisplayHandle,
        },
    },
    utils::{Clock, IsAlive, Logical, Monotonic, Point, Size},
    wayland::{
        compositor::{self, CompositorClientState, CompositorState},
        data_device::DataDeviceState,
        dmabuf::DmabufFeedback,
        fractional_scale::FractionalScaleManagerState,
        output::OutputManagerState,
        primary_selection::PrimarySelectionState,
        shell::{
            wlr_layer::WlrLayerShellState,
            xdg::{XdgShellState, XdgToplevelSurfaceData},
        },
        shm::ShmState,
        socket::ListeningSocketSource,
        viewporter::ViewporterState,
    },
    xwayland::{X11Wm, XWayland, XWaylandEvent},
};

use crate::{backend::Backend, input::InputState};

/// The main state of the application.
pub struct State<B: Backend> {
    pub backend_data: B,

    pub loop_signal: LoopSignal,
    pub loop_handle: LoopHandle<'static, CalloopData<B>>,
    pub display_handle: DisplayHandle,
    pub clock: Clock<Monotonic>,

    pub space: Space<WindowElement>,
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

    pub async_scheduler: Scheduler<()>,

    // TODO: move into own struct
    // |     basically just clean this mess up
    pub output_callback_ids: Vec<CallbackId>,

    pub xwayland: XWayland,
    pub xwm: Option<X11Wm>,
    pub xdisplay: Option<u32>,
}

impl<B: Backend> State<B> {
    pub fn handle_msg(&mut self, msg: Msg) {
        // tracing::debug!("Got {msg:?}");
        match msg {
            Msg::SetKeybind {
                key,
                modifiers,
                callback_id,
            } => {
                tracing::info!("set keybind: {:?}, {}", modifiers, key);
                self.input_state
                    .keybinds
                    .insert((modifiers.into(), key), callback_id);
            }
            Msg::SetMousebind { button: _ } => todo!(),
            Msg::CloseWindow { window_id } => {
                if let Some(window) = window_id.window(self) {
                    match window {
                        WindowElement::Wayland(window) => window.toplevel().send_close(),
                        WindowElement::X11(surface) => {
                            surface.close().expect("failed to close x11 win");
                        }
                    }
                }
            }

            Msg::Spawn {
                command,
                callback_id,
            } => {
                self.handle_spawn(command, callback_id);
            }

            Msg::SetWindowSize {
                window_id,
                width,
                height,
            } => {
                let Some(window) = window_id.window(self) else { return };

                // TODO: tiled vs floating
                // FIXME: this will map unmapped windows at 0,0
                let window_loc = self
                    .space
                    .element_location(&window)
                    .unwrap_or((0, 0).into());
                let mut window_size = window.geometry().size;
                if let Some(width) = width {
                    window_size.w = width;
                }
                if let Some(height) = height {
                    window_size.h = height;
                }
                window.request_size_change(&mut self.space, window_loc, window_size);
            }
            Msg::MoveWindowToTag { window_id, tag_id } => {
                let Some(window) = window_id.window(self) else { return };
                let Some(tag) = tag_id.tag(self) else { return };
                window.with_state(|state| {
                    state.tags = vec![tag.clone()];
                });
                let Some(output) = tag.output(self) else { return };
                self.update_windows(&output);
                // self.re_layout(&output);
            }
            Msg::ToggleTagOnWindow { window_id, tag_id } => {
                let Some(window) = window_id.window(self) else { return };
                let Some(tag) = tag_id.tag(self) else { return };

                window.with_state(|state| {
                    if state.tags.contains(&tag) {
                        state.tags.retain(|tg| tg != &tag);
                    } else {
                        state.tags.push(tag.clone());
                    }
                });

                let Some(output) = tag.output(self) else { return };
                self.update_windows(&output);
                // self.re_layout(&output);
            }
            Msg::ToggleFloating { window_id } => {
                let Some(window) = window_id.window(self) else { return };
                window.toggle_floating();

                let Some(output) = window.output(self) else { return };
                self.update_windows(&output);
            }
            Msg::ToggleFullscreen { window_id } => {
                let Some(window) = window_id.window(self) else { return };
                window.toggle_fullscreen();

                let Some(output) = window.output(self) else { return };
                self.update_windows(&output);
            }
            Msg::ToggleMaximized { window_id } => {
                let Some(window) = window_id.window(self) else { return };
                window.toggle_maximized();

                let Some(output) = window.output(self) else { return };
                self.update_windows(&output);
            }

            // Tags ----------------------------------------
            Msg::ToggleTag { tag_id } => {
                tracing::debug!("ToggleTag");
                if let Some(tag) = tag_id.tag(self) {
                    tag.set_active(!tag.active());
                    if let Some(output) = tag.output(self) {
                        self.update_windows(&output);
                        // self.re_layout(&output);
                    }
                }
            }
            Msg::SwitchToTag { tag_id } => {
                let Some(tag) = tag_id.tag(self) else { return };
                let Some(output) = tag.output(self) else { return };
                output.with_state(|state| {
                    for op_tag in state.tags.iter_mut() {
                        op_tag.set_active(false);
                    }
                    tag.set_active(true);
                });
                self.update_windows(&output);
                // self.re_layout(&output);
            }
            Msg::AddTags {
                output_name,
                tag_names,
            } => {
                if let Some(output) = self
                    .space
                    .outputs()
                    .find(|output| output.name() == output_name)
                {
                    output.with_state(|state| {
                        state.tags.extend(tag_names.iter().cloned().map(Tag::new));
                        tracing::debug!("tags added, are now {:?}", state.tags);
                    });
                }
            }
            Msg::RemoveTags { tag_ids } => {
                let tags = tag_ids.into_iter().filter_map(|tag_id| tag_id.tag(self));
                for tag in tags {
                    let Some(output) = tag.output(self) else { continue };
                    output.with_state(|state| {
                        state.tags.retain(|tg| tg != &tag);
                    });
                }
            }
            Msg::SetLayout { tag_id, layout } => {
                let Some(tag) = tag_id.tag(self) else { return };
                tag.set_layout(layout);
                let Some(output) = tag.output(self) else { return };
                self.update_windows(&output);
                // self.re_layout(&output);
            }

            Msg::ConnectForAllOutputs { callback_id } => {
                let stream = self
                    .api_state
                    .stream
                    .as_ref()
                    .expect("Stream doesn't exist");
                let mut stream = stream.lock().expect("Couldn't lock stream");
                for output in self.space.outputs() {
                    crate::api::send_to_client(
                        &mut stream,
                        &OutgoingMsg::CallCallback {
                            callback_id,
                            args: Some(Args::ConnectForAllOutputs {
                                output_name: output.name(),
                            }),
                        },
                    )
                    .expect("Send to client failed");
                }
                self.output_callback_ids.push(callback_id);
            }
            Msg::SetOutputLocation { output_name, x, y } => {
                let Some(output) = output_name.output(self) else { return };
                let mut loc = output.current_location();
                if let Some(x) = x {
                    loc.x = x;
                }
                if let Some(y) = y {
                    loc.y = y;
                }
                output.change_current_state(None, None, None, Some(loc));
                self.space.map_output(&output, loc);
                tracing::debug!("mapping output {} to {loc:?}", output.name());
                self.update_windows(&output);
                // self.re_layout(&output);
            }

            Msg::Quit => {
                self.loop_signal.stop();
            }

            Msg::Request {
                request_id,
                request,
            } => {
                self.handle_request(request_id, request);
            }
        }
    }

    fn handle_request(&mut self, request_id: RequestId, request: Request) {
        let stream = self
            .api_state
            .stream
            .as_ref()
            .expect("Stream doesn't exist");
        let mut stream = stream.lock().expect("Couldn't lock stream");
        match request {
            Request::GetWindows => {
                let window_ids = self
                    .windows
                    .iter()
                    .map(|win| win.with_state(|state| state.id))
                    .collect::<Vec<_>>();

                // FIXME: figure out what to do if error
                crate::api::send_to_client(
                    &mut stream,
                    &OutgoingMsg::RequestResponse {
                        request_id,
                        response: RequestResponse::Windows { window_ids },
                    },
                )
                .expect("Couldn't send to client");
            }
            Request::GetWindowProps { window_id } => {
                let window = window_id.window(self);
                let size = window
                    .as_ref()
                    .map(|win| (win.geometry().size.w, win.geometry().size.h));
                let loc = window
                    .as_ref()
                    .and_then(|win| self.space.element_location(win))
                    .map(|loc| (loc.x, loc.y));
                let (class, title) = window.as_ref().map_or((None, None), |win| match &win {
                    WindowElement::Wayland(_) => {
                        if let Some(wl_surf) = win.wl_surface() {
                            compositor::with_states(&wl_surf, |states| {
                                let lock = states
                                    .data_map
                                    .get::<XdgToplevelSurfaceData>()
                                    .expect("XdgToplevelSurfaceData wasn't in surface's data map")
                                    .lock()
                                    .expect("failed to acquire lock");
                                (lock.app_id.clone(), lock.title.clone())
                            })
                        } else {
                            (None, None)
                        }
                    }
                    WindowElement::X11(surface) => (Some(surface.class()), Some(surface.title())),
                });
                let focused = window.as_ref().and_then(|win| {
                    self.focus_state
                        .current_focus() // TODO: actual focus
                        .map(|foc_win| win == &foc_win)
                });
                let floating = window
                    .as_ref()
                    .map(|win| win.with_state(|state| state.floating_or_tiled.is_floating()));
                let fullscreen_or_maximized = window
                    .as_ref()
                    .map(|win| win.with_state(|state| state.fullscreen_or_maximized));
                crate::api::send_to_client(
                    &mut stream,
                    &OutgoingMsg::RequestResponse {
                        request_id,
                        response: RequestResponse::WindowProps {
                            size,
                            loc,
                            class,
                            title,
                            focused,
                            floating,
                            fullscreen_or_maximized,
                        },
                    },
                )
                .expect("failed to send to client");
            }
            Request::GetOutputs => {
                let output_names = self
                    .space
                    .outputs()
                    .map(|output| output.name())
                    .collect::<Vec<_>>();
                crate::api::send_to_client(
                    &mut stream,
                    &OutgoingMsg::RequestResponse {
                        request_id,
                        response: RequestResponse::Outputs { output_names },
                    },
                )
                .expect("failed to send to client");
            }
            Request::GetOutputProps { output_name } => {
                let output = self
                    .space
                    .outputs()
                    .find(|output| output.name() == output_name);
                let res = output.as_ref().and_then(|output| {
                    output.current_mode().map(|mode| (mode.size.w, mode.size.h))
                });
                let refresh_rate = output
                    .as_ref()
                    .and_then(|output| output.current_mode().map(|mode| mode.refresh));
                let model = output
                    .as_ref()
                    .map(|output| output.physical_properties().model);
                let physical_size = output.as_ref().map(|output| {
                    (
                        output.physical_properties().size.w,
                        output.physical_properties().size.h,
                    )
                });
                let make = output
                    .as_ref()
                    .map(|output| output.physical_properties().make);
                let loc = output
                    .as_ref()
                    .map(|output| (output.current_location().x, output.current_location().y));
                let focused = self
                    .focus_state
                    .focused_output
                    .as_ref()
                    .and_then(|foc_op| output.map(|op| op == foc_op));
                let tag_ids = output.as_ref().map(|output| {
                    output.with_state(|state| {
                        state.tags.iter().map(|tag| tag.id()).collect::<Vec<_>>()
                    })
                });
                crate::api::send_to_client(
                    &mut stream,
                    &OutgoingMsg::RequestResponse {
                        request_id,
                        response: RequestResponse::OutputProps {
                            make,
                            model,
                            loc,
                            res,
                            refresh_rate,
                            physical_size,
                            focused,
                            tag_ids,
                        },
                    },
                )
                .expect("failed to send to client");
            }
            Request::GetTags => {
                let tag_ids = self
                    .space
                    .outputs()
                    .flat_map(|op| op.with_state(|state| state.tags.clone()))
                    .map(|tag| tag.id())
                    .collect::<Vec<_>>();
                tracing::debug!("GetTags: {:?}", tag_ids);
                crate::api::send_to_client(
                    &mut stream,
                    &OutgoingMsg::RequestResponse {
                        request_id,
                        response: RequestResponse::Tags { tag_ids },
                    },
                )
                .expect("failed to send to client");
            }
            Request::GetTagProps { tag_id } => {
                let tag = tag_id.tag(self);
                let output_name = tag
                    .as_ref()
                    .and_then(|tag| tag.output(self))
                    .map(|output| output.name());
                let active = tag.as_ref().map(|tag| tag.active());
                let name = tag.as_ref().map(|tag| tag.name());
                crate::api::send_to_client(
                    &mut stream,
                    &OutgoingMsg::RequestResponse {
                        request_id,
                        response: RequestResponse::TagProps {
                            active,
                            name,
                            output_name,
                        },
                    },
                )
                .expect("failed to send to client");
            }
        }
    }

    pub fn handle_spawn(&self, command: Vec<String>, callback_id: Option<CallbackId>) {
        let mut command = command.into_iter();
        let Some(program) = command.next() else {
            // TODO: notify that command was nothing
            return;
        };

        let program = OsString::from(program);
        let Ok(mut child) = async_process::Command::new(&program)
            .envs(
                [("WAYLAND_DISPLAY", self.socket_name.clone())]
                    .into_iter()
                    .chain(
                        self.xdisplay.map(|xdisp| ("DISPLAY", format!(":{xdisp}")))
                    )
            )
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
            let stream_out = self
                .api_state
                .stream
                .as_ref()
                .expect("Stream doesn't exist")
                .clone();
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
                            }
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
                            }
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

/// Schedule something to be done when windows have finished committing and have become
/// idle.
pub fn schedule_on_commit<F, B: Backend>(
    data: &mut CalloopData<B>,
    windows: Vec<WindowElement>,
    on_commit: F,
) where
    F: FnOnce(&mut CalloopData<B>) + 'static,
{
    for window in windows.iter().filter(|win| win.alive()) {
        if window.with_state(|state| !matches!(state.loc_request_state, LocationRequestState::Idle))
        {
            // tracing::debug!(
            //     "window state is {:?}",
            //     window.with_state(|state| state.loc_request_state.clone())
            // );
            data.state.loop_handle.insert_idle(|data| {
                schedule_on_commit(data, windows, on_commit);
            });
            return;
        }
    }

    on_commit(data);
}

// Schedule something to be done when `condition` returns true.
pub fn schedule<F1, F2, B: Backend>(data: &mut CalloopData<B>, condition: F1, run: F2)
where
    F1: Fn(&mut CalloopData<B>) -> bool + 'static,
    F2: FnOnce(&mut CalloopData<B>) + 'static,
{
    if !condition(data) {
        data.state.loop_handle.insert_idle(|data| {
            schedule(data, condition, run);
        });
        return;
    }

    run(data);
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

        // We want to replace the client if a new one pops up
        // TODO: there should only ever be one client working at a time, and creating a new client
        // |     when one is already running should be impossible.
        // INFO: this source try_clone()s the stream

        // TODO: probably use anyhow or something
        let socket_source = match PinnacleSocketSource::new(tx_channel) {
            Ok(source) => source,
            Err(err) => {
                tracing::error!("Failed to create the socket source: {err}");
                Err(err)?
            }
        };

        loop_handle.insert_source(socket_source, |stream, _, data| {
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

        let (executor, sched) =
            calloop::futures::executor::<()>().expect("Couldn't create executor");
        loop_handle.insert_source(executor, |_, _, _| {})?;

        start_lua_config()?;

        let display_handle = display.handle();
        let mut seat_state = SeatState::new();
        let mut seat = seat_state.new_wl_seat(&display_handle, backend_data.seat_name());
        seat.add_pointer();
        seat.add_keyboard(XkbConfig::default(), 200, 25)?;

        loop_handle.insert_idle(|data| {
            data.state
                .loop_handle
                .insert_source(rx_channel, |msg, _, data| match msg {
                    Event::Msg(msg) => data.state.handle_msg(msg),
                    Event::Closed => todo!(),
                })
                .expect("failed to insert rx_channel into loop");
        });

        tracing::debug!("before xwayland");
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
                    tracing::debug!("XWaylandEvent ready");
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
        tracing::debug!("after xwayland");

        Ok(Self {
            backend_data,
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

            input_state: InputState::new(),
            api_state: ApiState::new(),
            focus_state: FocusState::new(),

            seat,

            dnd_icon: None,

            move_mode: false,
            socket_name: socket_name.to_string_lossy().to_string(),

            popup_manager: PopupManager::default(),

            async_scheduler: sched,

            windows: vec![],
            output_callback_ids: vec![],

            xwayland,
            xwm: None,
            xdisplay: None,
        })
    }
}

fn start_lua_config() -> Result<(), Box<dyn std::error::Error>> {
    // TODO: move all this into the lua api
    let config_path = std::env::var("PINNACLE_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let default_path = std::env::var("XDG_CONFIG_HOME").unwrap_or("~/.config".to_string());
            let mut default_path = PathBuf::from(default_path);
            default_path.push("pinnacle/init.lua");
            default_path
        });

    let config_path = {
        let path = shellexpand::tilde(&config_path.to_string_lossy().to_string()).to_string();
        PathBuf::from(path)
    };

    if config_path.exists() {
        let lua_path = std::env::var("LUA_PATH").unwrap_or_else(|_| {
            tracing::info!("LUA_PATH was not set, using empty string");
            "".to_string()
        });
        let mut local_lua_path = std::env::current_dir()
            .expect("Couldn't get current dir")
            .to_string_lossy()
            .to_string();
        local_lua_path.push_str("/api/lua"); // TODO: get from crate root and do dynamically
        let new_lua_path =
            format!("{local_lua_path}/?.lua;{local_lua_path}/?/init.lua;{local_lua_path}/lib/?.lua;{local_lua_path}/lib/?/init.lua;{lua_path}");

        let lua_cpath = std::env::var("LUA_CPATH").unwrap_or_else(|_| {
            tracing::info!("LUA_CPATH was not set, using empty string");
            "".to_string()
        });
        let new_lua_cpath = format!("{local_lua_path}/lib/?.so;{lua_cpath}");

        if let Err(err) = std::process::Command::new("lua")
            .arg(config_path)
            .env("LUA_PATH", new_lua_path)
            .env("LUA_CPATH", new_lua_cpath)
            .spawn()
        {
            tracing::error!("Failed to start Lua: {err}");
            return Err(err)?;
        }
        Ok(())
    } else {
        tracing::error!("Could not find config {:?}", config_path);
        if std::env::var("PINNACLE_CONFIG").is_err() {
            tracing::error!("Help: Run Pinnacle with PINNACLE_CONFIG set to a valid config file, or copy the provided example_config.lua to the path mentioned above.");
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "No config found",
        ))?
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
    space: &Space<WindowElement>,
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

    let map = smithay::desktop::layer_map_for_output(output);
    for layer_surface in map.layers() {
        layer_surface.take_presentation_feedback(
            &mut output_presentation_feedback,
            surface_primary_scanout_output,
            |surface, _| {
                surface_presentation_feedback_flags_from_states(surface, render_element_states)
            },
        );
    }

    output_presentation_feedback
}

/// State containing the config API's stream.
#[derive(Default)]
pub struct ApiState {
    // TODO: this may not need to be in an arc mutex because of the move to async
    pub stream: Option<Arc<Mutex<UnixStream>>>,
}

impl ApiState {
    pub fn new() -> Self {
        Default::default()
    }
}

pub trait WithState {
    type State: Default;
    fn with_state<F, T>(&self, func: F) -> T
    where
        F: FnMut(&mut Self::State) -> T;
}

#[derive(Default, Debug)]
pub struct WlSurfaceState {
    pub resize_state: ResizeSurfaceState,
}

impl WithState for WlSurface {
    type State = WlSurfaceState;

    fn with_state<F, T>(&self, mut func: F) -> T
    where
        F: FnMut(&mut Self::State) -> T,
    {
        compositor::with_states(self, |states| {
            states
                .data_map
                .insert_if_missing(RefCell::<Self::State>::default);
            let state = states
                .data_map
                .get::<RefCell<Self::State>>()
                .expect("This should never happen");

            func(&mut state.borrow_mut())
        })
    }
}
