use std::{ffi::OsString, num::NonZeroU32, pin::Pin, process::Stdio};

use pinnacle_api_defs::pinnacle::{
    input::libinput::v0alpha1::set_libinput_setting_request::{
        AccelProfile, ClickMethod, ScrollMethod, TapButtonMap,
    },
    output::v0alpha1::{ConnectForAllRequest, ConnectForAllResponse, SetLocationRequest},
    tag::v0alpha1::{
        AddRequest, AddResponse, RemoveRequest, SetActiveRequest, SetLayoutRequest, SwitchToRequest,
    },
    v0alpha1::Geometry,
    window::{
        rules::v0alpha1::{
            AddWindowRuleRequest, FullscreenOrMaximized, WindowRule, WindowRuleCondition,
        },
        v0alpha1::{
            CloseRequest, MoveGrabRequest, MoveToTagRequest, ResizeGrabRequest, SetFloatingRequest,
            SetFullscreenRequest, SetGeometryRequest, SetMaximizedRequest, SetTagRequest,
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
use tokio::io::AsyncBufReadExt;
use tokio_stream::Stream;
use tonic::{Request, Response, Status};

use crate::{
    config::ConnectorSavedState,
    focus::FocusTarget,
    input::ModifierMask,
    output::OutputName,
    state::{State, WithState},
    tag::{Tag, TagId},
    window::{
        rules::FloatingOrTiled,
        window_state::{FloatingOrTiled, WindowId},
        WindowElement,
    },
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

        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<AddResponse>();

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

            let _ = sender.send(AddResponse { tag_ids });

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

        let response = receiver
            .recv()
            .await
            .ok_or_else(|| Status::internal("internal state was not running"))?;

        Ok(Response::new(response))
    }

    // TODO: test
    async fn remove(&self, request: Request<RemoveRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let tag_ids = request.tag_ids.into_iter().map(TagId::Some);

        let f = Box::new(move |state: &mut State| {
            let tags_to_remove = tag_ids.flat_map(|id| id.tag(state)).collect::<Vec<_>>();

            for output in state.space.outputs() {
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
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        Ok(Response::new(()))
    }

    async fn set_layout(&self, request: Request<SetLayoutRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let tag_id = TagId::Some(
            request
                .tag_id
                .ok_or_else(|| Status::invalid_argument("no tag specified"))?,
        );

        use pinnacle::tag::v0alpha1::set_layout_request::Layout;

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

        let f = Box::new(move |state: &mut State| {
            let Some(tag) = tag_id.tag(state) else { return };

            tag.set_layout(layout);

            let Some(output) = tag.output(state) else { return };

            state.update_windows(&output);
            state.schedule_render(&output);
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        Ok(Response::new(()))
    }

    async fn get(
        &self,
        _request: Request<pinnacle::tag::v0alpha1::GetRequest>,
    ) -> Result<Response<pinnacle::tag::v0alpha1::GetResponse>, Status> {
        let (sender, mut receiver) =
            tokio::sync::mpsc::unbounded_channel::<pinnacle::tag::v0alpha1::GetResponse>();

        let f = Box::new(move |state: &mut State| {
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

            let _ = sender.send(pinnacle::tag::v0alpha1::GetResponse { tag_ids });
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        let response = receiver
            .recv()
            .await
            .ok_or_else(|| Status::internal("internal state was not running"))?;

        Ok(Response::new(response))
    }

    async fn get_properties(
        &self,
        request: Request<pinnacle::tag::v0alpha1::GetPropertiesRequest>,
    ) -> Result<Response<pinnacle::tag::v0alpha1::GetPropertiesResponse>, Status> {
        let request = request.into_inner();

        let tag_id = TagId::Some(
            request
                .tag_id
                .ok_or_else(|| Status::invalid_argument("no tag specified"))?,
        );

        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<
            pinnacle::tag::v0alpha1::GetPropertiesResponse,
        >();

        let f = Box::new(move |state: &mut State| {
            let tag = tag_id.tag(state);

            let output_name = tag
                .as_ref()
                .and_then(|tag| tag.output(state))
                .map(|output| output.name());
            let active = tag.as_ref().map(|tag| tag.active());
            let name = tag.as_ref().map(|tag| tag.name());

            let _ = sender.send(pinnacle::tag::v0alpha1::GetPropertiesResponse {
                active,
                name,
                output_name,
            });
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        let response = receiver
            .recv()
            .await
            .ok_or_else(|| Status::internal("internal state was not running"))?;

        Ok(Response::new(response))
    }
}

pub struct OutputService {
    pub sender: StateFnSender,
}

#[tonic::async_trait]
impl pinnacle::output::v0alpha1::output_service_server::OutputService for OutputService {
    type ConnectForAllStream = ResponseStream<ConnectForAllResponse>;

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

        let f = Box::new(move |state: &mut State| {
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
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        Ok(Response::new(()))
    }

    async fn connect_for_all(
        &self,
        _request: Request<ConnectForAllRequest>,
    ) -> Result<Response<Self::ConnectForAllStream>, Status> {
        let (sender, receiver) =
            tokio::sync::mpsc::unbounded_channel::<Result<ConnectForAllResponse, Status>>();

        let f = Box::new(move |state: &mut State| {
            for output in state.space.outputs() {
                let _ = sender.send(Ok(ConnectForAllResponse {
                    output_name: Some(output.name()),
                }));
            }

            state.config.grpc_output_callback_senders.push(sender);
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        let receiver_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);

        Ok(Response::new(Box::pin(receiver_stream)))
    }

    async fn get(
        &self,
        _request: Request<pinnacle::output::v0alpha1::GetRequest>,
    ) -> Result<Response<pinnacle::output::v0alpha1::GetResponse>, Status> {
        let (sender, mut receiver) =
            tokio::sync::mpsc::unbounded_channel::<pinnacle::output::v0alpha1::GetResponse>();

        let f = Box::new(move |state: &mut State| {
            let output_names = state
                .space
                .outputs()
                .map(|output| output.name())
                .collect::<Vec<_>>();

            let _ = sender.send(pinnacle::output::v0alpha1::GetResponse { output_names });
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        let response = receiver
            .recv()
            .await
            .ok_or_else(|| Status::internal("internal state was not running"))?;

        Ok(Response::new(response))
    }

    async fn get_properties(
        &self,
        request: Request<pinnacle::output::v0alpha1::GetPropertiesRequest>,
    ) -> Result<Response<pinnacle::output::v0alpha1::GetPropertiesResponse>, Status> {
        let request = request.into_inner();

        let output_name = OutputName(
            request
                .output_name
                .ok_or_else(|| Status::invalid_argument("no output specified"))?,
        );

        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<
            pinnacle::output::v0alpha1::GetPropertiesResponse,
        >();

        let f = Box::new(move |state: &mut State| {
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
                .focus_state
                .focused_output
                .as_ref()
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

            let _ = sender.send(pinnacle::output::v0alpha1::GetPropertiesResponse {
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
            });
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        let response = receiver
            .recv()
            .await
            .ok_or_else(|| Status::internal("internal state was not running"))?;

        Ok(Response::new(response))
    }
}

pub struct WindowService {
    pub sender: StateFnSender,
}

#[tonic::async_trait]
impl pinnacle::window::v0alpha1::window_service_server::WindowService for WindowService {
    async fn close(&self, request: Request<CloseRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let window_id = WindowId::Some(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        let f = Box::new(move |state: &mut State| {
            let Some(window) = window_id.window(state) else { return };

            match window {
                WindowElement::Wayland(window) => window.toplevel().send_close(),
                WindowElement::X11(surface) => surface.close().expect("failed to close x11 win"),
                WindowElement::X11OverrideRedirect(_) => {
                    tracing::warn!("tried to close override redirect window");
                }
                _ => unreachable!(),
            }
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        Ok(Response::new(()))
    }

    async fn set_geometry(
        &self,
        request: Request<SetGeometryRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let window_id = WindowId::Some(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        let geometry = request.geometry.unwrap_or_default();
        let x = geometry.x;
        let y = geometry.y;
        let width = geometry.width;
        let height = geometry.height;

        let f = Box::new(move |state: &mut State| {
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
            window.change_geometry(rect);
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
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        Ok(Response::new(()))
    }

    async fn set_fullscreen(
        &self,
        request: Request<SetFullscreenRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let window_id = WindowId::Some(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        let set_or_toggle = match request.set_or_toggle {
            Some(pinnacle::window::v0alpha1::set_fullscreen_request::SetOrToggle::Set(set)) => {
                Some(set)
            }
            Some(pinnacle::window::v0alpha1::set_fullscreen_request::SetOrToggle::Toggle(_)) => {
                None
            }
            None => return Err(Status::invalid_argument("unspecified set or toggle")),
        };

        let f = Box::new(move |state: &mut State| {
            let Some(window) = window_id.window(state) else {
                return;
            };
            match set_or_toggle {
                Some(set) => {
                    let is_fullscreen =
                        window.with_state(|state| state.fullscreen_or_maximized.is_fullscreen());
                    if set != is_fullscreen {
                        window.toggle_fullscreen();
                    }
                }
                None => window.toggle_fullscreen(),
            }

            let Some(output) = window.output(state) else {
                return;
            };

            state.update_windows(&output);
            state.schedule_render(&output);
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        Ok(Response::new(()))
    }

    async fn set_maximized(
        &self,
        request: Request<SetMaximizedRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let window_id = WindowId::Some(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        let set_or_toggle = match request.set_or_toggle {
            Some(pinnacle::window::v0alpha1::set_maximized_request::SetOrToggle::Set(set)) => {
                Some(set)
            }
            Some(pinnacle::window::v0alpha1::set_maximized_request::SetOrToggle::Toggle(_)) => None,
            None => return Err(Status::invalid_argument("unspecified set or toggle")),
        };

        let f = Box::new(move |state: &mut State| {
            let Some(window) = window_id.window(state) else {
                return;
            };
            match set_or_toggle {
                Some(set) => {
                    let is_maximized =
                        window.with_state(|state| state.fullscreen_or_maximized.is_maximized());
                    if set != is_maximized {
                        window.toggle_maximized();
                    }
                }
                None => window.toggle_maximized(),
            }

            let Some(output) = window.output(state) else {
                return;
            };

            state.update_windows(&output);
            state.schedule_render(&output);
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        Ok(Response::new(()))
    }

    async fn set_floating(
        &self,
        request: Request<SetFloatingRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let window_id = WindowId::Some(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        let set_or_toggle = match request.set_or_toggle {
            Some(pinnacle::window::v0alpha1::set_floating_request::SetOrToggle::Set(set)) => {
                Some(set)
            }
            Some(pinnacle::window::v0alpha1::set_floating_request::SetOrToggle::Toggle(_)) => None,
            None => return Err(Status::invalid_argument("unspecified set or toggle")),
        };

        let f = Box::new(move |state: &mut State| {
            let Some(window) = window_id.window(state) else {
                return;
            };
            match set_or_toggle {
                Some(set) => {
                    let is_floating =
                        window.with_state(|state| state.floating_or_tiled.is_floating());
                    if set != is_floating {
                        window.toggle_floating();
                    }
                }
                None => window.toggle_floating(),
            }

            let Some(output) = window.output(state) else {
                return;
            };

            state.update_windows(&output);
            state.schedule_render(&output);
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        Ok(Response::new(()))
    }

    async fn move_to_tag(
        &self,
        request: Request<MoveToTagRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let window_id = WindowId::Some(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        let tag_id = TagId::Some(
            request
                .tag_id
                .ok_or_else(|| Status::invalid_argument("no tag specified"))?,
        );

        let f = Box::new(move |state: &mut State| {
            let Some(window) = window_id.window(state) else { return };
            let Some(tag) = tag_id.tag(state) else { return };
            window.with_state(|state| {
                state.tags = vec![tag.clone()];
            });
            let Some(output) = tag.output(state) else { return };
            state.update_windows(&output);
            state.schedule_render(&output);
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        Ok(Response::new(()))
    }

    async fn set_tag(&self, request: Request<SetTagRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let window_id = WindowId::Some(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        let tag_id = TagId::Some(
            request
                .tag_id
                .ok_or_else(|| Status::invalid_argument("no tag specified"))?,
        );

        let set_or_toggle = match request.set_or_toggle {
            Some(pinnacle::window::v0alpha1::set_tag_request::SetOrToggle::Set(set)) => Some(set),
            Some(pinnacle::window::v0alpha1::set_tag_request::SetOrToggle::Toggle(_)) => None,
            None => return Err(Status::invalid_argument("unspecified set or toggle")),
        };

        let f = Box::new(move |state: &mut State| {
            let Some(window) = window_id.window(state) else { return };
            let Some(tag) = tag_id.tag(state) else { return };

            // TODO: turn state.tags into a hashset
            window.with_state(|state| state.tags.retain(|tg| tg != &tag));
            match set_or_toggle {
                Some(set) => {
                    if set {
                        window.with_state(|state| {
                            state.tags.push(tag.clone());
                        })
                    }
                }
                None => window.with_state(|state| {
                    if !state.tags.contains(&tag) {
                        state.tags.push(tag.clone());
                    }
                }),
            }

            let Some(output) = tag.output(state) else { return };
            state.update_windows(&output);
            state.schedule_render(&output);
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        Ok(Response::new(()))
    }

    async fn move_grab(&self, request: Request<MoveGrabRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let button = request
            .button
            .ok_or_else(|| Status::invalid_argument("no button specified"))?;

        let f = Box::new(move |state: &mut State| {
            let Some((FocusTarget::Window(window), _)) =
                state.focus_target_under(state.pointer_location)
            else {
                return;
            };
            let Some(wl_surf) = window.wl_surface() else { return };
            let seat = state.seat.clone();

            // We use the server one and not the client because windows like Steam don't provide
            // GrabStartData, so we need to create it ourselves.
            crate::grab::move_grab::move_request_server(
                state,
                &wl_surf,
                &seat,
                SERIAL_COUNTER.next_serial(),
                button,
            );
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        Ok(Response::new(()))
    }

    async fn resize_grab(
        &self,
        request: Request<ResizeGrabRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let button = request
            .button
            .ok_or_else(|| Status::invalid_argument("no button specified"))?;

        let f = Box::new(move |state: &mut State| {
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
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        Ok(Response::new(()))
    }

    async fn get(
        &self,
        _request: Request<pinnacle::window::v0alpha1::GetRequest>,
    ) -> Result<Response<pinnacle::window::v0alpha1::GetResponse>, Status> {
        let (sender, mut receiver) =
            tokio::sync::mpsc::unbounded_channel::<pinnacle::window::v0alpha1::GetResponse>();

        let f = Box::new(move |state: &mut State| {
            let window_ids = state
                .windows
                .iter()
                .map(|win| {
                    win.with_state(|state| match state.id {
                        WindowId::None => unreachable!(),
                        WindowId::Some(id) => id,
                    })
                })
                .collect::<Vec<_>>();

            let _ = sender.send(pinnacle::window::v0alpha1::GetResponse { window_ids });
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        let response = receiver
            .recv()
            .await
            .ok_or_else(|| Status::internal("internal state was not running"))?;

        Ok(Response::new(response))
    }

    async fn get_properties(
        &self,
        request: Request<pinnacle::window::v0alpha1::GetPropertiesRequest>,
    ) -> Result<Response<pinnacle::window::v0alpha1::GetPropertiesResponse>, Status> {
        let request = request.into_inner();

        let window_id = WindowId::Some(
            request
                .window_id
                .ok_or_else(|| Status::invalid_argument("no window specified"))?,
        );

        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<
            pinnacle::window::v0alpha1::GetPropertiesResponse,
        >();

        let f = Box::new(move |state: &mut State| {
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

            let _ = sender.send(pinnacle::window::v0alpha1::GetPropertiesResponse {
                geometry,
                class,
                title,
                focused,
                floating,
                fullscreen_or_maximized,
            });
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        let response = receiver
            .recv()
            .await
            .ok_or_else(|| Status::internal("internal state was not running"))?;

        Ok(Response::new(response))
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

        let f = Box::new(move |state: &mut State| {
            state.config.window_rules.push((cond, rule));
        });

        self.sender
            .send(f)
            .map_err(|_| Status::internal("internal state was not running"))?;

        Ok(Response::new(()))
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
        let output = rule.output.map(OutputName);
        let tags = match rule.tags.is_empty() {
            true => None,
            false => Some(rule.tags.into_iter().map(TagId::Some).collect::<Vec<_>>()),
        };
        let floating_or_tiled = rule.floating.map(|floating| match floating {
            true => crate::window::rules::FloatingOrTiled::Floating,
            false => crate::window::rules::FloatingOrTiled::Tiled,
        });
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
