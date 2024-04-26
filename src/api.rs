pub mod layout;
pub mod signal;
pub mod window;

use std::{ffi::OsString, pin::Pin, process::Stdio};

use pinnacle_api_defs::pinnacle::{
    input::v0alpha1::{
        input_service_server,
        set_libinput_setting_request::{AccelProfile, ClickMethod, ScrollMethod, TapButtonMap},
        set_mousebind_request::MouseEdge,
        SetKeybindRequest, SetKeybindResponse, SetLibinputSettingRequest, SetMousebindRequest,
        SetMousebindResponse, SetRepeatRateRequest, SetXkbConfigRequest,
    },
    output::{
        self,
        v0alpha1::{
            output_service_server, set_scale_request::AbsoluteOrRelative, SetLocationRequest,
            SetModeRequest, SetScaleRequest, SetTransformRequest,
        },
    },
    process::v0alpha1::{process_service_server, SetEnvRequest, SpawnRequest, SpawnResponse},
    render::v0alpha1::{
        render_service_server, Filter, SetDownscaleFilterRequest, SetUpscaleFilterRequest,
    },
    tag::{
        self,
        v0alpha1::{
            tag_service_server, AddRequest, AddResponse, RemoveRequest, SetActiveRequest,
            SwitchToRequest,
        },
    },
    v0alpha1::{
        pinnacle_service_server, PingRequest, PingResponse, QuitRequest, ReloadConfigRequest,
        SetOrToggle, ShutdownWatchRequest, ShutdownWatchResponse,
    },
};
use smithay::{
    backend::renderer::TextureFilter,
    input::keyboard::XkbConfig,
    output::Scale,
    reexports::{calloop, input as libinput},
};
use sysinfo::ProcessRefreshKind;
use tokio::{
    io::AsyncBufReadExt,
    sync::mpsc::{unbounded_channel, UnboundedSender},
    task::JoinHandle,
};
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};
use tracing::{debug, error, info, trace, warn};

use crate::{
    backend::BackendData,
    config::ConnectorSavedState,
    input::ModifierMask,
    output::OutputName,
    state::{State, WithState},
    tag::{Tag, TagId},
};

type ResponseStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;
pub type StateFnSender = calloop::channel::Sender<Box<dyn FnOnce(&mut State) + Send>>;

async fn run_unary_no_response<F>(
    fn_sender: &StateFnSender,
    with_state: F,
) -> Result<Response<()>, Status>
where
    F: FnOnce(&mut State) + Send + 'static,
{
    fn_sender
        .send(Box::new(with_state))
        .map_err(|_| Status::internal("failed to execute request"))?;

    Ok(Response::new(()))
}

async fn run_unary<F, T>(fn_sender: &StateFnSender, with_state: F) -> Result<Response<T>, Status>
where
    F: FnOnce(&mut State) -> T + Send + 'static,
    T: Send + 'static,
{
    let (sender, receiver) = tokio::sync::oneshot::channel::<T>();

    let f = Box::new(|state: &mut State| {
        // TODO: find a way to handle this error
        if sender.send(with_state(state)).is_err() {
            warn!("failed to send result of API call to config; receiver already dropped");
        }
    });

    fn_sender
        .send(f)
        .map_err(|_| Status::internal("failed to execute request"))?;

    receiver.await.map(Response::new).map_err(|err| {
        Status::internal(format!(
            "failed to transfer response for transport to client: {err}"
        ))
    })
}

fn run_server_streaming<F, T>(
    fn_sender: &StateFnSender,
    with_state: F,
) -> Result<Response<ResponseStream<T>>, Status>
where
    F: FnOnce(&mut State, UnboundedSender<Result<T, Status>>) + Send + 'static,
    T: Send + 'static,
{
    let (sender, receiver) = unbounded_channel::<Result<T, Status>>();

    let f = Box::new(|state: &mut State| {
        with_state(state, sender);
    });

    fn_sender
        .send(f)
        .map_err(|_| Status::internal("failed to execute request"))?;

    let receiver_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);
    Ok(Response::new(Box::pin(receiver_stream)))
}

