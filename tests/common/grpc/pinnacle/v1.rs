use tonic::Streaming;

use pinnacle::api::ResponseStream;
use pinnacle_api_defs::pinnacle::v1::{
    BackendRequest, BackendResponse, KeepaliveRequest, KeepaliveResponse, QuitRequest,
    ReloadConfigRequest,
};

use crate::gen_test_infra;

gen_test_infra! {
    name = PinnacleService,
    service = pinnacle_api_defs::pinnacle::v1::pinnacle_service_server::PinnacleService,
    assoc_tys = {
        type KeepaliveStream = ResponseStream<KeepaliveResponse>;
    },
    unary = {
        quit(QuitRequest) -> (),
        reload_config(ReloadConfigRequest) -> (),
        backend(BackendRequest) -> BackendResponse,
    },
    other = {
        keepalive(Streaming<KeepaliveRequest>) -> Self::KeepaliveStream,
    }
}
