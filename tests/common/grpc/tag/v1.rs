use pinnacle_api_defs::pinnacle::tag::v1::{
    AddRequest, AddResponse, GetActiveRequest, GetActiveResponse, GetNameRequest, GetNameResponse,
    GetOutputNameRequest, GetOutputNameResponse, GetRequest, GetResponse, RemoveRequest,
    SetActiveRequest, SwitchToRequest,
};

use crate::gen_test_infra;

gen_test_infra! {
    name = TagService,
    service = pinnacle_api_defs::pinnacle::tag::v1::tag_service_server::TagService,
    assoc_tys = {},
    unary = {
        get(GetRequest) -> GetResponse,
        get_active(GetActiveRequest) -> GetActiveResponse,
        get_name(GetNameRequest) -> GetNameResponse,
        get_output_name(GetOutputNameRequest) -> GetOutputNameResponse,
        set_active(SetActiveRequest) -> (),
        switch_to(SwitchToRequest) -> (),
        add(AddRequest) -> AddResponse,
        remove(RemoveRequest) -> (),
    },
    other = {},
}
