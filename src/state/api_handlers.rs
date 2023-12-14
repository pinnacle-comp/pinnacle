use std::ffi::OsString;

use async_process::Stdio;
use futures_lite::AsyncBufReadExt;
use smithay::{
    desktop::space::SpaceElement,
    input::keyboard::XkbConfig,
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge,
    utils::{Point, Rectangle, SERIAL_COUNTER},
    wayland::{compositor, shell::xdg::XdgToplevelSurfaceData},
};

use crate::{
    config::{
        api::msg::{
            Args, CallbackId, KeyIntOrString, Msg, OutgoingMsg, Request, RequestId, RequestResponse,
        },
        ConnectorSavedState,
    },
    focus::FocusTarget,
    tag::Tag,
    window::WindowElement,
};

use super::{State, WithState};

impl State {
    pub fn handle_msg(&mut self, msg: Msg) {
        // tracing::debug!("Got {msg:?}");
        match msg {
            Msg::SetKeybind {
                key,
                modifiers,
                callback_id,
            } => {
                let key = match key {
                    KeyIntOrString::Int(num) => {
                        tracing::info!("set keybind: {:?}, raw {}", modifiers, num);
                        num
                    }
                    KeyIntOrString::String(s) => {
                        if s.chars().count() == 1 {
                            let Some(ch) = s.chars().next() else { unreachable!() };
                            let raw = xkbcommon::xkb::Keysym::from_char(ch).raw();
                            tracing::info!("set keybind: {:?}, {:?} (raw {})", modifiers, ch, raw);
                            raw
                        } else {
                            let raw = xkbcommon::xkb::keysym_from_name(
                                &s,
                                xkbcommon::xkb::KEYSYM_NO_FLAGS,
                            )
                            .raw();
                            tracing::info!("set keybind: {:?}, {:?}", modifiers, raw);
                            raw
                        }
                    }
                };

                self.input_state
                    .keybinds
                    .insert((modifiers.into(), key.into()), callback_id);
            }
            Msg::SetMousebind {
                modifiers,
                button,
                edge,
                callback_id,
            } => {
                // TODO: maybe validate/parse valid codes?
                self.input_state
                    .mousebinds
                    .insert((modifiers.into(), button, edge), callback_id);
            }
            Msg::CloseWindow { window_id } => {
                if let Some(window) = window_id.window(self) {
                    match window {
                        WindowElement::Wayland(window) => window.toplevel().send_close(),
                        WindowElement::X11(surface) => {
                            surface.close().expect("failed to close x11 win");
                        }
                        WindowElement::X11OverrideRedirect(_) => (),
                    }
                }
            }

            Msg::Spawn {
                command,
                callback_id,
            } => {
                self.handle_spawn(command, callback_id);
            }
            Msg::SetEnv { key, value } => std::env::set_var(key, value),

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
                use crate::window::window_state::FloatingOrTiled;

                let rect = Rectangle::from_loc_and_size(window_loc, window_size);
                window.change_geometry(rect);
                window.with_state(|state| {
                    state.floating_or_tiled = match state.floating_or_tiled {
                        FloatingOrTiled::Floating(_) => FloatingOrTiled::Floating(rect),
                        FloatingOrTiled::Tiled(_) => FloatingOrTiled::Tiled(Some(rect)),
                    }
                });
                if let Some(output) = window.output(self) {
                    self.update_windows(&output);
                    self.schedule_render(&output);
                }
            }
            Msg::MoveWindowToTag { window_id, tag_id } => {
                let Some(window) = window_id.window(self) else { return };
                let Some(tag) = tag_id.tag(self) else { return };
                window.with_state(|state| {
                    state.tags = vec![tag.clone()];
                });
                let Some(output) = tag.output(self) else { return };
                self.update_windows(&output);
                self.schedule_render(&output);
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
                self.schedule_render(&output);
            }
            Msg::ToggleFloating { window_id } => {
                let Some(window) = window_id.window(self) else { return };
                window.toggle_floating();

                let Some(output) = window.output(self) else { return };
                self.update_windows(&output);

                // Sometimes toggling won't change the window size,
                // causing no commit.
                //
                // Schedule a render in case the window moves.
                self.schedule_render(&output);
            }
            Msg::ToggleFullscreen { window_id } => {
                let Some(window) = window_id.window(self) else { return };
                window.toggle_fullscreen();

                let Some(output) = window.output(self) else { return };
                self.update_windows(&output);
                self.schedule_render(&output);
            }
            Msg::ToggleMaximized { window_id } => {
                let Some(window) = window_id.window(self) else { return };
                window.toggle_maximized();

                let Some(output) = window.output(self) else { return };
                self.update_windows(&output);
                self.schedule_render(&output);
            }
            Msg::AddWindowRule { cond, rule } => {
                self.config.window_rules.push((cond, rule));
            }
            Msg::WindowMoveGrab { button } => {
                // TODO: in the future, there may be movable layer surfaces
                let Some((FocusTarget::Window(window), _)) =
                    self.surface_under(self.pointer_location)
                else {
                    return;
                };
                let Some(wl_surf) = window.wl_surface() else { return };
                let seat = self.seat.clone();

                // We use the server one and not the client because windows like Steam don't provide
                // GrabStartData, so we need to create it ourselves.
                crate::grab::move_grab::move_request_server(
                    self,
                    &wl_surf,
                    &seat,
                    SERIAL_COUNTER.next_serial(),
                    button,
                );
            }
            Msg::WindowResizeGrab { button } => {
                // TODO: in the future, there may be movable layer surfaces
                let pointer_loc = self.pointer_location;
                let Some((FocusTarget::Window(window), window_loc)) =
                    self.surface_under(pointer_loc)
                else {
                    return;
                };
                let Some(wl_surf) = window.wl_surface() else { return };

                let window_geometry = window.geometry();
                let window_x = window_loc.x as f64;
                let window_y = window_loc.y as f64;
                let window_width = window_geometry.size.w as f64;
                let window_height = window_geometry.size.h as f64;
                let half_width = window_x + window_width / 2.0;
                let half_height = window_y + window_height / 2.0;
                let full_width = window_x + window_width;
                let full_height = window_y + window_height;

                let edges = match pointer_loc {
                    Point { x, y, .. }
                        if (window_x..=half_width).contains(&x)
                            && (window_y..=half_height).contains(&y) =>
                    {
                        ResizeEdge::TopLeft
                    }
                    Point { x, y, .. }
                        if (half_width..=full_width).contains(&x)
                            && (window_y..=half_height).contains(&y) =>
                    {
                        ResizeEdge::TopRight
                    }
                    Point { x, y, .. }
                        if (window_x..=half_width).contains(&x)
                            && (half_height..=full_height).contains(&y) =>
                    {
                        ResizeEdge::BottomLeft
                    }
                    Point { x, y, .. }
                        if (half_width..=full_width).contains(&x)
                            && (half_height..=full_height).contains(&y) =>
                    {
                        ResizeEdge::BottomRight
                    }
                    _ => ResizeEdge::None,
                };

                crate::grab::resize_grab::resize_request_server(
                    self,
                    &wl_surf,
                    &self.seat.clone(),
                    SERIAL_COUNTER.next_serial(),
                    edges.into(),
                    button,
                );
            }

            // Tags ----------------------------------------
            Msg::ToggleTag { tag_id } => {
                tracing::debug!("ToggleTag");
                if let Some(tag) = tag_id.tag(self) {
                    tag.set_active(!tag.active());
                    if let Some(output) = tag.output(self) {
                        self.update_windows(&output);
                        self.update_focus(&output);
                        self.schedule_render(&output);
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
                self.update_focus(&output);
                self.schedule_render(&output);
            }
            Msg::AddTags {
                output_name,
                tag_names,
            } => {
                let new_tags = tag_names.into_iter().map(Tag::new).collect::<Vec<_>>();
                if let Some(saved_state) = self.config.connector_saved_states.get_mut(&output_name)
                {
                    let mut tags = saved_state.tags.clone();
                    tags.extend(new_tags.clone());
                    saved_state.tags = tags;
                } else {
                    self.config.connector_saved_states.insert(
                        output_name.clone(),
                        ConnectorSavedState {
                            tags: new_tags.clone(),
                            ..Default::default()
                        },
                    );
                }

                if let Some(output) = self
                    .space
                    .outputs()
                    .find(|output| output.name() == output_name.0)
                {
                    output.with_state(|state| {
                        state.tags.extend(new_tags.clone());
                        tracing::debug!("tags added, are now {:?}", state.tags);
                    });

                    // replace tags that windows have that are the same id
                    // (this should only happen on config reload)
                    for tag in new_tags {
                        for window in self.windows.iter() {
                            window.with_state(|state| {
                                for win_tag in state.tags.iter_mut() {
                                    if win_tag.id() == tag.id() {
                                        *win_tag = tag.clone();
                                    }
                                }
                            });
                        }
                    }
                }
            }
            Msg::RemoveTags { tag_ids } => {
                let tags = tag_ids
                    .into_iter()
                    .filter_map(|tag_id| tag_id.tag(self))
                    .collect::<Vec<_>>();

                for tag in tags {
                    for saved_state in self.config.connector_saved_states.values_mut() {
                        saved_state.tags.retain(|tg| tg != &tag);
                    }
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
            }

            Msg::ConnectForAllOutputs { callback_id } => {
                let stream = self
                    .api_state
                    .stream
                    .as_ref()
                    .expect("stream doesn't exist");
                let mut stream = stream.lock().expect("couldn't lock stream");
                for output in self.space.outputs() {
                    crate::config::api::send_to_client(
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
                self.config.output_callback_ids.push(callback_id);
            }
            Msg::SetOutputLocation { output_name, x, y } => {
                if let Some(saved_state) = self.config.connector_saved_states.get_mut(&output_name)
                {
                    if let Some(x) = x {
                        saved_state.loc.x = x;
                    }
                    if let Some(y) = y {
                        saved_state.loc.y = y;
                    }
                } else {
                    self.config.connector_saved_states.insert(
                        output_name.clone(),
                        ConnectorSavedState {
                            loc: (x.unwrap_or_default(), y.unwrap_or_default()).into(),
                            ..Default::default()
                        },
                    );
                }

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
                tracing::debug!("Mapping output {} to {loc:?}", output.name());
                self.update_windows(&output);
            }

            Msg::Quit => {
                tracing::info!("Quitting Pinnacle");
                self.loop_signal.stop();
            }

            Msg::SetXkbConfig {
                rules,
                variant,
                layout,
                model,
                options,
            } => {
                let new_config = XkbConfig {
                    rules: &rules.unwrap_or_default(),
                    model: &model.unwrap_or_default(),
                    layout: &layout.unwrap_or_default(),
                    variant: &variant.unwrap_or_default(),
                    options,
                };
                if let Some(kb) = self.seat.get_keyboard() {
                    if let Err(err) = kb.set_xkb_config(self, new_config) {
                        tracing::error!("Failed to set xkbconfig: {err}");
                    }
                }
            }

            Msg::SetLibinputSetting(setting) => {
                for device in self.input_state.libinput_devices.iter_mut() {
                    // We're just gonna indiscriminately apply everything and ignore errors
                    setting.apply_to_device(device);
                }

                self.input_state.libinput_settings.push(setting);
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
            .expect("Stream doesn't exist")
            .clone();
        let mut stream = stream.lock().expect("Couldn't lock stream");
        match request {
            Request::GetWindows => {
                let window_ids = self
                    .windows
                    .iter()
                    .map(|win| win.with_state(|state| state.id))
                    .collect::<Vec<_>>();

                // FIXME: figure out what to do if error
                crate::config::api::send_to_client(
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
                    WindowElement::X11(surface) | WindowElement::X11OverrideRedirect(surface) => {
                        (Some(surface.class()), Some(surface.title()))
                    }
                });
                let focused = window.as_ref().and_then(|win| {
                    let output = win.output(self)?;
                    self.focused_window(&output).map(|foc_win| win == &foc_win)
                });
                let floating = window
                    .as_ref()
                    .map(|win| win.with_state(|state| state.floating_or_tiled.is_floating()));
                let fullscreen_or_maximized = window
                    .as_ref()
                    .map(|win| win.with_state(|state| state.fullscreen_or_maximized));
                crate::config::api::send_to_client(
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
                crate::config::api::send_to_client(
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
                crate::config::api::send_to_client(
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
                crate::config::api::send_to_client(
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
                crate::config::api::send_to_client(
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
                    .chain(self.xdisplay.map(|xdisp| ("DISPLAY", format!(":{xdisp}")))),
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
            tracing::warn!(
                "tried to run {}, but it doesn't exist",
                program.to_string_lossy()
            );
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
                                crate::config::api::send_to_client(
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
                                crate::config::api::send_to_client(
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
                        crate::config::api::send_to_client(
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
