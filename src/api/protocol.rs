use std::{ffi::OsString, pin::Pin, process::Stdio};

use pinnacle_api_defs::pinnacle::{
    input::libinput::v0alpha1::set_libinput_setting_request::{
        AccelProfile, ClickMethod, ScrollMethod, TapButtonMap,
    },
    tag::v0alpha1::{
        AddRequest, AddResponse, RemoveRequest, SetActiveRequest, SetLayoutRequest, SwitchToRequest,
    },
};
use smithay::{
    input::keyboard::XkbConfig,
    reexports::{calloop, input as libinput},
};
use sysinfo::ProcessRefreshKind;
use tokio::io::AsyncBufReadExt;
use tokio_stream::Stream;
use tonic::{Request, Response, Status};

use crate::{
    input::ModifierMask,
    output::OutputName,
    state::{State, WithState},
    tag::{Tag, TagId},
};

use self::pinnacle::{
    input::{
        libinput::v0alpha1::SetLibinputSettingRequest,
        v0alpha1::{
            SetKeybindRequest, SetKeybindResponse, SetMousebindRequest, SetMousebindResponse,
            SetRepeatRateRequest, SetXkbConfigRequest,
        },
    },
    process::v0alpha1::{SpawnRequest, SpawnResponse},
    v0alpha1::QuitRequest,
};

pub use pinnacle_api_defs::pinnacle;
pub use pinnacle_api_defs::FILE_DESCRIPTOR_SET;

type ResponseStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;
pub type StateFnSender = calloop::channel::Sender<Box<dyn FnOnce(&mut State) + Send>>;

pub struct PinnacleService {
    pub sender: StateFnSender,
}

#[tonic::async_trait]
impl pinnacle::v0alpha1::pinnacle_service_server::PinnacleService for PinnacleService {
    async fn quit(&self, _request: Request<QuitRequest>) -> Result<Response<()>, Status> {
        tracing::trace!("PinnacleService.quit");
        let f = Box::new(|state: &mut State| {
            state.loop_signal.stop();
        });
        // Expect is ok here, if it panics then the state was dropped beforehand
        self.sender.send(f).expect("failed to send f");

        Ok(Response::new(()))
    }
}

pub struct InputService {
    pub sender: StateFnSender,
}

#[tonic::async_trait]
impl pinnacle::input::v0alpha1::input_service_server::InputService for InputService {
    type SetKeybindStream = ResponseStream<SetKeybindResponse>;
    type SetMousebindStream = ResponseStream<SetMousebindResponse>;

    async fn set_keybind(
        &self,
        request: Request<SetKeybindRequest>,
    ) -> Result<Response<Self::SetKeybindStream>, Status> {
        let request = request.into_inner();

        tracing::debug!(request = ?request);

        // TODO: impl From<&[Modifier]> for ModifierMask
        let modifiers = request
            .modifiers()
            .fold(ModifierMask::empty(), |acc, modifier| match modifier {
                pinnacle::input::v0alpha1::Modifier::Unspecified => acc,
                pinnacle::input::v0alpha1::Modifier::Shift => acc | ModifierMask::SHIFT,
                pinnacle::input::v0alpha1::Modifier::Ctrl => acc | ModifierMask::CTRL,
                pinnacle::input::v0alpha1::Modifier::Alt => acc | ModifierMask::ALT,
                pinnacle::input::v0alpha1::Modifier::Super => acc | ModifierMask::SUPER,
            });
        let key = request
            .key
            .ok_or_else(|| Status::invalid_argument("no key specified"))?;

        use pinnacle::input::v0alpha1::set_keybind_request::Key;
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

        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        self.sender
            .send(Box::new(move |state| {
                state
                    .input_state
                    .grpc_keybinds
                    .insert((modifiers, keysym), sender);
            }))
            .map_err(|_| Status::internal("internal state was not running"))?;

        let receiver_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);

