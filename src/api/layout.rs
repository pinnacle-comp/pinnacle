use pinnacle_api_defs::pinnacle::layout::v0alpha1::{
    layout_request::{self, ExplicitLayout},
    layout_service_server, LayoutRequest, LayoutResponse,
};
use tonic::{Request, Response, Status, Streaming};

use crate::output::OutputName;

use super::{run_bidirectional_streaming, ResponseStream, StateFnSender};

pub struct LayoutService {
    sender: StateFnSender,
}

impl LayoutService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}

#[tonic::async_trait]
impl layout_service_server::LayoutService for LayoutService {
    type LayoutStream = ResponseStream<LayoutResponse>;

    async fn layout(
        &self,
        request: Request<Streaming<LayoutRequest>>,
    ) -> Result<Response<Self::LayoutStream>, Status> {
        let in_stream = request.into_inner();

        run_bidirectional_streaming(
            self.sender.clone(),
            in_stream,
            |state, request| match request {
                Ok(request) => {
                    if let Some(body) = request.body {
                        match body {
                            layout_request::Body::Geometries(geos) => {
                                if let Err(err) = state.apply_layout(geos) {
                                    // TODO: send a Status and handle the error client side
                                    tracing::error!("{err}")
                                }
                            }
                            layout_request::Body::Layout(ExplicitLayout { output_name }) => {
                                if let Some(output) = output_name
                                    .map(OutputName)
                                    .and_then(|name| name.output(&state.pinnacle))
                                    .or_else(|| state.pinnacle.focused_output().cloned())
                                {
                                    state.pinnacle.request_layout(&output);
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    // Ignore broken pipes here, they have a code of `Unknown`
                    //
                    // Silences errors when reloading the config, unfortunately also ignores other
                    // `Unknown` errors
                    if err.code() != tonic::Code::Unknown {
                        tracing::error!("{err}")
                    }
                }
            },
            |state, sender, _join_handle| {
                state.pinnacle.layout_state.layout_request_sender = Some(sender);
            },
        )
    }
}
