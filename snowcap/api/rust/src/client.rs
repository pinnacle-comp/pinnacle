// static LAYER: RwLock<Option<LayerServiceClient<Channel>>> = RwLock::new(None);
// static INPUT: RwLock<Option<InputServiceClient<Channel>>> = RwLock::new(None);
// static WIDGET: RwLock<Option<WidgetServiceClient<Channel>>> = RwLock::new(None);

use snowcap_api_defs::snowcap::{
    decoration::v1::decoration_service_client::DecorationServiceClient,
    input::v1::input_service_client::InputServiceClient,
    layer::v1::layer_service_client::LayerServiceClient,
    popup::v1::popup_service_client::PopupServiceClient,
    widget::v1::widget_service_client::WidgetServiceClient,
};
use tokio::sync::{RwLock, RwLockReadGuard};
use tonic::transport::Channel;

use crate::BlockOnTokio;

static CLIENT: RwLock<Option<Client>> = RwLock::const_new(None);

pub struct Client {
    layer: LayerServiceClient<Channel>,
    input: InputServiceClient<Channel>,
    widget: WidgetServiceClient<Channel>,
    decoration: DecorationServiceClient<Channel>,
    popup: PopupServiceClient<Channel>,
}

impl Client {
    pub fn init(channel: Channel) {
        CLIENT.write().block_on_tokio().replace(Self::new(channel));
    }

    fn get() -> RwLockReadGuard<'static, Self> {
        RwLockReadGuard::map(CLIENT.read().block_on_tokio(), |client| {
            client
                .as_ref()
                .expect("`Client::init` must be called beforehand")
        })
    }

    pub fn layer() -> LayerServiceClient<Channel> {
        Self::get().layer.clone()
    }

    pub fn popup() -> PopupServiceClient<Channel> {
        Self::get().popup.clone()
    }

    pub fn input() -> InputServiceClient<Channel> {
        Self::get().input.clone()
    }

    pub fn widget() -> WidgetServiceClient<Channel> {
        Self::get().widget.clone()
    }

    pub fn decoration() -> DecorationServiceClient<Channel> {
        Self::get().decoration.clone()
    }

    fn new(channel: Channel) -> Self {
        Self {
            layer: LayerServiceClient::new(channel.clone()),
            input: InputServiceClient::new(channel.clone()),
            widget: WidgetServiceClient::new(channel.clone()),
            decoration: DecorationServiceClient::new(channel.clone()),
            popup: PopupServiceClient::new(channel.clone()),
        }
    }
}
