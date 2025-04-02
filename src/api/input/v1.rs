use pinnacle_api_defs::pinnacle::input::{
    self,
    v1::{
        set_device_map_target_request::Target, switch_xkb_layout_request::Action, AccelProfile,
        BindInfo, BindRequest, BindResponse, ClickMethod, EnterBindLayerRequest,
        GetBindInfosRequest, GetBindInfosResponse, GetBindLayerStackRequest,
        GetBindLayerStackResponse, GetDeviceCapabilitiesRequest, GetDeviceCapabilitiesResponse,
        GetDeviceInfoRequest, GetDeviceInfoResponse, GetDeviceTypeRequest, GetDeviceTypeResponse,
        GetDevicesRequest, GetDevicesResponse, KeybindOnPressRequest, KeybindStreamRequest,
        KeybindStreamResponse, MousebindOnPressRequest, MousebindStreamRequest,
        MousebindStreamResponse, ScrollMethod, SendEventsMode, SetBindDescriptionRequest,
        SetBindGroupRequest, SetDeviceLibinputSettingRequest, SetDeviceMapTargetRequest,
        SetQuitBindRequest, SetReloadConfigBindRequest, SetRepeatRateRequest, SetXcursorRequest,
        SetXkbConfigRequest, SetXkbKeymapRequest, SwitchXkbLayoutRequest, TapButtonMap,
    },
};
use smithay::reexports::input as libinput;
use smithay::{
    input::keyboard::XkbConfig,
    output::Output,
    utils::{Logical, Rectangle},
};
use tonic::{Request, Status};
use tracing::{error, warn};

use crate::{
    api::{run_server_streaming, run_unary, run_unary_no_response, ResponseStream, TonicResult},
    input::{
        bind::{Edge, ModMask},
        libinput::device_type,
    },
    output::OutputName,
};

use super::InputService;

#[tonic::async_trait]
impl input::v1::input_service_server::InputService for InputService {
    type KeybindStreamStream = ResponseStream<KeybindStreamResponse>;
    type MousebindStreamStream = ResponseStream<MousebindStreamResponse>;

    async fn bind(&self, request: Request<BindRequest>) -> TonicResult<BindResponse> {
        let request = request.into_inner();

        let Some(bind) = request.bind else {
            return Err(Status::invalid_argument("bind was not specified"));
        };

        let mut mods = ModMask::new();
        for modif in bind.mods() {
            match modif {
                input::v1::Modifier::Unspecified => (),
                input::v1::Modifier::Shift => mods.shift = Some(true),
                input::v1::Modifier::Ctrl => mods.ctrl = Some(true),
                input::v1::Modifier::Alt => mods.alt = Some(true),
                input::v1::Modifier::Super => mods.super_ = Some(true),
                input::v1::Modifier::IsoLevel3Shift => mods.iso_level3_shift = Some(true),
                input::v1::Modifier::IsoLevel5Shift => mods.iso_level5_shift = Some(true),
            }
        }
        for modif in bind.ignore_mods() {
            match modif {
                input::v1::Modifier::Unspecified => (),
                input::v1::Modifier::Shift => mods.shift = None,
                input::v1::Modifier::Ctrl => mods.ctrl = None,
                input::v1::Modifier::Alt => mods.alt = None,
                input::v1::Modifier::Super => mods.super_ = None,
                input::v1::Modifier::IsoLevel3Shift => mods.iso_level3_shift = None,
                input::v1::Modifier::IsoLevel5Shift => mods.iso_level5_shift = None,
            }
        }

        let layer = bind.layer_name;
        let group = bind.group;
        let desc = bind.description;

        let Some(bind) = bind.bind else {
            return Err(Status::invalid_argument("bind.bind was not specified"));
        };

        run_unary(&self.sender, move |state| {
            let bind_id = match bind {
                input::v1::bind::Bind::Key(keybind) => {
                    let mut keysym = None;
                    if let Some(xkb_name) = keybind.xkb_name {
                        keysym = Some(if xkb_name.chars().count() == 1 {
                            let Some(ch) = xkb_name.chars().next() else {
                                unreachable!()
                            };
                            let keysym = xkbcommon::xkb::Keysym::from_char(ch);
                            keysym
                        } else {
                            let keysym = xkbcommon::xkb::keysym_from_name(
                                &xkb_name,
                                xkbcommon::xkb::KEYSYM_NO_FLAGS,
                            );
                            keysym
                        })
                    }
                    if let Some(key_code) = keybind.key_code {
                        keysym = Some(xkbcommon::xkb::Keysym::new(key_code));
                    }

                    let Some(keysym) = keysym else {
                        return Err(Status::invalid_argument("no key was specified"));
                    };

                    let bind_id = state
                        .pinnacle
                        .input_state
                        .bind_state
                        .keybinds
                        .add_keybind(keysym, mods, layer, group, desc);

                    bind_id
                }
                input::v1::bind::Bind::Mouse(mousebind) => {
                    let button = mousebind.button;
                    let bind_id = state
                        .pinnacle
                        .input_state
                        .bind_state
                        .mousebinds
                        .add_mousebind(button, mods, layer, group, desc);

                    bind_id
                }
            };

            Ok(BindResponse { bind_id })
        })
        .await
    }

