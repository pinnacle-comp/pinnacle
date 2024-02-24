pub mod signal;

use std::{ffi::OsString, num::NonZeroU32, pin::Pin, process::Stdio};

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
        v0alpha1::{output_service_server, SetLocationRequest},
    },
    process::v0alpha1::{process_service_server, SetEnvRequest, SpawnRequest, SpawnResponse},
    tag::{
        self,
        v0alpha1::{
            tag_service_server, AddRequest, AddResponse, RemoveRequest, SetActiveRequest,
            SetLayoutRequest, SwitchToRequest,
        },
    },
    v0alpha1::{pinnacle_service_server, Geometry, QuitRequest, SetOrToggle},
    window::{
        self,
        v0alpha1::{
            window_service_server, AddWindowRuleRequest, CloseRequest, FullscreenOrMaximized,
            MoveGrabRequest, MoveToTagRequest, ResizeGrabRequest, SetFloatingRequest,
            SetFocusedRequest, SetFullscreenRequest, SetGeometryRequest, SetMaximizedRequest,
            SetTagRequest, WindowRule, WindowRuleCondition,
        },
    },
};
use smithay::{
    desktop::space::SpaceElement,
    input::keyboard::XkbConfig,
    reexports::{calloop, input as libinput, wayland_protocols::xdg::shell::server},
    utils::{Point, Rectangle, SERIAL_COUNTER},
    wayland::{compositor, shell::xdg::XdgToplevelSurfaceData},
};
use sysinfo::ProcessRefreshKind;
use tokio::{
    io::AsyncBufReadExt,
    sync::mpsc::{unbounded_channel, UnboundedSender},
    task::JoinHandle,
};
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};

