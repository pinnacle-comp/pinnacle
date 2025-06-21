use pinnacle_api_defs::pinnacle::{
    output::{
        self,
        v1::{
            FocusRequest, FocusResponse, GetEnabledRequest, GetEnabledResponse,
            GetFocusStackWindowIdsRequest, GetFocusStackWindowIdsResponse, GetFocusedRequest,
            GetFocusedResponse, GetInfoRequest, GetInfoResponse, GetLocRequest, GetLocResponse,
            GetLogicalSizeRequest, GetLogicalSizeResponse, GetModesRequest, GetModesResponse,
            GetPhysicalSizeRequest, GetPhysicalSizeResponse, GetPoweredRequest, GetPoweredResponse,
            GetRequest, GetResponse, GetScaleRequest, GetScaleResponse, GetTagIdsRequest,
            GetTagIdsResponse, GetTransformRequest, GetTransformResponse, SetLocRequest,
            SetModeRequest, SetModelineRequest, SetPoweredRequest, SetScaleRequest,
            SetTransformRequest,
        },
    },
    util::{
        self,
        v1::{AbsOrRel, SetOrToggle},
    },
};
use smithay::output::Scale;
use tonic::{Request, Status};
use tracing::debug;

use crate::{
    api::{run_unary, run_unary_no_response, TonicResult},
    backend::udev::drm_mode_from_modeinfo,
    config::ConnectorSavedState,
    output::{OutputMode, OutputName},
    state::{State, WithState},
};

#[tonic::async_trait]
impl output::v1::output_service_server::OutputService for super::OutputService {
    async fn get(&self, _request: Request<GetRequest>) -> TonicResult<GetResponse> {
        run_unary(&self.sender, move |state| {
            let output_names = state
                .pinnacle
                .outputs
                .iter()
                .map(|output| output.name())
                .collect::<Vec<_>>();

            Ok(GetResponse { output_names })
        })
        .await
    }

    async fn set_loc(&self, request: Request<SetLocRequest>) -> TonicResult<()> {
        let request = request.into_inner();

        let output_name = OutputName(request.output_name);

        let x = request.x;
        let y = request.y;

        run_unary_no_response(&self.sender, move |state| {
            if let Some(saved_state) = state
                .pinnacle
                .config
                .connector_saved_states
                .get_mut(&output_name)
            {
                saved_state.loc.x = x;
                saved_state.loc.y = y;
            } else {
                state.pinnacle.config.connector_saved_states.insert(
                    output_name.clone(),
                    ConnectorSavedState {
                        loc: (x, y).into(),
                        ..Default::default()
                    },
                );
            }

            let Some(output) = output_name.output(&state.pinnacle) else {
                return;
            };
            let loc = (x, y).into();

            state.pinnacle.change_output_state(
                &mut state.backend,
                &output,
                None,
                None,
                None,
                Some(loc),
            );

            debug!("Mapping output {} to {loc:?}", output.name());

            state.pinnacle.request_layout(&output);
            state
                .pinnacle
                .output_management_manager_state
                .update::<State>();
        })
        .await
    }

    async fn set_mode(&self, request: Request<SetModeRequest>) -> TonicResult<()> {
        let request = request.into_inner();
        let output_name = OutputName(request.output_name.clone());

        run_unary(&self.sender, move |state| {
            let Some(output) = output_name.output(&state.pinnacle) else {
                return Ok(());
            };

            let Some(size) = request.size else {
                return Err(Status::invalid_argument("no size specified"));
            };

            let width = size.width;
            let height = size.height;

            let mode = match request.custom {
                true => Some(smithay::output::Mode {
                    size: (width as i32, height as i32).into(),
                    refresh: request.refresh_rate_mhz.unwrap_or(60_000) as i32,
                }),
                false => {
                    crate::output::try_pick_mode(&output, width, height, request.refresh_rate_mhz)
                }
            };

            let Some(mode) = mode else {
                return Ok(());
            };

            state.pinnacle.change_output_state(
                &mut state.backend,
                &output,
                Some(OutputMode::Smithay(mode)),
                None,
                None,
                None,
            );
            state.pinnacle.request_layout(&output);
            state
                .pinnacle
                .output_management_manager_state
                .update::<State>();

            Ok(())
        })
        .await
    }

