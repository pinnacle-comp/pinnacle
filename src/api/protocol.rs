use std::{collections::HashSet, pin::Pin};

use smithay::reexports::calloop;
use tokio_stream::Stream;
use tonic::{Response, Status};

use crate::{input::ModifierMask, state::State};

use self::pinnacle::{
    input::{
        libinput::v0alpha1::SetLibinputSettingRequest,
        v0alpha1::{
            SetKeybindRequest, SetKeybindResponse, SetMousebindRequest, SetMousebindResponse,
            SetXkbConfigRequest, SetXkbRepeatRequest,
        },
    },
    v0alpha1::QuitRequest,
};

pub use pinnacle_api_defs::pinnacle;
pub use pinnacle_api_defs::FILE_DESCRIPTOR_SET;

type ResponseStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;
pub type StateFnSender = calloop::channel::Sender<Box<dyn FnOnce(&mut State) + Send>>;

pub struct PinnacleService {
    pub sender: StateFnSender,
}

#[tonic::async_trait]
impl pinnacle::v0alpha1::pinnacle_service_server::PinnacleService for PinnacleService {
    async fn quit(
        &self,
        _request: tonic::Request<QuitRequest>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        tracing::trace!("PinnacleService.quit");
        let f = Box::new(|state: &mut State| {
            state.loop_signal.stop();
        });
        // Expect is ok here, if it panics then the state was dropped beforehand
        self.sender.send(f).expect("failed to send f");

        Ok(tonic::Response::new(()))
    }
}

pub struct InputService {
    pub sender: StateFnSender,
}

#[tonic::async_trait]
impl pinnacle::input::v0alpha1::input_service_server::InputService for InputService {
    type SetKeybindStream = ResponseStream<SetKeybindResponse>;
    type SetMousebindStream = ResponseStream<SetMousebindResponse>;

    async fn set_keybind(
        &self,
        request: tonic::Request<SetKeybindRequest>,
    ) -> Result<Response<Self::SetKeybindStream>, Status> {
        let request = request.into_inner();

        tracing::debug!(request = ?request);

        let modifiers = request
            .modifiers()
            .fold(ModifierMask::empty(), |acc, modifier| match modifier {
                pinnacle::input::v0alpha1::Modifier::Unspecified => acc,
                pinnacle::input::v0alpha1::Modifier::Shift => acc | ModifierMask::SHIFT,
                pinnacle::input::v0alpha1::Modifier::Ctrl => acc | ModifierMask::CTRL,
                pinnacle::input::v0alpha1::Modifier::Alt => acc | ModifierMask::ALT,
                pinnacle::input::v0alpha1::Modifier::Super => acc | ModifierMask::SUPER,
            });
        let key = request
            .key
            .ok_or_else(|| Status::invalid_argument("no key specified"))?;

        use pinnacle::input::v0alpha1::set_keybind_request::Key;
        let keysym = match key {
            Key::RawCode(num) => {
                tracing::info!("set keybind: {:?}, raw {}", modifiers, num);
                xkbcommon::xkb::Keysym::new(num)
            }
            Key::XkbName(s) => {
                if s.chars().count() == 1 {
                    let Some(ch) = s.chars().next() else { unreachable!() };
                    let keysym = xkbcommon::xkb::Keysym::from_char(ch);
                    tracing::info!("set keybind: {:?}, {:?}", modifiers, keysym);
                    keysym
                } else {
                    let keysym =
                        xkbcommon::xkb::keysym_from_name(&s, xkbcommon::xkb::KEYSYM_NO_FLAGS);
                    tracing::info!("set keybind: {:?}, {:?}", modifiers, keysym);
                    keysym
                }
            }
        };

        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        self.sender
            .send(Box::new(move |state| {
                state
                    .input_state
                    .grpc_keybinds
                    .insert((modifiers, keysym), sender);
            }))
            .map_err(|_| Status::internal("internal state was not running"))?;

        let receiver_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);

        Ok(Response::new(
            Box::pin(receiver_stream) as Self::SetKeybindStream
        ))
    }

    async fn set_mousebind(
        &self,
        request: tonic::Request<SetMousebindRequest>,
    ) -> Result<Response<Self::SetMousebindStream>, Status> {
        todo!()
    }

    async fn set_xkb_config(
        &self,
        request: tonic::Request<SetXkbConfigRequest>,
    ) -> Result<Response<()>, Status> {
        todo!()
    }

    async fn set_xkb_repeat(
        &self,
        request: tonic::Request<SetXkbRepeatRequest>,
    ) -> Result<Response<()>, Status> {
        todo!()
    }

    async fn set_libinput_setting(
        &self,
        request: tonic::Request<SetLibinputSettingRequest>,
    ) -> Result<Response<()>, Status> {
        todo!()
    }
}
