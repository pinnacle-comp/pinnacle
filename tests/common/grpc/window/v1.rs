use pinnacle::api::ResponseStream;
use pinnacle_api_defs::pinnacle::window::v1::{
    CloseRequest, GetAppIdRequest, GetAppIdResponse, GetFocusedRequest, GetFocusedResponse,
    GetLayoutModeRequest, GetLayoutModeResponse, GetLocRequest, GetLocResponse, GetRequest,
    GetResponse, GetSizeRequest, GetSizeResponse, GetTagIdsRequest, GetTagIdsResponse,
    GetTitleRequest, GetTitleResponse, MoveGrabRequest, MoveToTagRequest, RaiseRequest,
    ResizeGrabRequest, SetDecorationModeRequest, SetFloatingRequest, SetFocusedRequest,
    SetFullscreenRequest, SetGeometryRequest, SetMaximizedRequest, SetTagRequest,
    WindowRuleRequest, WindowRuleResponse,
};
use tonic::Streaming;

use crate::gen_test_infra;

gen_test_infra! {
    name = WindowService,
    service = pinnacle_api_defs::pinnacle::window::v1::window_service_server::WindowService,
    assoc_tys = {
        type WindowRuleStream = ResponseStream<WindowRuleResponse>;
    },
    unary = {
        get(GetRequest) -> GetResponse,
        get_app_id(GetAppIdRequest) -> GetAppIdResponse,
        get_title(GetTitleRequest) -> GetTitleResponse,
        get_loc(GetLocRequest) -> GetLocResponse,
        get_size(GetSizeRequest) -> GetSizeResponse,
        get_focused(GetFocusedRequest) -> GetFocusedResponse,
        get_layout_mode(GetLayoutModeRequest) -> GetLayoutModeResponse,
        get_tag_ids(GetTagIdsRequest) -> GetTagIdsResponse,
        close(CloseRequest) -> (),
        set_geometry(SetGeometryRequest) -> (),
        set_fullscreen(SetFullscreenRequest) -> (),
        set_maximized(SetMaximizedRequest) -> (),
        set_floating(SetFloatingRequest) -> (),
        set_focused(SetFocusedRequest) -> (),
        set_decoration_mode(SetDecorationModeRequest) -> (),
        move_to_tag(MoveToTagRequest) -> (),
        set_tag(SetTagRequest) -> (),
        raise(RaiseRequest) -> (),
        move_grab(MoveGrabRequest) -> (),
        resize_grab(ResizeGrabRequest) -> (),

    },
    other = {
        window_rule(Streaming<WindowRuleRequest>) -> Self::WindowRuleStream,
    }
}
