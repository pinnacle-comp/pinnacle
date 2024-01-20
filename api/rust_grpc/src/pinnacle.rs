use futures::executor::block_on;
use pinnacle_api_defs::pinnacle::v0alpha1::{
    pinnacle_service_client::PinnacleServiceClient, QuitRequest,
};
use tonic::transport::Channel;

#[derive(Debug, Clone)]
pub struct Pinnacle {
    channel: Channel,
}

impl Pinnacle {
    pub fn new(channel: Channel) -> Self {
        Self { channel }
    }

    fn create_pinnacle_client(&self) -> PinnacleServiceClient<Channel> {
        PinnacleServiceClient::new(self.channel.clone())
    }

    pub fn quit(&self) {
        let mut client = self.create_pinnacle_client();
        block_on(client.quit(QuitRequest {})).unwrap();
    }
}