    async fn set_modeline(&self, request: Request<SetModelineRequest>) -> TonicResult<()> {
        let request = request.into_inner();
        let output_name = OutputName(request.output_name);

        let Some(output::v1::Modeline {
            clock,
            hdisplay,
            hsync_start,
            hsync_end,
            htotal,
            vdisplay,
            vsync_start,
            vsync_end,
            vtotal,
            hsync,
            vsync,
        }) = request.modeline
        else {
            return Err(Status::invalid_argument("no modeline specified"));
        };

        run_unary_no_response(&self.sender, move |state| {
            let Some(output) = output_name.output(&state.pinnacle) else {
                return;
            };

            let mode = drm_mode_from_modeinfo(
                clock,
                hdisplay,
                hsync_start,
                hsync_end,
                htotal,
                vdisplay,
                vsync_start,
                vsync_end,
                vtotal,
                hsync,
                vsync,
            );

            state.pinnacle.change_output_state(
                &mut state.backend,
                &output,
                Some(OutputMode::Drm(mode)),
                None,
                None,
                None,
            );
            state.pinnacle.request_layout(&output);
            state
                .pinnacle
                .output_management_manager_state
                .update::<State>();
        })
        .await
    }

    async fn set_scale(&self, request: Request<SetScaleRequest>) -> TonicResult<()> {
        let request = request.into_inner();
        let abs_or_rel = request.abs_or_rel();
        let output_name = OutputName(request.output_name);
        let scale = request.scale;

        if abs_or_rel == AbsOrRel::Unspecified {
            return Err(Status::invalid_argument("abs_or_rel was unspecified"));
        }

        run_unary_no_response(&self.sender, move |state| {
            let Some(output) = output_name.output(&state.pinnacle) else {
                return;
            };

            let mut current_scale = output.current_scale().fractional_scale();

            match abs_or_rel {
                AbsOrRel::Absolute => current_scale = scale as f64,
                AbsOrRel::Relative => current_scale += scale as f64,
                AbsOrRel::Unspecified => unreachable!(),
            }

            current_scale = f64::max(current_scale, 0.25);

            state.pinnacle.change_output_state(
                &mut state.backend,
                &output,
                None,
                None,
                Some(Scale::Fractional(current_scale)),
                None,
            );

            state.pinnacle.request_layout(&output);

            state.schedule_render(&output);
            state
                .pinnacle
                .output_management_manager_state
                .update::<State>();
        })
        .await
    }

    async fn set_transform(&self, request: Request<SetTransformRequest>) -> TonicResult<()> {
        let request = request.into_inner();

        let smithay_transform = match request.transform() {
            output::v1::Transform::Unspecified => {
                return Err(Status::invalid_argument("transform was unspecified"));
            }
            output::v1::Transform::Normal => smithay::utils::Transform::Normal,
            output::v1::Transform::Transform90 => smithay::utils::Transform::_90,
            output::v1::Transform::Transform180 => smithay::utils::Transform::_180,
            output::v1::Transform::Transform270 => smithay::utils::Transform::_270,
            output::v1::Transform::Flipped => smithay::utils::Transform::Flipped,
            output::v1::Transform::Flipped90 => smithay::utils::Transform::Flipped90,
            output::v1::Transform::Flipped180 => smithay::utils::Transform::Flipped180,
            output::v1::Transform::Flipped270 => smithay::utils::Transform::Flipped270,
        };

        let output_name = OutputName(request.output_name);

        run_unary_no_response(&self.sender, move |state| {
            let Some(output) = output_name.output(&state.pinnacle) else {
                return;
            };

            state.pinnacle.change_output_state(
                &mut state.backend,
                &output,
                None,
                Some(smithay_transform),
                None,
                None,
            );
            state.pinnacle.request_layout(&output);
            state.schedule_render(&output);
            state
                .pinnacle
                .output_management_manager_state
                .update::<State>();
        })
        .await
    }

    async fn set_powered(&self, request: Request<SetPoweredRequest>) -> TonicResult<()> {
        let request = request.into_inner();

        let set = match request.set_or_toggle() {
            SetOrToggle::Unspecified => {
                return Err(Status::invalid_argument("set_or_toggle was unspecified"));
            }
            SetOrToggle::Set => Some(true),
            SetOrToggle::Unset => Some(false),
            SetOrToggle::Toggle => None,
        };

        let output_name = OutputName(request.output_name);

        run_unary_no_response(&self.sender, move |state| {
            let Some(output) = output_name.output(&state.pinnacle) else {
                return;
            };

            let mut powered = output.with_state(|state| state.powered);
            powered = set.unwrap_or(!powered);

            state.set_output_powered(&output, powered);

            if powered {
                state.schedule_render(&output);
            }
        })
        .await
    }