        Ok(Response::new(Box::pin(receiver_stream)))
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
                pinnacle::input::v0alpha1::Modifier::Unspecified => acc,
                pinnacle::input::v0alpha1::Modifier::Shift => acc | ModifierMask::SHIFT,
                pinnacle::input::v0alpha1::Modifier::Ctrl => acc | ModifierMask::CTRL,
                pinnacle::input::v0alpha1::Modifier::Alt => acc | ModifierMask::ALT,
                pinnacle::input::v0alpha1::Modifier::Super => acc | ModifierMask::SUPER,
            });
        let button = request
            .button
            .ok_or_else(|| Status::invalid_argument("no key specified"))?;

        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        self.sender
            .send(Box::new(move |state| {
                state
                    .input_state
                    .grpc_mousebinds
                    .insert((modifiers, button), sender);
            }))
            .map_err(|_| Status::internal("internal state was not running"))?;

        let receiver_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);

        Ok(Response::new(Box::pin(receiver_stream)))
    }

    async fn set_xkb_config(
        &self,
        request: Request<SetXkbConfigRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let f = Box::new(move |state: &mut State| {
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
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        Ok(Response::new(()))
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

        let f = Box::new(move |state: &mut State| {
            if let Some(kb) = state.seat.get_keyboard() {
                kb.change_repeat_info(rate, delay);
            }
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        Ok(Response::new(()))
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

        use pinnacle::input::libinput::v0alpha1::set_libinput_setting_request::Setting;
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

        let f = Box::new(move |state: &mut State| {
            for device in state.input_state.libinput_devices.iter_mut() {
                apply_setting(device);
            }

            state
                .input_state
                .grpc_libinput_settings
                .insert(discriminant, apply_setting);
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        Ok(Response::new(()))
    }
}

pub struct ProcessService {
    pub sender: StateFnSender,
}

#[tonic::async_trait]
impl pinnacle::process::v0alpha1::process_service_server::ProcessService for ProcessService {
    type SpawnStream = ResponseStream<SpawnResponse>;

    async fn spawn(
        &self,
        request: Request<SpawnRequest>,
    ) -> Result<Response<Self::SpawnStream>, Status> {
        let request = request.into_inner();

        let once = request.once();
        let has_callback = request.has_callback();
        let mut command = request.args.into_iter();
        let arg0 = command
            .next()
            .ok_or_else(|| Status::invalid_argument("no args specified"))?;

        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        let f = Box::new(move |state: &mut State| {
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
                .envs(
                    [("WAYLAND_DISPLAY", state.socket_name.clone())]
                        .into_iter()
                        .chain(state.xdisplay.map(|xdisp| ("DISPLAY", format!(":{xdisp}")))),
                )
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
                        let _ = sender.send(response);
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
                        let _ = sender.send(response);
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
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        let receiver_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);

        Ok(Response::new(Box::pin(receiver_stream)))
    }
}

pub struct TagService {
    pub sender: StateFnSender,
}

#[tonic::async_trait]
impl pinnacle::tag::v0alpha1::tag_service_server::TagService for TagService {
    async fn set_active(&self, request: Request<SetActiveRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let tag_id = TagId::Some(
            request
                .tag_id
                .ok_or_else(|| Status::invalid_argument("no tag specified"))?,
        );

        let set_or_toggle = match request.set_or_toggle {
            Some(pinnacle::tag::v0alpha1::set_active_request::SetOrToggle::Set(set)) => Some(set),
            Some(pinnacle::tag::v0alpha1::set_active_request::SetOrToggle::Toggle(_)) => None,
            None => return Err(Status::invalid_argument("unspecified set or toggle")),
        };

        let f = Box::new(move |state: &mut State| {
            let Some(tag) = tag_id.tag(state) else {
                return;
            };
            match set_or_toggle {
                Some(set) => tag.set_active(set),
                None => tag.set_active(!tag.active()),
            }

            let Some(output) = tag.output(state) else {
                return;
            };

            state.update_windows(&output);
            state.update_focus(&output);
            state.schedule_render(&output);
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        Ok(Response::new(()))
    }

    async fn switch_to(&self, request: Request<SwitchToRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let tag_id = TagId::Some(
            request
                .tag_id
                .ok_or_else(|| Status::invalid_argument("no tag specified"))?,
        );

        let f = Box::new(move |state: &mut State| {
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
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        Ok(Response::new(()))
    }

    async fn add(&self, request: Request<AddRequest>) -> Result<Response<AddResponse>, Status> {
        let request = request.into_inner();

        let output_name = OutputName(
            request
                .output_name
                .ok_or_else(|| Status::invalid_argument("no output specified"))?,
        );

        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<Vec<u32>>();

        let f = Box::new(move |state: &mut State| {
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

            let _ = sender.send(tag_ids);

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

            let Some(output) = state
                .space
                .outputs()
                .find(|output| output.name() == output_name.0)
            else {
                return;
            };

            output.with_state(|state| {
                state.tags.extend(new_tags.clone());
                tracing::debug!("tags added, are now {:?}", state.tags);
            });

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
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        let ids = receiver
            .recv()
            .await
            .ok_or_else(|| Status::internal("internal state was not running"))?;

        Ok(Response::new(AddResponse { tag_ids: ids }))
    }

    async fn remove(&self, request: Request<RemoveRequest>) -> Result<Response<()>, Status> {
        todo!()
    }

    async fn set_layout(&self, request: Request<SetLayoutRequest>) -> Result<Response<()>, Status> {
        todo!()
    }

    async fn get(
        &self,
        request: Request<pinnacle::tag::v0alpha1::GetRequest>,
    ) -> Result<Response<pinnacle::tag::v0alpha1::GetResponse>, Status> {
        todo!()
    }

    async fn get_properties(
        &self,
        request: Request<pinnacle::tag::v0alpha1::GetPropertiesRequest>,
    ) -> Result<Response<pinnacle::tag::v0alpha1::GetPropertiesResponse>, Status> {
        todo!()
    }
}
