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
    popup::PopupId,
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

        let Some(target) = request.target.map(Target::from) else {
            return Err(Status::invalid_argument("no target"));
        };

        run_server_streaming_mapped(
            &self.sender,
            move |state, sender| match target {
                Target::Layer(id) => {
                    if let Some(layer) = state.layer_for_id(id) {
                        layer.keyboard_key_sender = Some(sender);
                    }
                }
                Target::Popup(id) => {
                    if let Some(popup) = state.popup_for_id(id) {
                        popup.keyboard_key_sender = Some(sender);
                    }
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
                    captured: item.captured,
                    text: item.text,
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

enum Target {
    Layer(LayerId),
    Popup(PopupId),
}

impl From<input::v1::keyboard_key_request::Target> for Target {
    fn from(value: input::v1::keyboard_key_request::Target) -> Self {
        use input::v1::keyboard_key_request::{self as api};

        match value {
            api::Target::LayerId(id) => Target::Layer(LayerId(id)),
            api::Target::PopupId(id) => Target::Popup(PopupId(id)),
        }
    }
}
