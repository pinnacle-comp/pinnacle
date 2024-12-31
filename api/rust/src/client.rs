use pinnacle_api_defs::pinnacle::{
    output::v1::output_service_client::OutputServiceClient,
    tag::v1::tag_service_client::TagServiceClient,
    v1::pinnacle_service_client::PinnacleServiceClient,
    window::v1::window_service_client::WindowServiceClient,
};
use tokio::sync::{RwLock, RwLockReadGuard};
use tonic::transport::Channel;

use crate::BlockOnTokio;

static CLIENT: RwLock<Option<Client>> = RwLock::const_new(None);

pub struct Client {
    pinnacle: PinnacleServiceClient<Channel>,
    window: WindowServiceClient<Channel>,
    tag: TagServiceClient<Channel>,
    output: OutputServiceClient<Channel>,
}

impl Client {
    pub fn init(channel: Channel) {
        CLIENT.write().block_on_tokio().replace(Self::new(channel));
    }

    pub fn get() -> RwLockReadGuard<'static, Self> {
        RwLockReadGuard::map(CLIENT.read().block_on_tokio(), |client| {
            client
                .as_ref()
                .expect("`Client::init` must be called beforehand")
        })
    }

    pub fn pinnacle() -> PinnacleServiceClient<Channel> {
        Self::get().pinnacle.clone()
    }

    pub fn window() -> WindowServiceClient<Channel> {
        Self::get().window.clone()
    }

    pub fn tag() -> TagServiceClient<Channel> {
        Self::get().tag.clone()
    }

    pub fn output() -> OutputServiceClient<Channel> {
        Self::get().output.clone()
    }

    fn new(channel: Channel) -> Self {
        Self {
            pinnacle: PinnacleServiceClient::new(channel.clone()),
            window: WindowServiceClient::new(channel.clone()),
            tag: TagServiceClient::new(channel.clone()),
            output: OutputServiceClient::new(channel.clone()),
        }
    }
}

// static PINNACLE: RwLock<Option<PinnacleServiceClient<Channel>>> = RwLock::const_new(None);
// static PROCESS: RwLock<Option<ProcessServiceClient<Channel>>> = RwLock::const_new(None);
// static WINDOW: RwLock<Option<WindowServiceClient<Channel>>> = RwLock::const_new(None);
// static INPUT: RwLock<Option<InputServiceClient<Channel>>> = RwLock::const_new(None);
// static OUTPUT: RwLock<Option<OutputServiceClient<Channel>>> = RwLock::const_new(None);
// static TAG: RwLock<Option<TagServiceClient<Channel>>> = RwLock::const_new(None);
// static LAYOUT: RwLock<Option<LayoutServiceClient<Channel>>> = RwLock::const_new(None);
// static RENDER: RwLock<Option<RenderServiceClient<Channel>>> = RwLock::const_new(None);
// static SIGNAL: RwLock<Option<SignalServiceClient<Channel>>> = RwLock::const_new(None);
//
// static SIGNAL_MODULE: Mutex<Option<SignalState>> = Mutex::const_new(None);