    async fn focus(&self, request: Request<FocusRequest>) -> TonicResult<FocusResponse> {
        let request = request.into_inner();

        let output_name = OutputName(request.output_name);

        run_unary(&self.sender, move |state| {
            let Some(output) = output_name.output(&state.pinnacle) else {
                return Ok(FocusResponse {});
            };

            state.pinnacle.focus_output(&output);

            Ok(FocusResponse {})
        })
        .await
    }

    async fn get_info(&self, request: Request<GetInfoRequest>) -> TonicResult<GetInfoResponse> {
        let output_name = OutputName(request.into_inner().output_name);

        run_unary(&self.sender, move |state| {
            let output = output_name.output(&state.pinnacle);

            let make = output
                .as_ref()
                .map(|op| op.physical_properties().make)
                .unwrap_or_default();
            let model = output
                .as_ref()
                .map(|op| op.physical_properties().model)
                .unwrap_or_default();
            let serial = output
                .as_ref()
                .map(|op| op.with_state(|state| state.serial.clone()))
                .unwrap_or_default();

            Ok(GetInfoResponse {
                make,
                model,
                serial,
            })
        })
        .await
    }

    async fn get_loc(&self, request: Request<GetLocRequest>) -> TonicResult<GetLocResponse> {
        let output_name = OutputName(request.into_inner().output_name);

        run_unary(&self.sender, move |state| {
            let output = output_name.output(&state.pinnacle);

            let loc = output.map(|op| op.current_location());

            Ok(GetLocResponse {
                loc: loc.map(|loc| util::v1::Point { x: loc.x, y: loc.y }),
            })
        })
        .await
    }

    async fn get_logical_size(
        &self,
        request: Request<GetLogicalSizeRequest>,
    ) -> TonicResult<GetLogicalSizeResponse> {
        let output_name = OutputName(request.into_inner().output_name);

        run_unary(&self.sender, move |state| {
            let output = output_name.output(&state.pinnacle);

            let size = output
                .and_then(|op| state.pinnacle.space.output_geometry(&op))
                .map(|mode| mode.size);

            Ok(GetLogicalSizeResponse {
                logical_size: size.map(|size| util::v1::Size {
                    width: size.w.try_into().unwrap_or_default(),
                    height: size.h.try_into().unwrap_or_default(),
                }),
            })
        })
        .await
    }

    async fn get_physical_size(
        &self,
        request: Request<GetPhysicalSizeRequest>,
    ) -> TonicResult<GetPhysicalSizeResponse> {
        let output_name = OutputName(request.into_inner().output_name);

        run_unary(&self.sender, move |state| {
            let output = output_name.output(&state.pinnacle);

            let size = output
                .map(|op| op.physical_properties().size)
                .unwrap_or_default();

            Ok(GetPhysicalSizeResponse {
                physical_size: Some(util::v1::Size {
                    width: size.w.try_into().unwrap_or_default(),
                    height: size.h.try_into().unwrap_or_default(),
                }),
            })
        })
        .await
    }

    async fn get_modes(&self, request: Request<GetModesRequest>) -> TonicResult<GetModesResponse> {
        let output_name = OutputName(request.into_inner().output_name);

        let from_smithay_mode = |mode: smithay::output::Mode| -> output::v1::Mode {
            output::v1::Mode {
                size: Some(util::v1::Size {
                    width: mode.size.w.try_into().unwrap_or_default(),
                    height: mode.size.h.try_into().unwrap_or_default(),
                }),
                refresh_rate_mhz: mode.refresh as u32,
            }
        };

        run_unary(&self.sender, move |state| {
            let output = output_name.output(&state.pinnacle);

            let current_mode = output
                .as_ref()
                .and_then(|op| op.current_mode())
                .map(from_smithay_mode);
            let preferred_mode = output
                .as_ref()
                .and_then(|op| op.preferred_mode())
                .map(from_smithay_mode);
            let modes = output
                .as_ref()
                .map(|output| {
                    output.with_state(|state| {
                        state.modes.iter().cloned().map(from_smithay_mode).collect()
                    })
                })
                .unwrap_or_default();

            Ok(GetModesResponse {
                current_mode,
                preferred_mode,
                modes,
            })
        })
        .await
    }