/// Begin a bidirectional grpc stream.
///
/// # Parameters
/// - `fn_sender`: The function sender
/// - `in_stream`: The incoming client stream
/// - `on_client_request`: A callback that will be run with every received request.
/// - `with_out_stream_and_in_stream_join_handle`:
///     Do something with the outbound server-to-client stream.
///     This also receives the join handle for the tokio task listening to
///     the incoming client-to-server stream.
fn run_bidirectional_streaming<F1, F2, I, O>(
    fn_sender: StateFnSender,
    mut in_stream: Streaming<I>,
    on_client_request: F1,
    with_out_stream_and_in_stream_join_handle: F2,
) -> Result<Response<ResponseStream<O>>, Status>
where
    F1: Fn(&mut State, Result<I, Status>) + Clone + Send + 'static,
    F2: FnOnce(&mut State, UnboundedSender<Result<O, Status>>, JoinHandle<()>) + Send + 'static,
    I: Send + 'static,
    O: Send + 'static,
{
    let (sender, receiver) = unbounded_channel::<Result<O, Status>>();

    let fn_sender_clone = fn_sender.clone();

    let with_in_stream = async move {
        while let Some(request) = in_stream.next().await {
            let on_client_request = on_client_request.clone();
            // TODO: handle error
            let _ = fn_sender_clone.send(Box::new(move |state: &mut State| {
                on_client_request(state, request);
            }));
        }
    };

    let join_handle = tokio::spawn(with_in_stream);

    let with_out_stream_and_in_stream_join_handle = Box::new(|state: &mut State| {
        with_out_stream_and_in_stream_join_handle(state, sender, join_handle);
    });

    fn_sender
        .send(with_out_stream_and_in_stream_join_handle)
        .map_err(|_| Status::internal("failed to execute request"))?;

    let receiver_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);
    Ok(Response::new(Box::pin(receiver_stream)))
}

pub struct PinnacleService {
    sender: StateFnSender,
}

impl PinnacleService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}

#[tonic::async_trait]
impl pinnacle_service_server::PinnacleService for PinnacleService {
    type ShutdownWatchStream = ResponseStream<ShutdownWatchResponse>;

    async fn quit(&self, _request: Request<QuitRequest>) -> Result<Response<()>, Status> {
        trace!("PinnacleService.quit");

        run_unary_no_response(&self.sender, |state| {
            state.shutdown();
        })
        .await
    }

    async fn reload_config(
        &self,
        _request: Request<ReloadConfigRequest>,
    ) -> Result<Response<()>, Status> {
        run_unary_no_response(&self.sender, |state| {
            info!("Reloading config");
            state
                .start_config(Some(
                    state.pinnacle.config.dir(&state.pinnacle.xdg_base_dirs),
                ))
                .expect("failed to restart config");
        })
        .await
    }

    async fn ping(&self, request: Request<PingRequest>) -> Result<Response<PingResponse>, Status> {
        let payload = request.into_inner().payload;
        Ok(Response::new(PingResponse { payload }))
    }

    async fn shutdown_watch(
        &self,
        _request: Request<ShutdownWatchRequest>,
    ) -> Result<Response<Self::ShutdownWatchStream>, Status> {
        run_server_streaming(&self.sender, |state, sender| {
            state.pinnacle.config.shutdown_sender.replace(sender);
        })
    }
}

pub struct InputService {
    sender: StateFnSender,
}

impl InputService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}

#[tonic::async_trait]
impl input_service_server::InputService for InputService {
    type SetKeybindStream = ResponseStream<SetKeybindResponse>;
    type SetMousebindStream = ResponseStream<SetMousebindResponse>;

    async fn set_keybind(
        &self,
        request: Request<SetKeybindRequest>,
    ) -> Result<Response<Self::SetKeybindStream>, Status> {
        let request = request.into_inner();

        // TODO: impl From<&[Modifier]> for ModifierMask
        let modifiers = request
            .modifiers()
            .fold(ModifierMask::empty(), |acc, modifier| match modifier {
                pinnacle_api_defs::pinnacle::input::v0alpha1::Modifier::Unspecified => acc,
                pinnacle_api_defs::pinnacle::input::v0alpha1::Modifier::Shift => {
                    acc | ModifierMask::SHIFT
                }
                pinnacle_api_defs::pinnacle::input::v0alpha1::Modifier::Ctrl => {
                    acc | ModifierMask::CTRL
                }
                pinnacle_api_defs::pinnacle::input::v0alpha1::Modifier::Alt => {
                    acc | ModifierMask::ALT
                }
                pinnacle_api_defs::pinnacle::input::v0alpha1::Modifier::Super => {
                    acc | ModifierMask::SUPER
                }
            });
        let key = request
            .key
            .ok_or_else(|| Status::invalid_argument("no key specified"))?;

        use pinnacle_api_defs::pinnacle::input::v0alpha1::set_keybind_request::Key;
        let keysym = match key {
            Key::RawCode(num) => {
                debug!("Set keybind: {:?}, raw {}", modifiers, num);
                xkbcommon::xkb::Keysym::new(num)
            }
            Key::XkbName(s) => {
                if s.chars().count() == 1 {
                    let Some(ch) = s.chars().next() else { unreachable!() };
                    let keysym = xkbcommon::xkb::Keysym::from_char(ch);
                    debug!("Set keybind: {:?}, {:?}", modifiers, keysym);
                    keysym
                } else {
                    let keysym =
                        xkbcommon::xkb::keysym_from_name(&s, xkbcommon::xkb::KEYSYM_NO_FLAGS);
                    debug!("Set keybind: {:?}, {:?}", modifiers, keysym);
                    keysym
                }
            }
        };

        run_server_streaming(&self.sender, move |state, sender| {
            state
                .pinnacle
                .input_state
                .keybinds
                .insert((modifiers, keysym), sender);
        })
    }

