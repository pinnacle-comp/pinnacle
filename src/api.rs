pub mod layout;
pub mod output;
pub mod pinnacle;
pub mod signal;
pub mod tag;
pub mod window;

use std::{ffi::OsString, pin::Pin, process::Stdio};

use pinnacle_api_defs::pinnacle::{
    input::v0alpha1::{
        input_service_server,
        set_libinput_setting_request::{AccelProfile, ClickMethod, ScrollMethod, TapButtonMap},
        set_mousebind_request::MouseEdge,
        KeybindDescription, KeybindDescriptionsRequest, KeybindDescriptionsResponse, Modifier,
        SetKeybindRequest, SetKeybindResponse, SetLibinputSettingRequest, SetMousebindRequest,
        SetMousebindResponse, SetRepeatRateRequest, SetXcursorRequest, SetXkbConfigRequest,
    },
    process::v0alpha1::{process_service_server, SetEnvRequest, SpawnRequest, SpawnResponse},
    render::v0alpha1::{
        render_service_server, Filter, SetDownscaleFilterRequest, SetUpscaleFilterRequest,
    },
};
use smithay::{
    backend::renderer::TextureFilter,
    input::keyboard::XkbConfig,
    reexports::{calloop, input as libinput},
};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};
use tokio::{
    io::AsyncBufReadExt,
    sync::mpsc::{unbounded_channel, UnboundedSender},
    task::JoinHandle,
};
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};
use tracing::{debug, error, warn};

use crate::{
    backend::BackendData,
    input::{KeybindData, ModifierMask},
    state::State,
    util::restore_nofile_rlimit,
};

type ResponseStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;
pub type StateFnSender = calloop::channel::Sender<Box<dyn FnOnce(&mut State) + Send>>;
pub type TonicResult<T> = Result<Response<T>, Status>;

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
    F1: Fn(&mut State, I) + Clone + Send + 'static,
    F2: FnOnce(&mut State, UnboundedSender<Result<O, Status>>, JoinHandle<()>) + Send + 'static,
    I: Send + 'static,
    O: Send + 'static,
{
    let (sender, receiver) = unbounded_channel::<Result<O, Status>>();

    let fn_sender_clone = fn_sender.clone();

    let with_in_stream = async move {
        while let Some(request) = in_stream.next().await {
            match request {
                Ok(request) => {
                    let on_client_request = on_client_request.clone();
                    // TODO: handle error
                    let _ = fn_sender_clone.send(Box::new(move |state: &mut State| {
                        on_client_request(state, request);
                    }));
                }
                Err(err) => {
                    debug!("bidirectional stream error: {err}");
                    break;
                }
            }
        }
    };

    let join_handle = tokio::spawn(with_in_stream);
    // let join_handle = tokio::spawn(async {});

    let with_out_stream_and_in_stream_join_handle = Box::new(|state: &mut State| {
        with_out_stream_and_in_stream_join_handle(state, sender, join_handle);
    });

    fn_sender
        .send(with_out_stream_and_in_stream_join_handle)
        .map_err(|_| Status::internal("failed to execute request"))?;

    let receiver_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);
    Ok(Response::new(Box::pin(receiver_stream)))
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

        let group = request.group;
        let description = request.description;

        run_server_streaming(&self.sender, move |state, sender| {
            let keybind_data = KeybindData {
                sender,
                group,
                description,
            };

            state
                .pinnacle
                .input_state
                .keybinds
                .insert((modifiers, keysym), keybind_data);
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

    async fn keybind_descriptions(
        &self,
        _request: Request<KeybindDescriptionsRequest>,
    ) -> Result<Response<KeybindDescriptionsResponse>, Status> {
        run_unary(&self.sender, |state| {
            let descriptions =
                state
                    .pinnacle
                    .input_state
                    .keybinds
                    .iter()
                    .map(|((mods, key), data)| {
                        let mut modifiers = Vec::<i32>::new();
                        if mods.contains(ModifierMask::CTRL) {
                            modifiers.push(Modifier::Ctrl as i32);
                        }
                        if mods.contains(ModifierMask::ALT) {
                            modifiers.push(Modifier::Alt as i32);
                        }
                        if mods.contains(ModifierMask::SUPER) {
                            modifiers.push(Modifier::Super as i32);
                        }
                        if mods.contains(ModifierMask::SHIFT) {
                            modifiers.push(Modifier::Shift as i32);
                        }
                        KeybindDescription {
                            modifiers,
                            raw_code: Some(key.raw()),
                            xkb_name: Some(xkbcommon::xkb::keysym_get_name(*key)),
                            group: data.group.clone(),
                            description: data.description.clone(),
                        }
                    });

            KeybindDescriptionsResponse {
                descriptions: descriptions.collect(),
            }
        })
        .await
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

    async fn set_xcursor(
        &self,
        request: Request<SetXcursorRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let theme = request.theme;
        let size = request.size;

        run_unary_no_response(&self.sender, move |state| {
            if let Some(theme) = theme {
                state.pinnacle.cursor_state.set_theme(&theme);
            }

            if let Some(size) = size {
                state.pinnacle.cursor_state.set_size(size);
            }

            if let Some(output) = state.pinnacle.focused_output().cloned() {
                state.schedule_render(&output)
            }
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
                state.pinnacle.system_processes.refresh_processes_specifics(
                    ProcessesToUpdate::All,
                    true,
                    ProcessRefreshKind::nothing(),
                );

                let compositor_pid = std::process::id();
                let already_running = state
                    .pinnacle
                    .system_processes
                    .processes_by_exact_name(arg0.as_ref())
                    .any(|proc| {
                        proc.parent()
                            .is_some_and(|parent_pid| parent_pid.as_u32() == compositor_pid)
                    });

                if already_running {
                    return;
                }
            }

            let mut cmd = tokio::process::Command::new(OsString::from(arg0.clone()));

            cmd.stdin(match has_callback {
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
            .args(command);

            unsafe {
                cmd.pre_exec(|| {
                    restore_nofile_rlimit();
                    Ok(())
                });
            }

            let Ok(mut child) = cmd.spawn() else {
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
            for output in state.pinnacle.outputs.keys().cloned().collect::<Vec<_>>() {
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
            for output in state.pinnacle.outputs.keys().cloned().collect::<Vec<_>>() {
                state.backend.reset_buffers(&output);
                state.schedule_render(&output);
            }
        })
        .await
    }
}