    async fn get_focused(
        &self,
        request: Request<GetFocusedRequest>,
    ) -> TonicResult<GetFocusedResponse> {
        let output_name = OutputName(request.into_inner().output_name);

        run_unary(&self.sender, move |state| {
            let output = output_name.output(&state.pinnacle);

            let focused = state
                .pinnacle
                .focused_output()
                .is_some_and(|foc_op| Some(foc_op) == output.as_ref());

            Ok(GetFocusedResponse { focused })
        })
        .await
    }

    async fn get_tag_ids(
        &self,
        request: Request<GetTagIdsRequest>,
    ) -> TonicResult<GetTagIdsResponse> {
        let output_name = OutputName(request.into_inner().output_name);

        run_unary(&self.sender, move |state| {
            let output = output_name.output(&state.pinnacle);

            let tag_ids = output
                .map(|op| {
                    op.with_state(|state| {
                        state
                            .tags
                            .iter()
                            .map(|tag| tag.id().to_inner())
                            .collect::<Vec<_>>()
                    })
                })
                .unwrap_or_default();

            Ok(GetTagIdsResponse { tag_ids })
        })
        .await
    }

    async fn get_scale(&self, request: Request<GetScaleRequest>) -> TonicResult<GetScaleResponse> {
        let output_name = OutputName(request.into_inner().output_name);

        run_unary(&self.sender, move |state| {
            let output = output_name.output(&state.pinnacle);

            let scale = output
                .map(|op| op.current_scale().fractional_scale())
                .unwrap_or(1.0);

            Ok(GetScaleResponse {
                scale: scale as f32,
            })
        })
        .await
    }

    async fn get_transform(
        &self,
        request: Request<GetTransformRequest>,
    ) -> TonicResult<GetTransformResponse> {
        let output_name = OutputName(request.into_inner().output_name);

        run_unary(&self.sender, move |state| {
            let output = output_name.output(&state.pinnacle);

            let transform = output
                .map(|output| match output.current_transform() {
                    smithay::utils::Transform::Normal => output::v1::Transform::Normal,
                    smithay::utils::Transform::_90 => output::v1::Transform::Transform90,
                    smithay::utils::Transform::_180 => output::v1::Transform::Transform180,
                    smithay::utils::Transform::_270 => output::v1::Transform::Transform270,
                    smithay::utils::Transform::Flipped => output::v1::Transform::Flipped,
                    smithay::utils::Transform::Flipped90 => output::v1::Transform::Flipped90,
                    smithay::utils::Transform::Flipped180 => output::v1::Transform::Flipped180,
                    smithay::utils::Transform::Flipped270 => output::v1::Transform::Flipped270,
                })
                .unwrap_or_default();

            Ok(GetTransformResponse {
                transform: transform.into(),
            })
        })
        .await
    }

    async fn get_enabled(
        &self,
        request: Request<GetEnabledRequest>,
    ) -> TonicResult<GetEnabledResponse> {
        let output_name = OutputName(request.into_inner().output_name);

        run_unary(&self.sender, move |state| {
            let output = output_name.output(&state.pinnacle);

            let enabled = output
                .map(|output| output.with_state(|state| state.enabled_global_id.is_some()))
                .unwrap_or_default();

            Ok(GetEnabledResponse { enabled })
        })
        .await
    }

    async fn get_powered(
        &self,
        request: Request<GetPoweredRequest>,
    ) -> TonicResult<GetPoweredResponse> {
        let output_name = OutputName(request.into_inner().output_name);

        run_unary(&self.sender, move |state| {
            let output = output_name.output(&state.pinnacle);

            let powered = output
                .map(|output| {
                    // TODO: could check drm state somehow idk
                    output.with_state(|state| state.powered)
                })
                .unwrap_or_default();

            Ok(GetPoweredResponse { powered })
        })
        .await
    }

    async fn get_focus_stack_window_ids(
        &self,
        request: Request<GetFocusStackWindowIdsRequest>,
    ) -> TonicResult<GetFocusStackWindowIdsResponse> {
        let output_name = OutputName(request.into_inner().output_name);

        run_unary(&self.sender, move |state| {
            let output = output_name.output(&state.pinnacle);

            let focus_stack_window_ids = output
                .as_ref()
                .map(|output| {
                    state
                        .pinnacle
                        .focus_stack_for_output(output)
                        .map(|win| win.with_state(|state| state.id.0))
                        .collect()
                })
                .unwrap_or_default();

            Ok(GetFocusStackWindowIdsResponse {
                window_ids: focus_stack_window_ids,
            })
        })
        .await
    }
}