    async fn set_mousebind(
        &self,
        request: Request<SetMousebindRequest>,
    ) -> Result<Response<Self::SetMousebindStream>, Status> {
        let request = request.into_inner();

        debug!(request = ?request);

        let modifiers = request
            .modifiers()
            .fold(ModifierMask::empty(), |acc, modifier| match modifier {
                pinnacle_api_defs::pinnacle::input::v0alpha1::Modifier::Unspecified => acc,
                pinnacle_api_defs::pinnacle::input::v0alpha1::Modifier::Shift => {
                    acc | ModifierMask::SHIFT
                }
                pinnacle_api_defs::pinnacle::input::v0alpha1::Modifier::Ctrl => {
                    acc | ModifierMask::CTRL
                }
                pinnacle_api_defs::pinnacle::input::v0alpha1::Modifier::Alt => {
                    acc | ModifierMask::ALT
                }
                pinnacle_api_defs::pinnacle::input::v0alpha1::Modifier::Super => {
                    acc | ModifierMask::SUPER
                }
            });
        let button = request
            .button
            .ok_or_else(|| Status::invalid_argument("no key specified"))?;

        let edge = request.edge();

        if let MouseEdge::Unspecified = edge {
            return Err(Status::invalid_argument("press or release not specified"));
        }

        run_server_streaming(&self.sender, move |state, sender| {
            state
                .pinnacle
                .input_state
                .mousebinds
                .insert((modifiers, button, edge), sender);
        })
    }

    async fn set_xkb_config(
        &self,
        request: Request<SetXkbConfigRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        run_unary_no_response(&self.sender, move |state| {
            let new_config = XkbConfig {
                rules: request.rules(),
                variant: request.variant(),
                model: request.model(),
                layout: request.layout(),
                options: request.options.clone(),
            };
            if let Some(kb) = state.pinnacle.seat.get_keyboard() {
                if let Err(err) = kb.set_xkb_config(state, new_config) {
                    error!("Failed to set xkbconfig: {err}");
                }
            }
        })
        .await
    }

    async fn set_repeat_rate(
        &self,
        request: Request<SetRepeatRateRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let rate = request
            .rate
            .ok_or_else(|| Status::invalid_argument("no rate specified"))?;
        let delay = request
            .delay
            .ok_or_else(|| Status::invalid_argument("no rate specified"))?;

        run_unary_no_response(&self.sender, move |state| {
            if let Some(kb) = state.pinnacle.seat.get_keyboard() {
                kb.change_repeat_info(rate, delay);
            }
        })
        .await
    }