    async fn get_bind_infos(
        &self,
        _request: Request<GetBindInfosRequest>,
    ) -> TonicResult<GetBindInfosResponse> {
        run_unary(&self.sender, |state| {
            // So I don't forget to add info here for new bind types
            match input::v1::bind::Bind::Key(input::v1::Keybind::default()) {
                input::v1::bind::Bind::Key(_) => (),
                input::v1::bind::Bind::Mouse(_) => (),
            }

            let push_mods = |mods: &mut Vec<input::v1::Modifier>,
                             ignore_mods: &mut Vec<input::v1::Modifier>,
                             mask: Option<bool>,
                             modif: input::v1::Modifier| {
                match mask {
                    Some(true) => mods.push(modif),
                    None => ignore_mods.push(modif),
                    Some(false) => (),
                };
            };

            let keybind_infos = state
                .pinnacle
                .input_state
                .bind_state
                .keybinds
                .id_map
                .values()
                .map(|keybind| {
                    let keybind = keybind.borrow();

                    let mut mods = Vec::new();
                    let mut ignore_mods = Vec::new();

                    push_mods(
                        &mut mods,
                        &mut ignore_mods,
                        keybind.bind_data.mods.shift,
                        input::v1::Modifier::Shift,
                    );
                    push_mods(
                        &mut mods,
                        &mut ignore_mods,
                        keybind.bind_data.mods.ctrl,
                        input::v1::Modifier::Ctrl,
                    );
                    push_mods(
                        &mut mods,
                        &mut ignore_mods,
                        keybind.bind_data.mods.alt,
                        input::v1::Modifier::Alt,
                    );
                    push_mods(
                        &mut mods,
                        &mut ignore_mods,
                        keybind.bind_data.mods.super_,
                        input::v1::Modifier::Super,
                    );
                    push_mods(
                        &mut mods,
                        &mut ignore_mods,
                        keybind.bind_data.mods.iso_level3_shift,
                        input::v1::Modifier::IsoLevel3Shift,
                    );
                    push_mods(
                        &mut mods,
                        &mut ignore_mods,
                        keybind.bind_data.mods.iso_level5_shift,
                        input::v1::Modifier::IsoLevel5Shift,
                    );

                    BindInfo {
                        bind_id: keybind.bind_data.id,
                        bind: Some(input::v1::Bind {
                            mods: mods.into_iter().map(|m| m.into()).collect(),
                            ignore_mods: ignore_mods.into_iter().map(|m| m.into()).collect(),
                            layer_name: keybind.bind_data.layer.clone(),
                            group: keybind.bind_data.group.clone(),
                            description: keybind.bind_data.desc.clone(),
                            bind: Some(input::v1::bind::Bind::Key(input::v1::Keybind {
                                key_code: Some(keybind.key.into()),
                                xkb_name: Some(xkbcommon::xkb::keysym_get_name(keybind.key)),
                            })),
                        }),
                    }
                });

            let mousebind_infos = state
                .pinnacle
                .input_state
                .bind_state
                .mousebinds
                .id_map
                .values()
                .map(|mousebind| {
                    let mousebind = mousebind.borrow();

                    let mut mods = Vec::new();
                    let mut ignore_mods = Vec::new();

                    push_mods(
                        &mut mods,
                        &mut ignore_mods,
                        mousebind.bind_data.mods.shift,
                        input::v1::Modifier::Shift,
                    );
                    push_mods(
                        &mut mods,
                        &mut ignore_mods,
                        mousebind.bind_data.mods.ctrl,
                        input::v1::Modifier::Ctrl,
                    );
                    push_mods(
                        &mut mods,
                        &mut ignore_mods,
                        mousebind.bind_data.mods.alt,
                        input::v1::Modifier::Alt,
                    );
                    push_mods(
                        &mut mods,
                        &mut ignore_mods,
                        mousebind.bind_data.mods.super_,
                        input::v1::Modifier::Super,
                    );
                    push_mods(
                        &mut mods,
                        &mut ignore_mods,
                        mousebind.bind_data.mods.iso_level3_shift,
                        input::v1::Modifier::IsoLevel3Shift,
                    );
                    push_mods(
                        &mut mods,
                        &mut ignore_mods,
                        mousebind.bind_data.mods.iso_level5_shift,
                        input::v1::Modifier::IsoLevel5Shift,
                    );

                    BindInfo {
                        bind_id: mousebind.bind_data.id,
                        bind: Some(input::v1::Bind {
                            mods: mods.into_iter().map(|m| m.into()).collect(),
                            ignore_mods: ignore_mods.into_iter().map(|m| m.into()).collect(),
                            layer_name: mousebind.bind_data.layer.clone(),
                            group: mousebind.bind_data.group.clone(),
                            description: mousebind.bind_data.desc.clone(),
                            bind: Some(input::v1::bind::Bind::Mouse(input::v1::Mousebind {
                                button: mousebind.button,
                            })),
                        }),
                    }
                });

            Ok(GetBindInfosResponse {
                bind_infos: keybind_infos.chain(mousebind_infos).collect(),
            })
        })
        .await
    }

