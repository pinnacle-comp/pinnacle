use snowcap_api_defs::snowcap::input::{
    self,
    v1::{
        KeyboardKeyRequest, KeyboardKeyResponse, PointerButtonRequest, PointerButtonResponse,
        input_service_server,
    },
};
use tonic::{Request, Response, Status};

use crate::{
    api::{ResponseStream, run_server_streaming, run_server_streaming_mapped},
    layer::LayerId,
};

#[tonic::async_trait]
impl input_service_server::InputService for super::InputService {
    type KeyboardKeyStream = ResponseStream<KeyboardKeyResponse>;
    type PointerButtonStream = ResponseStream<PointerButtonResponse>;

    async fn keyboard_key(
        &self,
        request: Request<KeyboardKeyRequest>,
    ) -> Result<Response<Self::KeyboardKeyStream>, Status> {
        let request = request.into_inner();

        let id = LayerId(request.id);

        run_server_streaming_mapped(
            &self.sender,
            move |state, sender| {
                if let Some(layer) = state.layer_for_id(id) {
                    layer.keyboard_key_sender = Some(sender);
                }
            },
            |item| {
                let api_modifiers = input::v1::Modifiers {
                    shift: item.modifiers.shift,
                    ctrl: item.modifiers.ctrl,
                    alt: item.modifiers.alt,
                    super_: item.modifiers.logo,
                };
                Ok(KeyboardKeyResponse {
                    key: item.key.raw(),
                    modifiers: Some(api_modifiers),
                    pressed: item.pressed,
                })
            },
        )
    }

    async fn pointer_button(
        &self,
        request: Request<PointerButtonRequest>,
    ) -> Result<Response<Self::PointerButtonStream>, Status> {
        let request = request.into_inner();

        let _id = request.id;

        run_server_streaming(&self.sender, move |_state, _sender| todo!())
    }
}