    async fn set_libinput_setting(
        &self,
        request: Request<SetLibinputSettingRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let setting = request
            .setting
            .ok_or_else(|| Status::invalid_argument("no setting specified"))?;

        let discriminant = std::mem::discriminant(&setting);

        use pinnacle_api_defs::pinnacle::input::v0alpha1::set_libinput_setting_request::Setting;
        let apply_setting: Box<dyn Fn(&mut libinput::Device) + Send> = match setting {
            Setting::AccelProfile(profile) => {
                let profile = AccelProfile::try_from(profile).unwrap_or(AccelProfile::Unspecified);

                match profile {
                    AccelProfile::Unspecified => {
                        return Err(Status::invalid_argument("unspecified accel profile"));
                    }
                    AccelProfile::Flat => Box::new(|device| {
                        let _ = device.config_accel_set_profile(libinput::AccelProfile::Flat);
                    }),
                    AccelProfile::Adaptive => Box::new(|device| {
                        let _ = device.config_accel_set_profile(libinput::AccelProfile::Adaptive);
                    }),
                }
            }
            Setting::AccelSpeed(speed) => Box::new(move |device| {
                let _ = device.config_accel_set_speed(speed);
            }),
            Setting::CalibrationMatrix(matrix) => {
                let matrix = <[f32; 6]>::try_from(matrix.matrix).map_err(|vec| {
                    Status::invalid_argument(format!(
                        "matrix requires exactly 6 floats but {} were specified",
                        vec.len()
                    ))
                })?;

                Box::new(move |device| {
                    let _ = device.config_calibration_set_matrix(matrix);
                })
            }
            Setting::ClickMethod(method) => {
                let method = ClickMethod::try_from(method).unwrap_or(ClickMethod::Unspecified);

                match method {
                    ClickMethod::Unspecified => {
                        return Err(Status::invalid_argument("unspecified click method"))
                    }
                    ClickMethod::ButtonAreas => Box::new(|device| {
                        let _ = device.config_click_set_method(libinput::ClickMethod::ButtonAreas);
                    }),
                    ClickMethod::ClickFinger => Box::new(|device| {
                        let _ = device.config_click_set_method(libinput::ClickMethod::Clickfinger);
                    }),
                }
            }
            Setting::DisableWhileTyping(disable) => Box::new(move |device| {
                let _ = device.config_dwt_set_enabled(disable);
            }),
            Setting::LeftHanded(enable) => Box::new(move |device| {
                let _ = device.config_left_handed_set(enable);
            }),
            Setting::MiddleEmulation(enable) => Box::new(move |device| {
                let _ = device.config_middle_emulation_set_enabled(enable);
            }),
            Setting::RotationAngle(angle) => Box::new(move |device| {
                let _ = device.config_rotation_set_angle(angle % 360);
            }),
            Setting::ScrollButton(button) => Box::new(move |device| {
                let _ = device.config_scroll_set_button(button);
            }),
            Setting::ScrollButtonLock(enable) => Box::new(move |device| {
                let _ = device.config_scroll_set_button_lock(match enable {
                    true => libinput::ScrollButtonLockState::Enabled,
                    false => libinput::ScrollButtonLockState::Disabled,
                });
            }),
            Setting::ScrollMethod(method) => {
                let method = ScrollMethod::try_from(method).unwrap_or(ScrollMethod::Unspecified);

                match method {
                    ScrollMethod::Unspecified => {
                        return Err(Status::invalid_argument("unspecified scroll method"));
                    }
                    ScrollMethod::NoScroll => Box::new(|device| {
                        let _ = device.config_scroll_set_method(libinput::ScrollMethod::NoScroll);
                    }),
                    ScrollMethod::TwoFinger => Box::new(|device| {
                        let _ = device.config_scroll_set_method(libinput::ScrollMethod::TwoFinger);
                    }),
                    ScrollMethod::Edge => Box::new(|device| {
                        let _ = device.config_scroll_set_method(libinput::ScrollMethod::Edge);
                    }),
                    ScrollMethod::OnButtonDown => Box::new(|device| {
                        let _ =
                            device.config_scroll_set_method(libinput::ScrollMethod::OnButtonDown);
                    }),
                }
            }
            Setting::NaturalScroll(enable) => Box::new(move |device| {
                let _ = device.config_scroll_set_natural_scroll_enabled(enable);
            }),
            Setting::TapButtonMap(map) => {
                let map = TapButtonMap::try_from(map).unwrap_or(TapButtonMap::Unspecified);

                match map {
                    TapButtonMap::Unspecified => {
                        return Err(Status::invalid_argument("unspecified tap button map"));
                    }
                    TapButtonMap::LeftRightMiddle => Box::new(|device| {
                        let _ = device
                            .config_tap_set_button_map(libinput::TapButtonMap::LeftRightMiddle);
                    }),
                    TapButtonMap::LeftMiddleRight => Box::new(|device| {
                        let _ = device
                            .config_tap_set_button_map(libinput::TapButtonMap::LeftMiddleRight);
                    }),
                }
            }
            Setting::TapDrag(enable) => Box::new(move |device| {
                let _ = device.config_tap_set_drag_enabled(enable);
            }),
            Setting::TapDragLock(enable) => Box::new(move |device| {
                let _ = device.config_tap_set_drag_lock_enabled(enable);
            }),
            Setting::Tap(enable) => Box::new(move |device| {
                let _ = device.config_tap_set_enabled(enable);
            }),
        };

        run_unary_no_response(&self.sender, move |state| {
            for device in state.pinnacle.input_state.libinput_devices.iter_mut() {
                apply_setting(device);
            }

            state
                .pinnacle
                .input_state
                .libinput_settings
                .insert(discriminant, apply_setting);
        })
        .await
    }
}

pub struct ProcessService {
    sender: StateFnSender,
}

impl ProcessService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}

#[tonic::async_trait]
impl process_service_server::ProcessService for ProcessService {
    type SpawnStream = ResponseStream<SpawnResponse>;

