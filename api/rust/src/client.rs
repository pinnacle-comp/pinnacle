use pinnacle_api_defs::pinnacle::{
    debug::v1::debug_service_client::DebugServiceClient,
    input::v1::input_service_client::InputServiceClient,
    layout::v1::layout_service_client::LayoutServiceClient,
    output::v1::output_service_client::OutputServiceClient,
    process::v1::process_service_client::ProcessServiceClient,
    render::v1::render_service_client::RenderServiceClient,
    signal::v1::signal_service_client::SignalServiceClient,
    tag::v1::tag_service_client::TagServiceClient,
    v1::pinnacle_service_client::PinnacleServiceClient,
    window::v1::window_service_client::WindowServiceClient,
};
use tokio::sync::{MappedMutexGuard, Mutex, MutexGuard, RwLock, RwLockReadGuard};
use tonic::transport::Channel;

use crate::{signal::SignalState, BlockOnTokio};

static CLIENT: RwLock<Option<Client>> = RwLock::const_new(None);
static SIGNAL_STATE: Mutex<Option<SignalState>> = Mutex::const_new(None);

pub struct Client {
    pinnacle: PinnacleServiceClient<Channel>,
    window: WindowServiceClient<Channel>,
    tag: TagServiceClient<Channel>,
    output: OutputServiceClient<Channel>,
    input: InputServiceClient<Channel>,
    process: ProcessServiceClient<Channel>,
    layout: LayoutServiceClient<Channel>,
    render: RenderServiceClient<Channel>,
    signal: SignalServiceClient<Channel>,
    debug: DebugServiceClient<Channel>,
}

impl Client {
    pub fn init(channel: Channel) {
        CLIENT.write().block_on_tokio().replace(Self::new(channel));
        SIGNAL_STATE
            .lock()
            .block_on_tokio()
            .replace(SignalState::new());
    }

    fn get() -> RwLockReadGuard<'static, Self> {
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

    pub fn input() -> InputServiceClient<Channel> {
        Self::get().input.clone()
    }

    pub fn process() -> ProcessServiceClient<Channel> {
        Self::get().process.clone()
    }

    pub fn layout() -> LayoutServiceClient<Channel> {
        Self::get().layout.clone()
    }

    pub fn render() -> RenderServiceClient<Channel> {
        Self::get().render.clone()
    }

    pub fn signal() -> SignalServiceClient<Channel> {
        Self::get().signal.clone()
    }

    pub fn signal_state() -> MappedMutexGuard<'static, SignalState> {
        MutexGuard::map(SIGNAL_STATE.lock().block_on_tokio(), |signal_state| {
            signal_state
                .as_mut()
                .expect("`Client::init` must be called beforehand")
        })
    }

    pub fn debug() -> DebugServiceClient<Channel> {
        Self::get().debug.clone()
    }

    fn new(channel: Channel) -> Self {
        Self {
            pinnacle: PinnacleServiceClient::new(channel.clone()),
            window: WindowServiceClient::new(channel.clone()),
            tag: TagServiceClient::new(channel.clone()),
            output: OutputServiceClient::new(channel.clone()),
            input: InputServiceClient::new(channel.clone()),
            process: ProcessServiceClient::new(channel.clone()),
            layout: LayoutServiceClient::new(channel.clone()),
            render: RenderServiceClient::new(channel.clone()),
            signal: SignalServiceClient::new(channel.clone()),
            debug: DebugServiceClient::new(channel.clone()),
        }
    }
}
