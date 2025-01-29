use pinnacle_api_defs::pinnacle::output::v1::{
    GetEnabledRequest, GetEnabledResponse, GetFocusStackWindowIdsRequest,
    GetFocusStackWindowIdsResponse, GetFocusedRequest, GetFocusedResponse, GetInfoRequest,
    GetInfoResponse, GetLocRequest, GetLocResponse, GetLogicalSizeRequest, GetLogicalSizeResponse,
    GetModesRequest, GetModesResponse, GetPhysicalSizeRequest, GetPhysicalSizeResponse,
    GetPoweredRequest, GetPoweredResponse, GetRequest, GetResponse, GetScaleRequest,
    GetScaleResponse, GetTagIdsRequest, GetTagIdsResponse, GetTransformRequest,
    GetTransformResponse, SetLocRequest, SetModeRequest, SetModelineRequest, SetPoweredRequest,
    SetScaleRequest, SetTransformRequest,
};

use crate::gen_test_infra;

gen_test_infra! {
    name = OutputService,
    service = pinnacle_api_defs::pinnacle::output::v1::output_service_server::OutputService,
    assoc_tys = {},
    unary = {
        get(GetRequest) -> GetResponse,
        set_loc(SetLocRequest) -> (),
        set_mode(SetModeRequest) -> (),
        set_modeline(SetModelineRequest) -> (),
        set_scale(SetScaleRequest) -> (),
        set_transform(SetTransformRequest) -> (),
        set_powered(SetPoweredRequest) -> (),
        get_info(GetInfoRequest) -> GetInfoResponse,
        get_loc(GetLocRequest) -> GetLocResponse,
        get_logical_size(GetLogicalSizeRequest) -> GetLogicalSizeResponse,
        get_physical_size(GetPhysicalSizeRequest) -> GetPhysicalSizeResponse,
        get_modes(GetModesRequest) -> GetModesResponse,
        get_focused(GetFocusedRequest) -> GetFocusedResponse,
        get_tag_ids(GetTagIdsRequest) -> GetTagIdsResponse,
        get_scale(GetScaleRequest) -> GetScaleResponse,
        get_transform(GetTransformRequest) -> GetTransformResponse,
        get_enabled(GetEnabledRequest) -> GetEnabledResponse,
        get_powered(GetPoweredRequest) -> GetPoweredResponse,
        get_focus_stack_window_ids(GetFocusStackWindowIdsRequest) -> GetFocusStackWindowIdsResponse,
    },
    other = {},
}