    async fn spawn(
        &self,
        request: Request<SpawnRequest>,
    ) -> Result<Response<Self::SpawnStream>, Status> {
        debug!("ProcessService.spawn");
        let request = request.into_inner();

        let once = request.once();
        let has_callback = request.has_callback();
        let mut command = request.args.into_iter();
        let arg0 = command
            .next()
            .ok_or_else(|| Status::invalid_argument("no args specified"))?;

        run_server_streaming(&self.sender, move |state, sender| {
            if once {
                state
                    .pinnacle
                    .system_processes
                    .refresh_processes_specifics(ProcessRefreshKind::new());

                let compositor_pid = std::process::id();
                let already_running = state
                    .pinnacle
                    .system_processes
                    .processes_by_exact_name(&arg0)
                    .any(|proc| {
                        proc.parent()
                            .is_some_and(|parent_pid| parent_pid.as_u32() == compositor_pid)
                    });

                if already_running {
                    return;
                }
            }

            let Ok(mut child) = tokio::process::Command::new(OsString::from(arg0.clone()))
                .stdin(match has_callback {
                    true => Stdio::piped(),
                    false => Stdio::null(),
                })
                .stdout(match has_callback {
                    true => Stdio::piped(),
                    false => Stdio::null(),
                })
                .stderr(match has_callback {
                    true => Stdio::piped(),
                    false => Stdio::null(),
                })
                .args(command)
                .spawn()
            else {
                warn!("Tried to run {arg0}, but it doesn't exist",);
                return;
            };

            if !has_callback {
                return;
            }

            let stdout = child.stdout.take();
            let stderr = child.stderr.take();

            if let Some(stdout) = stdout {
                let sender = sender.clone();

                let mut reader = tokio::io::BufReader::new(stdout).lines();

                tokio::spawn(async move {
                    while let Ok(Some(line)) = reader.next_line().await {
                        let response: Result<_, Status> = Ok(SpawnResponse {
                            stdout: Some(line),
                            ..Default::default()
                        });

                        // TODO: handle error
                        match sender.send(response) {
                            Ok(_) => (),
                            Err(err) => {
                                error!(err = ?err);
                                break;
                            }
                        }
                    }
                });
            }

            if let Some(stderr) = stderr {
                let sender = sender.clone();

                let mut reader = tokio::io::BufReader::new(stderr).lines();

                tokio::spawn(async move {
                    while let Ok(Some(line)) = reader.next_line().await {
                        let response: Result<_, Status> = Ok(SpawnResponse {
                            stderr: Some(line),
                            ..Default::default()
                        });

                        // TODO: handle error
                        match sender.send(response) {
                            Ok(_) => (),
                            Err(err) => {
                                error!(err = ?err);
                                break;
                            }
                        }
                    }
                });
            }

            tokio::spawn(async move {
                match child.wait().await {
                    Ok(exit_status) => {
                        let response = Ok(SpawnResponse {
                            exit_code: exit_status.code(),
                            exit_message: Some(exit_status.to_string()),
                            ..Default::default()
                        });
                        // TODO: handle error
                        let _ = sender.send(response);
                    }
                    Err(err) => warn!("child wait() err: {err}"),
                }
            });
        })
    }

    async fn set_env(&self, request: Request<SetEnvRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let key = request
            .key
            .ok_or_else(|| Status::invalid_argument("no key specified"))?;
        let value = request
            .value
            .ok_or_else(|| Status::invalid_argument("no value specified"))?;

        if key.is_empty() {
            return Err(Status::invalid_argument("key was empty"));
        }

        if key.contains(['\0', '=']) {
            return Err(Status::invalid_argument("key contained NUL or ="));
        }

        if value.contains('\0') {
            return Err(Status::invalid_argument("value contained NUL"));
        }

        std::env::set_var(key, value);

        Ok(Response::new(()))
    }
}

pub struct TagService {
    sender: StateFnSender,
}

impl TagService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}

#[tonic::async_trait]
impl tag_service_server::TagService for TagService {
    async fn set_active(&self, request: Request<SetActiveRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let tag_id = TagId(
            request
                .tag_id
                .ok_or_else(|| Status::invalid_argument("no tag specified"))?,
        );

        let set_or_toggle = request.set_or_toggle();

        if set_or_toggle == SetOrToggle::Unspecified {
            return Err(Status::invalid_argument("unspecified set or toggle"));
        }

        run_unary_no_response(&self.sender, move |state| {
            let Some(tag) = tag_id.tag(state) else {
                return;
            };

            match set_or_toggle {
                SetOrToggle::Set => tag.set_active(true, state),
                SetOrToggle::Unset => tag.set_active(false, state),
                SetOrToggle::Toggle => tag.set_active(!tag.active(), state),
                SetOrToggle::Unspecified => unreachable!(),
            }

            let Some(output) = tag.output(state) else {
                return;
            };

            state.fixup_xwayland_window_layering();

            state.request_layout(&output);
            state.update_focus(&output);
            state.schedule_render(&output);
        })
        .await
    }

    async fn switch_to(&self, request: Request<SwitchToRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let tag_id = TagId(
            request
                .tag_id
                .ok_or_else(|| Status::invalid_argument("no tag specified"))?,
        );

        run_unary_no_response(&self.sender, move |state| {
            let Some(tag) = tag_id.tag(state) else { return };
            let Some(output) = tag.output(state) else { return };

            output.with_state_mut(|op_state| {
                for op_tag in op_state.tags.iter_mut() {
                    op_tag.set_active(false, state);
                }
                tag.set_active(true, state);
            });

            state.fixup_xwayland_window_layering();

            state.request_layout(&output);
            state.update_focus(&output);
            state.schedule_render(&output);
        })
        .await
    }

