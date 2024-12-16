use snowcap_api_defs::snowcap::input::v0alpha1::{
    input_service_server, KeyboardKeyRequest, KeyboardKeyResponse, PointerButtonRequest,
    PointerButtonResponse,
};
use tonic::{Request, Response, Status};

use crate::widget::WidgetId;

use super::{run_server_streaming, ResponseStream, StateFnSender};

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
    type KeyboardKeyStream = ResponseStream<KeyboardKeyResponse>;
    type PointerButtonStream = ResponseStream<PointerButtonResponse>;

    async fn keyboard_key(
        &self,
        request: Request<KeyboardKeyRequest>,
    ) -> Result<Response<Self::KeyboardKeyStream>, Status> {
        let request = request.into_inner();

        let Some(id) = request.id else {
            return Err(Status::invalid_argument("id was null"));
        };

        run_server_streaming(&self.sender, move |state, sender| {
            if let Some(layer) = WidgetId::from(id).layer_for_mut(state) {
                layer.keyboard_key_sender = Some(sender);
            }
        })
    }

    async fn pointer_button(
        &self,
        request: Request<PointerButtonRequest>,
    ) -> Result<Response<Self::PointerButtonStream>, Status> {
        let request = request.into_inner();

        let Some(id) = request.id else {
            return Err(Status::invalid_argument("id was null"));
        };

        run_server_streaming(&self.sender, move |state, sender| {
            if let Some(layer) = WidgetId::from(id).layer_for_mut(state) {
                layer.pointer_button_sender = Some(sender);
            }
        })
    }
}
