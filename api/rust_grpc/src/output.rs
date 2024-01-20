use futures::{
    channel::mpsc::UnboundedSender, executor::block_on, future::BoxFuture, FutureExt, StreamExt,
};
use pinnacle_api_defs::pinnacle::{
    output::{
        self,
        v0alpha1::{
            output_service_client::OutputServiceClient, ConnectForAllRequest, SetLocationRequest,
        },
    },
    tag::v0alpha1::tag_service_client::TagServiceClient,
};
use tonic::transport::Channel;

use crate::tag::TagHandle;

#[derive(Debug, Clone)]
pub struct Output {
    // client: OutputServiceClient<Channel>,
    // tag_client: TagServiceClient<Channel>,
    channel: Channel,
    fut_sender: UnboundedSender<BoxFuture<'static, ()>>,
}

impl Output {
    pub fn new(channel: Channel, fut_sender: UnboundedSender<BoxFuture<'static, ()>>) -> Self {
        Self {
            channel,
            fut_sender,
        }
    }

    fn create_output_client(&self) -> OutputServiceClient<Channel> {
        OutputServiceClient::new(self.channel.clone())
    }

    fn create_tag_client(&self) -> TagServiceClient<Channel> {
        TagServiceClient::new(self.channel.clone())
    }

    pub fn get_all(&self) -> impl Iterator<Item = OutputHandle> {
        let mut client = self.create_output_client();
        let tag_client = self.create_tag_client();
        block_on(client.get(output::v0alpha1::GetRequest {}))
            .unwrap()
            .into_inner()
            .output_names
            .into_iter()
            .map(move |name| OutputHandle {
                client: client.clone(),
                tag_client: tag_client.clone(),
                name,
            })
    }

    pub fn get_focused(&self) -> Option<OutputHandle> {
        self.get_all()
            .find(|output| matches!(output.props().focused, Some(true)))
    }

    pub fn connect_for_all(&self, mut for_all: impl FnMut(OutputHandle) + 'static + Send) {
        for output in self.get_all() {
            for_all(output);
        }

        let mut client = self.create_output_client();
        let tag_client = self.create_tag_client();

        self.fut_sender
            .unbounded_send(
                async move {
                    let mut stream = client
                        .connect_for_all(ConnectForAllRequest {})
                        .await
                        .unwrap()
                        .into_inner();

                    while let Some(Ok(response)) = stream.next().await {
                        let Some(output_name) = response.output_name else {
                            continue;
                        };

                        let output = OutputHandle {
                            client: client.clone(),
                            tag_client: tag_client.clone(),
                            name: output_name,
                        };

                        for_all(output);
                    }
                }
                .boxed(),
            )
            .unwrap();
    }
}

#[derive(Clone, Debug)]
pub struct OutputHandle {
    pub(crate) client: OutputServiceClient<Channel>,
    pub(crate) tag_client: TagServiceClient<Channel>,
    pub(crate) name: String,
}

impl OutputHandle {
    pub fn set_location(&self, x: Option<i32>, y: Option<i32>) {
        let mut client = self.client.clone();
        block_on(client.set_location(SetLocationRequest {
            output_name: Some(self.name.clone()),
            x,
            y,
        }))
        .unwrap();
    }

    pub fn set_loc_adj_to(&self, other: &OutputHandle) {
        todo!()
    }

    pub fn props(&self) -> OutputProperties {
        let mut client = self.client.clone();
        let response = block_on(
            client.get_properties(output::v0alpha1::GetPropertiesRequest {
                output_name: Some(self.name.clone()),
            }),
        )
        .unwrap()
        .into_inner();

        OutputProperties {
            make: response.make,
            model: response.model,
            x: response.x,
            y: response.y,
            pixel_width: response.pixel_width,
            pixel_height: response.pixel_height,
            refresh_rate: response.refresh_rate,
            physical_width: response.physical_width,
            physical_height: response.physical_height,
            focused: response.focused,
            tags: response
                .tag_ids
                .into_iter()
                .map(|id| TagHandle {
                    client: self.tag_client.clone(),
                    output_client: self.client.clone(),
                    id,
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct OutputProperties {
    pub make: Option<String>,
    pub model: Option<String>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub pixel_width: Option<u32>,
    pub pixel_height: Option<u32>,
    pub refresh_rate: Option<u32>,
    pub physical_width: Option<u32>,
    pub physical_height: Option<u32>,
    pub focused: Option<bool>,
    pub tags: Vec<TagHandle>,
}