    async fn add(&self, request: Request<AddRequest>) -> Result<Response<AddResponse>, Status> {
        let request = request.into_inner();

        let output_name = OutputName(
            request
                .output_name
                .ok_or_else(|| Status::invalid_argument("no output specified"))?,
        );

        run_unary(&self.sender, move |state| {
            let new_tags = request
                .tag_names
                .into_iter()
                .map(Tag::new)
                .collect::<Vec<_>>();

            let tag_ids = new_tags
                .iter()
                .map(|tag| tag.id())
                .map(|id| id.0)
                .collect::<Vec<_>>();

            state
                .pinnacle
                .config
                .connector_saved_states
                .entry(output_name.clone())
                .or_default()
                .tags
                .extend(new_tags.clone());

            if let Some(output) = output_name.output(state) {
                output.with_state_mut(|state| {
                    state.tags.extend(new_tags.clone());
                    debug!("tags added, are now {:?}", state.tags);
                });
            }

            for tag in new_tags {
                for window in state.pinnacle.windows.iter() {
                    window.with_state_mut(|state| {
                        for win_tag in state.tags.iter_mut() {
                            if win_tag.id() == tag.id() {
                                *win_tag = tag.clone();
                            }
                        }
                    });
                }
            }

            AddResponse { tag_ids }
        })
        .await
    }

    // TODO: test
    async fn remove(&self, request: Request<RemoveRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let tag_ids = request.tag_ids.into_iter().map(TagId);

        run_unary_no_response(&self.sender, move |state| {
            let tags_to_remove = tag_ids.flat_map(|id| id.tag(state)).collect::<Vec<_>>();

            for output in state.pinnacle.space.outputs().cloned().collect::<Vec<_>>() {
                // TODO: seriously, convert state.tags into a hashset
                output.with_state_mut(|state| {
                    for tag_to_remove in tags_to_remove.iter() {
                        state.tags.retain(|tag| tag != tag_to_remove);
                    }
                });

                state.request_layout(&output);
                state.schedule_render(&output);
            }

            for conn_saved_state in state.pinnacle.config.connector_saved_states.values_mut() {
                for tag_to_remove in tags_to_remove.iter() {
                    conn_saved_state.tags.retain(|tag| tag != tag_to_remove);
                }
            }
        })
        .await
    }

    async fn get(
        &self,
        _request: Request<tag::v0alpha1::GetRequest>,
    ) -> Result<Response<tag::v0alpha1::GetResponse>, Status> {
        run_unary(&self.sender, move |state| {
            let tag_ids = state
                .pinnacle
                .space
                .outputs()
                .flat_map(|op| op.with_state(|state| state.tags.clone()))
                .map(|tag| tag.id())
                .map(|id| id.0)
                .collect::<Vec<_>>();

            tag::v0alpha1::GetResponse { tag_ids }
        })
        .await
    }