    async fn set_quit_bind(&self, request: Request<SetQuitBindRequest>) -> TonicResult<()> {
        let bind_id = request.into_inner().bind_id;

        run_unary_no_response(&self.sender, move |state| {
            state.pinnacle.input_state.bind_state.set_quit_bind(bind_id);
        })
        .await
    }

    async fn set_reload_config_bind(
        &self,
        request: Request<SetReloadConfigBindRequest>,
    ) -> TonicResult<()> {
        let bind_id = request.into_inner().bind_id;

        run_unary_no_response(&self.sender, move |state| {
            state
                .pinnacle
                .input_state
                .bind_state
                .set_reload_config_bind(bind_id);
        })
        .await
    }

    async fn set_bind_group(&self, request: Request<SetBindGroupRequest>) -> TonicResult<()> {
        let request = request.into_inner();
        let bind_id = request.bind_id;
        let group = request.group;

        run_unary_no_response(&self.sender, move |state| {
            state
                .pinnacle
                .input_state
                .bind_state
                .set_bind_group(bind_id, group);
        })
        .await
    }

    async fn set_bind_description(
        &self,
        request: Request<SetBindDescriptionRequest>,
    ) -> TonicResult<()> {
        let request = request.into_inner();
        let bind_id = request.bind_id;
        let desc = request.desc;

        run_unary_no_response(&self.sender, move |state| {
            state
                .pinnacle
                .input_state
                .bind_state
                .set_bind_desc(bind_id, desc);
        })
        .await
    }

    async fn get_bind_layer_stack(
        &self,
        _request: Request<GetBindLayerStackRequest>,
    ) -> TonicResult<GetBindLayerStackResponse> {
        run_unary(&self.sender, |state| {
            let layer_names = state.pinnacle.input_state.bind_state.layer_stack.clone();

            Ok(GetBindLayerStackResponse { layer_names })
        })
        .await
    }