use crate::{
    config::ConnectorSavedState,
    focus::FocusTarget,
    input::ModifierMask,
    output::OutputName,
    state::{State, WithState},
    tag::{Tag, TagId},
    window::{window_state::WindowId, WindowElement},
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
            panic!("failed to send result to config");
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

fn run_bidirectional_streaming<F1, F2, I, O>(
    fn_sender: StateFnSender,
    mut in_stream: Streaming<I>,
    with_client_request: F1,
    with_out_stream: F2,
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
            let with_client_request = with_client_request.clone();
            // TODO: handle error
            let _ = fn_sender_clone.send(Box::new(move |state: &mut State| {
                with_client_request(state, request);
            }));
        }
    };

    let join_handle = tokio::spawn(with_in_stream);

    let with_out_stream = Box::new(|state: &mut State| {
        with_out_stream(state, sender, join_handle);
    });

    fn_sender
        .send(with_out_stream)
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
    async fn quit(&self, _request: Request<QuitRequest>) -> Result<Response<()>, Status> {
        tracing::trace!("PinnacleService.quit");

        run_unary_no_response(&self.sender, |state| {
            state.shutdown();
        })
        .await
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
                tracing::info!("set keybind: {:?}, raw {}", modifiers, num);
                xkbcommon::xkb::Keysym::new(num)
            }
            Key::XkbName(s) => {
                if s.chars().count() == 1 {
                    let Some(ch) = s.chars().next() else { unreachable!() };
                    let keysym = xkbcommon::xkb::Keysym::from_char(ch);
                    tracing::info!("set keybind: {:?}, {:?}", modifiers, keysym);
                    keysym
                } else {
                    let keysym =
                        xkbcommon::xkb::keysym_from_name(&s, xkbcommon::xkb::KEYSYM_NO_FLAGS);
                    tracing::info!("set keybind: {:?}, {:?}", modifiers, keysym);
                    keysym
                }
            }
        };

        run_server_streaming(&self.sender, move |state, sender| {
            state
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

        tracing::debug!(request = ?request);

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
            if let Some(kb) = state.seat.get_keyboard() {
                if let Err(err) = kb.set_xkb_config(state, new_config) {
                    tracing::error!("Failed to set xkbconfig: {err}");
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
            if let Some(kb) = state.seat.get_keyboard() {
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
            for device in state.input_state.libinput_devices.iter_mut() {
                apply_setting(device);
            }

            state
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
        tracing::debug!("ProcessService.spawn");
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
                    .system_processes
                    .refresh_processes_specifics(ProcessRefreshKind::new());

                let compositor_pid = std::process::id();
                let already_running =
                    state
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
                .envs(state.xdisplay.map(|xdisp| ("DISPLAY", format!(":{xdisp}"))))
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
                tracing::warn!("Tried to run {arg0}, but it doesn't exist",);
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
                                tracing::error!(err = ?err);
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
                                tracing::error!(err = ?err);
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
                    Err(err) => tracing::warn!("child wait() err: {err}"),
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

        let tag_id = TagId::Some(
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
                SetOrToggle::Set => tag.set_active(true),
                SetOrToggle::Unset => tag.set_active(false),
                SetOrToggle::Toggle => tag.set_active(!tag.active()),
                SetOrToggle::Unspecified => unreachable!(),
            }

            let Some(output) = tag.output(state) else {
                return;
            };

            state.update_windows(&output);
            state.update_focus(&output);
            state.schedule_render(&output);
        })
        .await
    }

    async fn switch_to(&self, request: Request<SwitchToRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let tag_id = TagId::Some(
            request
                .tag_id
                .ok_or_else(|| Status::invalid_argument("no tag specified"))?,
        );

        run_unary_no_response(&self.sender, move |state| {
            let Some(tag) = tag_id.tag(state) else { return };
            let Some(output) = tag.output(state) else { return };

            output.with_state(|state| {
                for op_tag in state.tags.iter_mut() {
                    op_tag.set_active(false);
                }
                tag.set_active(true);
            });

            state.update_windows(&output);
            state.update_focus(&output);
            state.schedule_render(&output);

            state.signal_state.layout.signal(|buffer| {
                buffer.push_back(
                    pinnacle_api_defs::pinnacle::signal::v0alpha1::LayoutResponse {
                        window_ids: vec![1, 2, 3],
                        tag_id: Some(1),
                    },
                );
            });
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
                .map(|id| match id {
                    TagId::None => unreachable!(),
                    TagId::Some(id) => id,
                })
                .collect::<Vec<_>>();

            if let Some(saved_state) = state.config.connector_saved_states.get_mut(&output_name) {
                let mut tags = saved_state.tags.clone();
                tags.extend(new_tags.clone());
                saved_state.tags = tags;
            } else {
                state.config.connector_saved_states.insert(
                    output_name.clone(),
                    crate::config::ConnectorSavedState {
                        tags: new_tags.clone(),
                        ..Default::default()
                    },
                );
            }

            if let Some(output) = state
                .space
                .outputs()
                .find(|output| output.name() == output_name.0)
            {
                output.with_state(|state| {
                    state.tags.extend(new_tags.clone());
                    tracing::debug!("tags added, are now {:?}", state.tags);
                });
            }

            for tag in new_tags {
                for window in state.windows.iter() {
                    window.with_state(|state| {
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

        let tag_ids = request.tag_ids.into_iter().map(TagId::Some);

        run_unary_no_response(&self.sender, move |state| {
            let tags_to_remove = tag_ids.flat_map(|id| id.tag(state)).collect::<Vec<_>>();

            for output in state.space.outputs().cloned().collect::<Vec<_>>() {
                // TODO: seriously, convert state.tags into a hashset
                output.with_state(|state| {
                    for tag_to_remove in tags_to_remove.iter() {
                        state.tags.retain(|tag| tag != tag_to_remove);
                    }
                });

                state.update_windows(&output);
                state.schedule_render(&output);
            }

            for conn_saved_state in state.config.connector_saved_states.values_mut() {
                for tag_to_remove in tags_to_remove.iter() {
                    conn_saved_state.tags.retain(|tag| tag != tag_to_remove);
                }
            }
        })
        .await
    }

    async fn set_layout(&self, request: Request<SetLayoutRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let tag_id = TagId::Some(
            request
                .tag_id
                .ok_or_else(|| Status::invalid_argument("no tag specified"))?,
        );

        use pinnacle_api_defs::pinnacle::tag::v0alpha1::set_layout_request::Layout;

        // TODO: from impl
        let layout = match request.layout() {
            Layout::Unspecified => return Err(Status::invalid_argument("unspecified layout")),
            Layout::MasterStack => crate::layout::Layout::MasterStack,
            Layout::Dwindle => crate::layout::Layout::Dwindle,
            Layout::Spiral => crate::layout::Layout::Spiral,
            Layout::CornerTopLeft => crate::layout::Layout::CornerTopLeft,
            Layout::CornerTopRight => crate::layout::Layout::CornerTopRight,
            Layout::CornerBottomLeft => crate::layout::Layout::CornerBottomLeft,
            Layout::CornerBottomRight => crate::layout::Layout::CornerBottomRight,
        };

        run_unary_no_response(&self.sender, move |state| {
            let Some(tag) = tag_id.tag(state) else { return };

            tag.set_layout(layout);

            let Some(output) = tag.output(state) else { return };

            state.update_windows(&output);
            state.schedule_render(&output);
        })
        .await
    }

    async fn get(
        &self,
        _request: Request<tag::v0alpha1::GetRequest>,
    ) -> Result<Response<tag::v0alpha1::GetResponse>, Status> {
        run_unary(&self.sender, move |state| {
            let tag_ids = state
                .space
                .outputs()
                .flat_map(|op| op.with_state(|state| state.tags.clone()))
                .map(|tag| tag.id())
                .map(|id| match id {
                    TagId::None => unreachable!(),
                    TagId::Some(id) => id,
                })
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

        let tag_id = TagId::Some(
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

            tag::v0alpha1::GetPropertiesResponse {
                active,
                name,
                output_name,
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
            if let Some(saved_state) = state.config.connector_saved_states.get_mut(&output_name) {
                if let Some(x) = x {
                    saved_state.loc.x = x;
                }
                if let Some(y) = y {
                    saved_state.loc.y = y;
                }
            } else {
                state.config.connector_saved_states.insert(
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
            output.change_current_state(None, None, None, Some(loc));
            state.space.map_output(&output, loc);
            tracing::debug!("Mapping output {} to {loc:?}", output.name());
            state.update_windows(&output);
        })
        .await
    }

    async fn get(
        &self,
        _request: Request<output::v0alpha1::GetRequest>,
    ) -> Result<Response<output::v0alpha1::GetResponse>, Status> {
        run_unary(&self.sender, move |state| {
            let output_names = state
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

        run_unary(&self.sender, move |state| {
            let output = output_name.output(state);

            let pixel_width = output
                .as_ref()
                .and_then(|output| output.current_mode().map(|mode| mode.size.w as u32));

            let pixel_height = output
                .as_ref()
                .and_then(|output| output.current_mode().map(|mode| mode.size.h as u32));

            let refresh_rate = output
                .as_ref()
                .and_then(|output| output.current_mode().map(|mode| mode.refresh as u32));

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
                .output_focus_stack
                .current_focus()
                .and_then(|foc_op| output.as_ref().map(|op| op == foc_op));

            let tag_ids = output
                .as_ref()
                .map(|output| {
                    output.with_state(|state| {
                        state
                            .tags
                            .iter()
                            .map(|tag| match tag.id() {
                                TagId::None => unreachable!(),
                                TagId::Some(id) => id,
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .unwrap_or_default();

            output::v0alpha1::GetPropertiesResponse {
                make,
                model,
                x,
                y,
                pixel_width,
                pixel_height,
                refresh_rate,
                physical_width,
                physical_height,
                focused,
                tag_ids,
            }
        })
        .await
    }
}

pub struct WindowService {
    sender: StateFnSender,
}

impl WindowService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}

#[tonic::async_trait]
impl window_service_server::WindowService for WindowService {
    async fn close(&self, request: Request<CloseRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let window_id = WindowId(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        run_unary_no_response(&self.sender, move |state| {
            let Some(window) = window_id.window(state) else { return };

            match window {
                WindowElement::Wayland(window) => window.toplevel().send_close(),
                WindowElement::X11(surface) => surface.close().expect("failed to close x11 win"),
                WindowElement::X11OverrideRedirect(_) => {
                    tracing::warn!("tried to close override redirect window");
                }
                _ => unreachable!(),
            }
        })
        .await
    }

    async fn set_geometry(
        &self,
        request: Request<SetGeometryRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        tracing::info!(request = ?request);

        let window_id = WindowId(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        let geometry = request.geometry.unwrap_or_default();
        let x = geometry.x;
        let y = geometry.y;
        let width = geometry.width;
        let height = geometry.height;

        run_unary_no_response(&self.sender, move |state| {
            let Some(window) = window_id.window(state) else { return };

            // TODO: with no x or y, defaults unmapped windows to 0, 0
            let mut window_loc = state
                .space
                .element_location(&window)
                .unwrap_or((x.unwrap_or_default(), y.unwrap_or_default()).into());
            window_loc.x = x.unwrap_or(window_loc.x);
            window_loc.y = y.unwrap_or(window_loc.y);

            let mut window_size = window.geometry().size;
            window_size.w = width.unwrap_or(window_size.w);
            window_size.h = height.unwrap_or(window_size.h);

            let rect = Rectangle::from_loc_and_size(window_loc, window_size);
            // window.change_geometry(rect);
            window.with_state(|state| {
                use crate::window::window_state::FloatingOrTiled;
                state.floating_or_tiled = match state.floating_or_tiled {
                    FloatingOrTiled::Floating(_) => FloatingOrTiled::Floating(rect),
                    FloatingOrTiled::Tiled(_) => FloatingOrTiled::Tiled(Some(rect)),
                }
            });

            for output in state.space.outputs_for_element(&window) {
                state.update_windows(&output);
                state.schedule_render(&output);
            }
        })
        .await
    }

    async fn set_fullscreen(
        &self,
        request: Request<SetFullscreenRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let window_id = WindowId(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        let set_or_toggle = request.set_or_toggle();

        if set_or_toggle == SetOrToggle::Unspecified {
            return Err(Status::invalid_argument("unspecified set or toggle"));
        }

        run_unary_no_response(&self.sender, move |state| {
            let Some(window) = window_id.window(state) else {
                return;
            };

            match set_or_toggle {
                SetOrToggle::Set => {
                    if !window.with_state(|state| state.fullscreen_or_maximized.is_fullscreen()) {
                        window.toggle_fullscreen();
                    }
                }
                SetOrToggle::Unset => {
                    if window.with_state(|state| state.fullscreen_or_maximized.is_fullscreen()) {
                        window.toggle_fullscreen();
                    }
                }
                SetOrToggle::Toggle => window.toggle_fullscreen(),
                SetOrToggle::Unspecified => unreachable!(),
            }

            let Some(output) = window.output(state) else {
                return;
            };

            state.update_windows(&output);
            state.schedule_render(&output);
        })
        .await
    }

    async fn set_maximized(
        &self,
        request: Request<SetMaximizedRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let window_id = WindowId(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        let set_or_toggle = request.set_or_toggle();

        if set_or_toggle == SetOrToggle::Unspecified {
            return Err(Status::invalid_argument("unspecified set or toggle"));
        }

        run_unary_no_response(&self.sender, move |state| {
            let Some(window) = window_id.window(state) else {
                return;
            };

            match set_or_toggle {
                SetOrToggle::Set => {
                    if !window.with_state(|state| state.fullscreen_or_maximized.is_maximized()) {
                        window.toggle_maximized();
                    }
                }
                SetOrToggle::Unset => {
                    if window.with_state(|state| state.fullscreen_or_maximized.is_maximized()) {
                        window.toggle_maximized();
                    }
                }
                SetOrToggle::Toggle => window.toggle_maximized(),
                SetOrToggle::Unspecified => unreachable!(),
            }

            let Some(output) = window.output(state) else {
                return;
            };

            state.update_windows(&output);
            state.schedule_render(&output);
        })
        .await
    }

    async fn set_floating(
        &self,
        request: Request<SetFloatingRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let window_id = WindowId(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        let set_or_toggle = request.set_or_toggle();

        if set_or_toggle == SetOrToggle::Unspecified {
            return Err(Status::invalid_argument("unspecified set or toggle"));
        }

        run_unary_no_response(&self.sender, move |state| {
            let Some(window) = window_id.window(state) else {
                return;
            };

            match set_or_toggle {
                SetOrToggle::Set => {
                    if !window.with_state(|state| state.floating_or_tiled.is_floating()) {
                        window.toggle_floating();
                    }
                }
                SetOrToggle::Unset => {
                    if window.with_state(|state| state.floating_or_tiled.is_floating()) {
                        window.toggle_floating();
                    }
                }
                SetOrToggle::Toggle => window.toggle_floating(),
                SetOrToggle::Unspecified => unreachable!(),
            }

            let Some(output) = window.output(state) else {
                return;
            };

            state.update_windows(&output);
            state.schedule_render(&output);
        })
        .await
    }

    async fn set_focused(
        &self,
        request: Request<SetFocusedRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let window_id = WindowId(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        let set_or_toggle = request.set_or_toggle();

        if set_or_toggle == SetOrToggle::Unspecified {
            return Err(Status::invalid_argument("unspecified set or toggle"));
        }

        run_unary_no_response(&self.sender, move |state| {
            let Some(window) = window_id.window(state) else {
                return;
            };

            let Some(output) = window.output(state) else {
                return;
            };

            //     if !matches!(
            //         &focus,
            //         FocusTarget::Window(WindowElement::X11OverrideRedirect(_))
            //     ) {
            //         keyboard.set_focus(self, Some(focus.clone()), serial);
            //     }
            //
            //     self.space.elements().for_each(|window| {
            //         if let WindowElement::Wayland(window) = window {
            //             window.toplevel().send_configure();
            //         }
            //     });
            // } else {
            //     self.space.elements().for_each(|window| {
            //         window.set_activate(false);
            //         if let WindowElement::Wayland(window) = window {
            //             window.toplevel().send_configure();
            //         }
            //     });
            //     keyboard.set_focus(self, None, serial);
            // }

            for win in state.space.elements() {
                win.set_activate(false);
            }

            match set_or_toggle {
                SetOrToggle::Set => {
                    window.set_activate(true);
                    output.with_state(|state| state.focus_stack.set_focus(window.clone()));
                    state.output_focus_stack.set_focus(output.clone());
                    if let Some(keyboard) = state.seat.get_keyboard() {
                        keyboard.set_focus(
                            state,
                            Some(FocusTarget::Window(window)),
                            SERIAL_COUNTER.next_serial(),
                        );
                    }
                }
                SetOrToggle::Unset => {
                    if output.with_state(|state| state.focus_stack.current_focus() == Some(&window))
                    {
                        output.with_state(|state| state.focus_stack.unset_focus());
                        if let Some(keyboard) = state.seat.get_keyboard() {
                            keyboard.set_focus(state, None, SERIAL_COUNTER.next_serial());
                        }
                    }
                }
                SetOrToggle::Toggle => {
                    if output.with_state(|state| state.focus_stack.current_focus() == Some(&window))
                    {
                        output.with_state(|state| state.focus_stack.unset_focus());
                        if let Some(keyboard) = state.seat.get_keyboard() {
                            keyboard.set_focus(state, None, SERIAL_COUNTER.next_serial());
                        }
                    } else {
                        window.set_activate(true);
                        output.with_state(|state| state.focus_stack.set_focus(window.clone()));
                        state.output_focus_stack.set_focus(output.clone());
                        if let Some(keyboard) = state.seat.get_keyboard() {
                            keyboard.set_focus(
                                state,
                                Some(FocusTarget::Window(window)),
                                SERIAL_COUNTER.next_serial(),
                            );
                        }
                    }
                }
                SetOrToggle::Unspecified => unreachable!(),
            }

            for window in state.space.elements() {
                if let WindowElement::Wayland(window) = window {
                    window.toplevel().send_configure();
                }
            }

            state.update_windows(&output);
            state.schedule_render(&output);
        })
        .await
    }

    async fn move_to_tag(
        &self,
        request: Request<MoveToTagRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let window_id = WindowId(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        let tag_id = TagId::Some(
            request
                .tag_id
                .ok_or_else(|| Status::invalid_argument("no tag specified"))?,
        );

        run_unary_no_response(&self.sender, move |state| {
            let Some(window) = window_id.window(state) else { return };
            let Some(tag) = tag_id.tag(state) else { return };
            window.with_state(|state| {
                state.tags = vec![tag.clone()];
            });
            let Some(output) = tag.output(state) else { return };
            state.update_windows(&output);
            state.schedule_render(&output);
        })
        .await
    }

    async fn set_tag(&self, request: Request<SetTagRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let window_id = WindowId(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        let tag_id = TagId::Some(
            request
                .tag_id
                .ok_or_else(|| Status::invalid_argument("no tag specified"))?,
        );

        let set_or_toggle = request.set_or_toggle();

        if set_or_toggle == SetOrToggle::Unspecified {
            return Err(Status::invalid_argument("unspecified set or toggle"));
        }

        run_unary_no_response(&self.sender, move |state| {
            let Some(window) = window_id.window(state) else { return };
            let Some(tag) = tag_id.tag(state) else { return };

            // TODO: turn state.tags into a hashset
            match set_or_toggle {
                SetOrToggle::Set => window.with_state(|state| {
                    state.tags.retain(|tg| tg != &tag);
                    state.tags.push(tag.clone());
                }),
                SetOrToggle::Unset => window.with_state(|state| {
                    state.tags.retain(|tg| tg != &tag);
                }),
                SetOrToggle::Toggle => window.with_state(|state| {
                    if !state.tags.contains(&tag) {
                        state.tags.push(tag.clone());
                    } else {
                        state.tags.retain(|tg| tg != &tag);
                    }
                }),
                SetOrToggle::Unspecified => unreachable!(),
            }

            let Some(output) = tag.output(state) else { return };
            state.update_windows(&output);
            state.schedule_render(&output);
        })
        .await
    }

    async fn move_grab(&self, request: Request<MoveGrabRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let button = request
            .button
            .ok_or_else(|| Status::invalid_argument("no button specified"))?;

        run_unary_no_response(&self.sender, move |state| {
            let Some((FocusTarget::Window(window), _)) =
                state.focus_target_under(state.pointer_location)
            else {
                return;
            };
            let Some(wl_surf) = window.wl_surface() else { return };
            let seat = state.seat.clone();

            crate::grab::move_grab::move_request_server(
                state,
                &wl_surf,
                &seat,
                SERIAL_COUNTER.next_serial(),
                button,
            );
        })
        .await
    }

    async fn resize_grab(
        &self,
        request: Request<ResizeGrabRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let button = request
            .button
            .ok_or_else(|| Status::invalid_argument("no button specified"))?;

        run_unary_no_response(&self.sender, move |state| {
            let pointer_loc = state.pointer_location;
            let Some((FocusTarget::Window(window), window_loc)) =
                state.focus_target_under(pointer_loc)
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
                    server::xdg_toplevel::ResizeEdge::TopLeft
                }
                Point { x, y, .. }
                    if (half_width..=full_width).contains(&x)
                        && (window_y..=half_height).contains(&y) =>
                {
                    server::xdg_toplevel::ResizeEdge::TopRight
                }
                Point { x, y, .. }
                    if (window_x..=half_width).contains(&x)
                        && (half_height..=full_height).contains(&y) =>
                {
                    server::xdg_toplevel::ResizeEdge::BottomLeft
                }
                Point { x, y, .. }
                    if (half_width..=full_width).contains(&x)
                        && (half_height..=full_height).contains(&y) =>
                {
                    server::xdg_toplevel::ResizeEdge::BottomRight
                }
                _ => server::xdg_toplevel::ResizeEdge::None,
            };

            crate::grab::resize_grab::resize_request_server(
                state,
                &wl_surf,
                &state.seat.clone(),
                SERIAL_COUNTER.next_serial(),
                edges.into(),
                button,
            );
        })
        .await
    }

    async fn get(
        &self,
        _request: Request<window::v0alpha1::GetRequest>,
    ) -> Result<Response<window::v0alpha1::GetResponse>, Status> {
        run_unary(&self.sender, move |state| {
            let window_ids = state
                .windows
                .iter()
                .map(|win| win.with_state(|state| state.id.0))
                .collect::<Vec<_>>();

            window::v0alpha1::GetResponse { window_ids }
        })
        .await
    }

    async fn get_properties(
        &self,
        request: Request<window::v0alpha1::GetPropertiesRequest>,
    ) -> Result<Response<window::v0alpha1::GetPropertiesResponse>, Status> {
        let request = request.into_inner();

        let window_id = WindowId(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        run_unary(&self.sender, move |state| {
            let window = window_id.window(state);

            let width = window.as_ref().map(|win| win.geometry().size.w);

            let height = window.as_ref().map(|win| win.geometry().size.h);

            let x = window
                .as_ref()
                .and_then(|win| state.space.element_location(win))
                .map(|loc| loc.x);

            let y = window
                .as_ref()
                .and_then(|win| state.space.element_location(win))
                .map(|loc| loc.y);

            let geometry = if width.is_none() && height.is_none() && x.is_none() && y.is_none() {
                None
            } else {
                Some(Geometry {
                    x,
                    y,
                    width,
                    height,
                })
            };

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
                _ => unreachable!(),
            });

            let focused = window.as_ref().and_then(|win| {
                let output = win.output(state)?;
                state.focused_window(&output).map(|foc_win| win == &foc_win)
            });

            let floating = window
                .as_ref()
                .map(|win| win.with_state(|state| state.floating_or_tiled.is_floating()));

            let fullscreen_or_maximized = window
                .as_ref()
                .map(|win| win.with_state(|state| state.fullscreen_or_maximized))
                .map(|fs_or_max| match fs_or_max {
                    // TODO: from impl
                    crate::window::window_state::FullscreenOrMaximized::Neither => {
                        FullscreenOrMaximized::Neither
                    }
                    crate::window::window_state::FullscreenOrMaximized::Fullscreen => {
                        FullscreenOrMaximized::Fullscreen
                    }
                    crate::window::window_state::FullscreenOrMaximized::Maximized => {
                        FullscreenOrMaximized::Maximized
                    }
                } as i32);

            let tag_ids = window
                .as_ref()
                .map(|win| {
                    win.with_state(|state| {
                        state
                            .tags
                            .iter()
                            .map(|tag| match tag.id() {
                                TagId::Some(id) => id,
                                TagId::None => unreachable!(),
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .unwrap_or_default();

            window::v0alpha1::GetPropertiesResponse {
                geometry,
                class,
                title,
                focused,
                floating,
                fullscreen_or_maximized,
                tag_ids,
            }
        })
        .await
    }

    async fn add_window_rule(
        &self,
        request: Request<AddWindowRuleRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let cond = request
            .cond
            .ok_or_else(|| Status::invalid_argument("no condition specified"))?
            .into();

        let rule = request
            .rule
            .ok_or_else(|| Status::invalid_argument("no rule specified"))?
            .into();

        run_unary_no_response(&self.sender, move |state| {
            state.config.window_rules.push((cond, rule));
        })
        .await
    }
}

impl From<WindowRuleCondition> for crate::window::rules::WindowRuleCondition {
    fn from(cond: WindowRuleCondition) -> Self {
        let cond_any = match cond.any.is_empty() {
            true => None,
            false => Some(
                cond.any
                    .into_iter()
                    .map(crate::window::rules::WindowRuleCondition::from)
                    .collect::<Vec<_>>(),
            ),
        };

        let cond_all = match cond.all.is_empty() {
            true => None,
            false => Some(
                cond.all
                    .into_iter()
                    .map(crate::window::rules::WindowRuleCondition::from)
                    .collect::<Vec<_>>(),
            ),
        };

        let class = match cond.classes.is_empty() {
            true => None,
            false => Some(cond.classes),
        };

        let title = match cond.titles.is_empty() {
            true => None,
            false => Some(cond.titles),
        };

        let tag = match cond.tags.is_empty() {
            true => None,
            false => Some(cond.tags.into_iter().map(TagId::Some).collect::<Vec<_>>()),
        };

        crate::window::rules::WindowRuleCondition {
            cond_any,
            cond_all,
            class,
            title,
            tag,
        }
    }
}

impl From<WindowRule> for crate::window::rules::WindowRule {
    fn from(rule: WindowRule) -> Self {
        let fullscreen_or_maximized = match rule.fullscreen_or_maximized() {
            FullscreenOrMaximized::Unspecified => None,
            FullscreenOrMaximized::Neither => {
                Some(crate::window::window_state::FullscreenOrMaximized::Neither)
            }
            FullscreenOrMaximized::Fullscreen => {
                Some(crate::window::window_state::FullscreenOrMaximized::Fullscreen)
            }
            FullscreenOrMaximized::Maximized => {
                Some(crate::window::window_state::FullscreenOrMaximized::Maximized)
            }
        };
        let output = rule.output.map(OutputName);
        let tags = match rule.tags.is_empty() {
            true => None,
            false => Some(rule.tags.into_iter().map(TagId::Some).collect::<Vec<_>>()),
        };
        let floating_or_tiled = rule.floating.map(|floating| match floating {
            true => crate::window::rules::FloatingOrTiled::Floating,
            false => crate::window::rules::FloatingOrTiled::Tiled,
        });
        let size = rule.width.and_then(|w| {
            rule.height.and_then(|h| {
                Some((
                    NonZeroU32::try_from(w as u32).ok()?,
                    NonZeroU32::try_from(h as u32).ok()?,
                ))
            })
        });
        let location = rule.x.and_then(|x| rule.y.map(|y| (x, y)));

        crate::window::rules::WindowRule {
            output,
            tags,
            floating_or_tiled,
            fullscreen_or_maximized,
            size,
            location,
        }
    }
}