    async fn get_properties(
        &self,
        request: Request<tag::v0alpha1::GetPropertiesRequest>,
    ) -> Result<Response<tag::v0alpha1::GetPropertiesResponse>, Status> {
        let request = request.into_inner();

        let tag_id = TagId(
            request
                .tag_id
                .ok_or_else(|| Status::invalid_argument("no tag specified"))?,
        );

        run_unary(&self.sender, move |state| {
            let tag = tag_id.tag(state);

            let output_name = tag
                .as_ref()
                .and_then(|tag| tag.output(state))
                .map(|output| output.name());
            let active = tag.as_ref().map(|tag| tag.active());
            let name = tag.as_ref().map(|tag| tag.name());
            let window_ids = tag
                .as_ref()
                .map(|tag| {
                    state
                        .pinnacle
                        .windows
                        .iter()
                        .filter_map(|win| {
                            win.with_state(|win_state| {
                                win_state.tags.contains(tag).then_some(win_state.id.0)
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();

            tag::v0alpha1::GetPropertiesResponse {
                active,
                name,
                output_name,
                window_ids,
            }
        })
        .await
    }
}

pub struct OutputService {
    sender: StateFnSender,
}

impl OutputService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}

#[tonic::async_trait]
impl output_service_server::OutputService for OutputService {
    async fn set_location(
        &self,
        request: Request<SetLocationRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let output_name = OutputName(
            request
                .output_name
                .ok_or_else(|| Status::invalid_argument("no output specified"))?,
        );

        let x = request.x;
        let y = request.y;

        run_unary_no_response(&self.sender, move |state| {
            if let Some(saved_state) = state
                .pinnacle
                .config
                .connector_saved_states
                .get_mut(&output_name)
            {
                if let Some(x) = x {
                    saved_state.loc.x = x;
                }
                if let Some(y) = y {
                    saved_state.loc.y = y;
                }
            } else {
                state.pinnacle.config.connector_saved_states.insert(
                    output_name.clone(),
                    ConnectorSavedState {
                        loc: (x.unwrap_or_default(), y.unwrap_or_default()).into(),
                        ..Default::default()
                    },
                );
            }

            let Some(output) = output_name.output(state) else {
                return;
            };
            let mut loc = output.current_location();
            if let Some(x) = x {
                loc.x = x;
            }
            if let Some(y) = y {
                loc.y = y;
            }
            state.change_output_state(&output, None, None, None, Some(loc));
            debug!("Mapping output {} to {loc:?}", output.name());
            state.request_layout(&output);
        })
        .await
    }

    async fn set_mode(&self, request: Request<SetModeRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        run_unary_no_response(&self.sender, |state| {
            let Some(output) = request
                .output_name
                .clone()
                .map(OutputName)
                .and_then(|name| name.output(state))
            else {
                return;
            };

            // poor man's try v2
            let Some(mode) = Some(request).and_then(|request| {
                Some(smithay::output::Mode {
                    size: (request.pixel_width? as i32, request.pixel_height? as i32).into(),
                    refresh: request.refresh_rate_millihz? as i32,
                })
            }) else {
                return;
            };

            state.resize_output(&output, mode);
        })
        .await
    }

    async fn set_scale(&self, request: Request<SetScaleRequest>) -> Result<Response<()>, Status> {
        let SetScaleRequest {
            output_name: Some(output_name),
            absolute_or_relative: Some(absolute_or_relative),
        } = request.into_inner()
        else {
            return Err(Status::invalid_argument(
                "output_name or absolute_or_relative were null",
            ));
        };

        run_unary_no_response(&self.sender, move |state| {
            let Some(output) = OutputName(output_name).output(state) else {
                return;
            };

            let mut current_scale = output.current_scale().fractional_scale();

            match absolute_or_relative {
                AbsoluteOrRelative::Absolute(abs) => current_scale = abs as f64,
                AbsoluteOrRelative::Relative(rel) => current_scale += rel as f64,
            }

            current_scale = f64::max(current_scale, 0.25);

            state.change_output_state(
                &output,
                None,
                None,
                Some(Scale::Fractional(current_scale)),
                None,
            );
            state.request_layout(&output);
            state.schedule_render(&output);
        })
        .await
    }

    async fn set_transform(
        &self,
        request: Request<SetTransformRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let smithay_transform = match request.transform() {
            output::v0alpha1::Transform::Unspecified => {
                return Err(Status::invalid_argument("transform was unspecified"));
            }
            output::v0alpha1::Transform::Normal => smithay::utils::Transform::Normal,
            output::v0alpha1::Transform::Transform90 => smithay::utils::Transform::_90,
            output::v0alpha1::Transform::Transform180 => smithay::utils::Transform::_180,
            output::v0alpha1::Transform::Transform270 => smithay::utils::Transform::_270,
            output::v0alpha1::Transform::Flipped => smithay::utils::Transform::Flipped,
            output::v0alpha1::Transform::Flipped90 => smithay::utils::Transform::Flipped90,
            output::v0alpha1::Transform::Flipped180 => smithay::utils::Transform::Flipped180,
            output::v0alpha1::Transform::Flipped270 => smithay::utils::Transform::Flipped270,
        };

        let Some(output_name) = request.output_name else {
            return Err(Status::invalid_argument("output_name was null"));
        };

        run_unary_no_response(&self.sender, move |state| {
            let Some(output) = OutputName(output_name).output(state) else {
                return;
            };

            state.change_output_state(&output, None, Some(smithay_transform), None, None);
            state.request_layout(&output);
            state.schedule_render(&output);
        })
        .await
    }

    async fn get(
        &self,
        _request: Request<output::v0alpha1::GetRequest>,
    ) -> Result<Response<output::v0alpha1::GetResponse>, Status> {
        run_unary(&self.sender, move |state| {
            let output_names = state
                .pinnacle
                .space
                .outputs()
                .map(|output| output.name())
                .collect::<Vec<_>>();

            output::v0alpha1::GetResponse { output_names }
        })
        .await
    }

    async fn get_properties(
        &self,
        request: Request<output::v0alpha1::GetPropertiesRequest>,
    ) -> Result<Response<output::v0alpha1::GetPropertiesResponse>, Status> {
        let request = request.into_inner();

        let output_name = OutputName(
            request
                .output_name
                .ok_or_else(|| Status::invalid_argument("no output specified"))?,
        );

        let from_smithay_mode = |mode: smithay::output::Mode| -> output::v0alpha1::Mode {
            output::v0alpha1::Mode {
                pixel_width: Some(mode.size.w as u32),
                pixel_height: Some(mode.size.h as u32),
                refresh_rate_millihz: Some(mode.refresh as u32),
            }
        };

        run_unary(&self.sender, move |state| {
            let output = output_name.output(state);

            let logical_size = output
                .as_ref()
                .and_then(|output| state.pinnacle.space.output_geometry(output))
                .map(|geo| (geo.size.w, geo.size.h));

            let current_mode = output
                .as_ref()
                .and_then(|output| output.current_mode().map(from_smithay_mode));

            let preferred_mode = output
                .as_ref()
                .and_then(|output| output.preferred_mode().map(from_smithay_mode));

            let modes = output
                .as_ref()
                .map(|output| {
                    output
                        .modes()
                        .into_iter()
                        .map(from_smithay_mode)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let model = output
                .as_ref()
                .map(|output| output.physical_properties().model);

            let physical_width = output
                .as_ref()
                .map(|output| output.physical_properties().size.w as u32);

            let physical_height = output
                .as_ref()
                .map(|output| output.physical_properties().size.h as u32);

            let make = output
                .as_ref()
                .map(|output| output.physical_properties().make);

            let x = output.as_ref().map(|output| output.current_location().x);

            let y = output.as_ref().map(|output| output.current_location().y);

            let focused = state
                .focused_output()
                .and_then(|foc_op| output.as_ref().map(|op| op == foc_op));

            let tag_ids = output
                .as_ref()
                .map(|output| {
                    output.with_state(|state| {
                        state.tags.iter().map(|tag| tag.id().0).collect::<Vec<_>>()
                    })
                })
                .unwrap_or_default();

            let scale = output
                .as_ref()
                .map(|output| output.current_scale().fractional_scale() as f32);

            let transform = output.as_ref().map(|output| {
                (match output.current_transform() {
                    smithay::utils::Transform::Normal => output::v0alpha1::Transform::Normal,
                    smithay::utils::Transform::_90 => output::v0alpha1::Transform::Transform90,
                    smithay::utils::Transform::_180 => output::v0alpha1::Transform::Transform180,
                    smithay::utils::Transform::_270 => output::v0alpha1::Transform::Transform270,
                    smithay::utils::Transform::Flipped => output::v0alpha1::Transform::Flipped,
                    smithay::utils::Transform::Flipped90 => output::v0alpha1::Transform::Flipped90,
                    smithay::utils::Transform::Flipped180 => {
                        output::v0alpha1::Transform::Flipped180
                    }
                    smithay::utils::Transform::Flipped270 => {
                        output::v0alpha1::Transform::Flipped270
                    }
                }) as i32
            });

            let serial = output.as_ref().and_then(|output| {
                output.with_state(|state| state.serial.map(|serial| serial.get()))
            });

            output::v0alpha1::GetPropertiesResponse {
                make,
                model,
                x,
                y,
                logical_width: logical_size.map(|(w, _)| w as u32),
                logical_height: logical_size.map(|(_, h)| h as u32),
                current_mode,
                preferred_mode,
                modes,
                physical_width,
                physical_height,
                focused,
                tag_ids,
                scale,
                transform,
                serial,
            }
        })
        .await
    }
}

pub struct RenderService {
    sender: StateFnSender,
}

impl RenderService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}

#[tonic::async_trait]
impl render_service_server::RenderService for RenderService {
    async fn set_upscale_filter(
        &self,
        request: Request<SetUpscaleFilterRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();
        if let Filter::Unspecified = request.filter() {
            return Err(Status::invalid_argument("unspecified filter"));
        }

        let filter = match request.filter() {
            Filter::Bilinear => TextureFilter::Linear,
            Filter::NearestNeighbor => TextureFilter::Nearest,
            _ => unreachable!(),
        };

        run_unary_no_response(&self.sender, move |state| {
            state.backend.set_upscale_filter(filter);
            for output in state.pinnacle.space.outputs().cloned().collect::<Vec<_>>() {
                state.backend.reset_buffers(&output);
                state.schedule_render(&output);
            }
        })
        .await
    }

    async fn set_downscale_filter(
        &self,
        request: Request<SetDownscaleFilterRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();
        if let Filter::Unspecified = request.filter() {
            return Err(Status::invalid_argument("unspecified filter"));
        }

        let filter = match request.filter() {
            Filter::Bilinear => TextureFilter::Linear,
            Filter::NearestNeighbor => TextureFilter::Nearest,
            _ => unreachable!(),
        };

        run_unary_no_response(&self.sender, move |state| {
            state.backend.set_downscale_filter(filter);
            for output in state.pinnacle.space.outputs().cloned().collect::<Vec<_>>() {
                state.backend.reset_buffers(&output);
                state.schedule_render(&output);
            }
        })
        .await
    }
}