    async fn enter_bind_layer(&self, request: Request<EnterBindLayerRequest>) -> TonicResult<()> {
        let layer_name = request.into_inner().layer_name;

        run_unary(&self.sender, move |state| {
            state
                .pinnacle
                .input_state
                .bind_state
                .enter_layer(layer_name);

            Ok(())
        })
        .await
    }

    async fn keybind_stream(
        &self,
        request: Request<KeybindStreamRequest>,
    ) -> TonicResult<Self::KeybindStreamStream> {
        let request = request.into_inner();

        let bind_id = request.bind_id;

        run_server_streaming(&self.sender, move |state, sender| {
            let Some(bind) = state
                .pinnacle
                .input_state
                .bind_state
                .keybinds
                .id_map
                .get(&bind_id)
            else {
                return Err(Status::not_found(format!("bind {bind_id} was not found")));
            };

            let Some(mut recv) = bind.borrow_mut().recv.take() else {
                return Err(Status::already_exists(format!(
                    "bind {bind_id} already has a stream set up"
                )));
            };

            tokio::spawn(async move {
                while let Some(edge) = recv.recv().await {
                    let msg = Ok(KeybindStreamResponse {
                        edge: match edge {
                            Edge::Press => input::v1::Edge::Press,
                            Edge::Release => input::v1::Edge::Release,
                        }
                        .into(),
                    });
                    if sender.send(msg).is_err() {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            });

            Ok(())
        })
        .await
    }

    async fn mousebind_stream(
        &self,
        request: Request<MousebindStreamRequest>,
    ) -> TonicResult<Self::MousebindStreamStream> {
        let request = request.into_inner();

        let bind_id = request.bind_id;

        run_server_streaming(&self.sender, move |state, sender| {
            let Some(bind) = state
                .pinnacle
                .input_state
                .bind_state
                .mousebinds
                .id_map
                .get(&bind_id)
            else {
                return Err(Status::not_found(format!("bind {bind_id} was not found")));
            };

            let Some(mut recv) = bind.borrow_mut().recv.take() else {
                return Err(Status::already_exists(format!(
                    "bind {bind_id} already has a stream set up"
                )));
            };

            tokio::spawn(async move {
                while let Some(edge) = recv.recv().await {
                    let msg = Ok(MousebindStreamResponse {
                        edge: match edge {
                            Edge::Press => input::v1::Edge::Press,
                            Edge::Release => input::v1::Edge::Release,
                        }
                        .into(),
                    });
                    if sender.send(msg).is_err() {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            });

            Ok(())
        })
        .await
    }

    async fn keybind_on_press(&self, request: Request<KeybindOnPressRequest>) -> TonicResult<()> {
        let bind_id = request.into_inner().bind_id;

        run_unary_no_response(&self.sender, move |state| {
            state
                .pinnacle
                .input_state
                .bind_state
                .keybinds
                .set_keybind_has_on_press(bind_id);
        })
        .await
    }

    async fn mousebind_on_press(
        &self,
        request: Request<MousebindOnPressRequest>,
    ) -> TonicResult<()> {
        let bind_id = request.into_inner().bind_id;

        run_unary_no_response(&self.sender, move |state| {
            state
                .pinnacle
                .input_state
                .bind_state
                .mousebinds
                .set_mousebind_has_on_press(bind_id);
        })
        .await
    }

    async fn set_xkb_config(&self, request: Request<SetXkbConfigRequest>) -> TonicResult<()> {
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

    async fn set_repeat_rate(&self, request: Request<SetRepeatRateRequest>) -> TonicResult<()> {
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

    async fn set_xkb_keymap(&self, request: Request<SetXkbKeymapRequest>) -> TonicResult<()> {
        let keymap = request.into_inner().keymap;

        run_unary_no_response(&self.sender, move |state| {
            let Some(kb) = state.pinnacle.seat.get_keyboard() else {
                return;
            };
            if let Err(err) = kb.set_keymap_from_string(state, keymap) {
                warn!("Failed to set keymap: {err}");
            }
        })
        .await
    }

    async fn switch_xkb_layout(&self, request: Request<SwitchXkbLayoutRequest>) -> TonicResult<()> {
        let Some(action) = request.into_inner().action else {
            return Err(Status::invalid_argument("no layout specified"));
        };

        run_unary_no_response(&self.sender, move |state| {
            let Some(kb) = state.pinnacle.seat.get_keyboard() else {
                return;
            };
            kb.with_xkb_state(state, |mut xkb_context| match action {
                Action::Next(()) => xkb_context.cycle_next_layout(),
                Action::Prev(()) => xkb_context.cycle_prev_layout(),
                Action::Index(index) => {
                    let layout_count = xkb_context.xkb().lock().unwrap().layouts().count();
                    if index as usize >= layout_count {
                        warn!("Failed to set layout to index {index}, there are only {layout_count} layouts");
                    } else {
                        xkb_context.set_layout(smithay::input::keyboard::Layout(index));
                    }
                }
            });
        })
        .await
    }

    // FIXME: FROM IMPLS PLEASE
    async fn set_device_libinput_setting(
        &self,
        request: Request<SetDeviceLibinputSettingRequest>,
    ) -> TonicResult<()> {
        let request = request.into_inner();

        let device_sysname = request.device_sysname;
        let setting = request
            .setting
            .ok_or_else(|| Status::invalid_argument("no setting specified"))?;

        use pinnacle_api_defs::pinnacle::input::v1::set_device_libinput_setting_request::Setting;
        // TODO: move into input/libinput.rs
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
            Setting::SendEventsMode(mode) => {
                let mode = SendEventsMode::try_from(mode).unwrap_or(SendEventsMode::Unspecified);

                match mode {
                    SendEventsMode::Unspecified => {
                        return Err(Status::invalid_argument("unspecified send events mode"));
                    }
                    SendEventsMode::Enabled => Box::new(|device| {
                        let _ =
                            device.config_send_events_set_mode(libinput::SendEventsMode::ENABLED);
                    }),
                    SendEventsMode::Disabled => Box::new(|device| {
                        let _ =
                            device.config_send_events_set_mode(libinput::SendEventsMode::DISABLED);
                    }),
                    SendEventsMode::DisabledOnExternalMouse => Box::new(|device| {
                        let _ = device.config_send_events_set_mode(
                            libinput::SendEventsMode::DISABLED_ON_EXTERNAL_MOUSE,
                        );
                    }),
                }
            }
        };

        run_unary_no_response(&self.sender, move |state| {
            let device = state
                .pinnacle
                .input_state
                .libinput_state
                .devices
                .keys()
                .find(|device| device.sysname() == device_sysname);

            if let Some(device) = device {
                apply_setting(&mut device.clone());
            }
        })
        .await
    }

    async fn set_xcursor(&self, request: Request<SetXcursorRequest>) -> TonicResult<()> {
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

    async fn get_devices(
        &self,
        _request: Request<GetDevicesRequest>,
    ) -> TonicResult<GetDevicesResponse> {
        run_unary(&self.sender, |state| {
            let device_sysnames = state
                .pinnacle
                .input_state
                .libinput_state
                .devices
                .keys()
                .map(|device| device.sysname().to_string())
                .collect();

            Ok(GetDevicesResponse { device_sysnames })
        })
        .await
    }

    async fn get_device_info(
        &self,
        request: Request<GetDeviceInfoRequest>,
    ) -> TonicResult<GetDeviceInfoResponse> {
        let device_sysname = request.into_inner().device_sysname;

        run_unary(&self.sender, move |state| {
            let info = state
                .pinnacle
                .input_state
                .libinput_state
                .devices
                .keys()
                .find(|device| device.sysname() == device_sysname)
                .map(|device| GetDeviceInfoResponse {
                    name: device.name().to_string(),
                    product_id: device.id_product(),
                    vendor_id: device.id_vendor(),
                })
                .unwrap_or_default();

            Ok(info)
        })
        .await
    }

    async fn get_device_capabilities(
        &self,
        request: Request<GetDeviceCapabilitiesRequest>,
    ) -> TonicResult<GetDeviceCapabilitiesResponse> {
        let device_sysname = request.into_inner().device_sysname;

        run_unary(&self.sender, move |state| {
            let caps = state
                .pinnacle
                .input_state
                .libinput_state
                .devices
                .keys()
                .find(|device| device.sysname() == device_sysname)
                .map(|device| GetDeviceCapabilitiesResponse {
                    keyboard: device.has_capability(libinput::DeviceCapability::Keyboard),
                    pointer: device.has_capability(libinput::DeviceCapability::Pointer),
                    touch: device.has_capability(libinput::DeviceCapability::Touch),
                    tablet_tool: device.has_capability(libinput::DeviceCapability::TabletTool),
                    tablet_pad: device.has_capability(libinput::DeviceCapability::TabletPad),
                    gesture: device.has_capability(libinput::DeviceCapability::Gesture),
                    switch: device.has_capability(libinput::DeviceCapability::Switch),
                })
                .unwrap_or_default();

            Ok(caps)
        })
        .await
    }

    async fn get_device_type(
        &self,
        request: Request<GetDeviceTypeRequest>,
    ) -> TonicResult<GetDeviceTypeResponse> {
        let device_sysname = request.into_inner().device_sysname;

        run_unary(&self.sender, move |state| {
            let device_type = state
                .pinnacle
                .input_state
                .libinput_state
                .devices
                .keys()
                .find(|device| device.sysname() == device_sysname)
                .map(|device| match device_type(device) {
                    crate::input::libinput::DeviceType::Unknown => {
                        input::v1::DeviceType::Unspecified
                    }
                    crate::input::libinput::DeviceType::Touchpad => input::v1::DeviceType::Touchpad,
                    crate::input::libinput::DeviceType::Trackball => {
                        input::v1::DeviceType::Trackball
                    }
                    crate::input::libinput::DeviceType::Trackpoint => {
                        input::v1::DeviceType::Trackpoint
                    }
                    crate::input::libinput::DeviceType::Mouse => input::v1::DeviceType::Mouse,
                    crate::input::libinput::DeviceType::Tablet => input::v1::DeviceType::Tablet,
                    crate::input::libinput::DeviceType::Keyboard => input::v1::DeviceType::Keyboard,
                    crate::input::libinput::DeviceType::Switch => input::v1::DeviceType::Switch,
                })
                .unwrap_or_default();

            Ok(GetDeviceTypeResponse {
                device_type: device_type.into(),
            })
        })
        .await
    }

    async fn set_device_map_target(
        &self,
        request: Request<SetDeviceMapTargetRequest>,
    ) -> TonicResult<()> {
        let request = request.into_inner();
        let device_sysname = request.device_sysname;
        let Some(map_target) = request.target else {
            return Err(Status::invalid_argument("no map target specified"));
        };

        // FIXME:
        // See I should not have OutputName::output take the entire Pinnacle struct but here we
        // are making a whole enum just to satisfy the borrow checker
        enum MapTarget {
            Region(Rectangle<f64, Logical>),
            Output(Output),
        }

        run_unary_no_response(&self.sender, move |state| {
            let map_target = match map_target {
                Target::Region(rect) => {
                    let loc = rect.loc.unwrap_or_default();
                    let size = rect.size.unwrap_or_default();
                    Some(MapTarget::Region(Rectangle::new(
                        (loc.x as f64, loc.y as f64).into(),
                        (size.width as f64, size.height as f64).into(),
                    )))
                }
                Target::OutputName(output_name) => OutputName(output_name)
                    .output(&state.pinnacle)
                    .map(MapTarget::Output),
            };

            let device = state
                .pinnacle
                .input_state
                .libinput_state
                .devices
                .iter_mut()
                .find(|(device, _)| device.sysname() == device_sysname);

            let Some((_device, device_state)) = device else {
                return;
            };

            let Some(map_target) = map_target else {
                return;
            };

            match map_target {
                MapTarget::Region(region) => device_state.map_to_region(region),
                MapTarget::Output(output) => device_state.map_to_output(&output),
            }
        })
        .await
    }
}
