use futures::executor::block_on;
use pinnacle_api_defs::pinnacle::v0alpha1::{
    pinnacle_service_client::PinnacleServiceClient, QuitRequest,
};
use tonic::transport::Channel;

#[derive(Debug, Clone)]
pub struct Pinnacle {
    client: PinnacleServiceClient<Channel>,
}

impl Pinnacle {
    pub fn new(client: PinnacleServiceClient<Channel>) -> Self {
        Self { client }
    }

    pub fn quit(&self) {
        let mut client = self.client.clone();
        block_on(client.quit(QuitRequest {})).unwrap();
    }
}
