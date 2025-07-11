use snowcap_api_defs::snowcap::input::{
    self,
    v0alpha1::{
        KeyboardKeyRequest, KeyboardKeyResponse, PointerButtonRequest, PointerButtonResponse,
        input_service_server,
    },
};
use tonic::{Request, Response, Status};

use crate::{
    api::{ResponseStream, run_server_streaming, run_server_streaming_mapped},
    widget::WidgetId,
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

        let Some(id) = request.id else {
            return Err(Status::invalid_argument("id was null"));
        };

        run_server_streaming_mapped(
            &self.sender,
            move |state, sender| {
                if let Some(layer) = WidgetId::from(id).layer_for_mut(state) {
                    layer.keyboard_key_sender = Some(sender);
                }
            },
            |item| {
                let api_modifiers = input::v0alpha1::Modifiers {
                    shift: Some(item.modifiers.shift),
                    ctrl: Some(item.modifiers.ctrl),
                    alt: Some(item.modifiers.alt),
                    super_: Some(item.modifiers.logo),
                };
                Ok(KeyboardKeyResponse {
                    key: Some(item.key.raw()),
                    modifiers: Some(api_modifiers),
                    pressed: Some(item.pressed),
                })
            },
        )
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
