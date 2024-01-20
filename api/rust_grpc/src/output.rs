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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Alignment {
    TopAlignLeft,
    TopAlignCenter,
    TopAlignRight,
    BottomAlignLeft,
    BottomAlignCenter,
    BottomAlignRight,
    LeftAlignTop,
    LeftAlignCenter,
    LeftAlignBottom,
    RightAlignTop,
    RightAlignCenter,
    RightAlignBottom,
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

    pub fn set_loc_adj_to(&self, other: &OutputHandle, alignment: Alignment) {
        let self_props = self.props();
        let other_props = other.props();

        let attempt_set_loc = || -> Option<()> {
            let other_x = other_props.x?;
            let other_y = other_props.y?;
            let other_width = other_props.pixel_width? as i32;
            let other_height = other_props.pixel_height? as i32;

            let self_width = self_props.pixel_width? as i32;
            let self_height = self_props.pixel_height? as i32;

            use Alignment::*;

            let x: i32;
            let y: i32;

            if let TopAlignLeft | TopAlignCenter | TopAlignRight | BottomAlignLeft
            | BottomAlignCenter | BottomAlignRight = alignment
            {
                if let TopAlignLeft | TopAlignCenter | TopAlignRight = alignment {
                    y = other_y - self_height;
                } else {
                    // bottom
                    y = other_y + other_height;
                }

                match alignment {
                    TopAlignLeft | BottomAlignLeft => x = other_x,
                    TopAlignCenter | BottomAlignCenter => {
                        x = other_x + (other_width - self_width) / 2;
                    }
                    TopAlignRight | BottomAlignRight => x = other_x + (other_width - self_width),
                    _ => unreachable!(),
                }
            } else {
                if let LeftAlignTop | LeftAlignCenter | LeftAlignBottom = alignment {
                    x = other_x - self_width;
                } else {
                    x = other_x + other_width;
                }

                match alignment {
                    LeftAlignTop | RightAlignTop => y = other_y,
                    LeftAlignCenter | RightAlignCenter => {
                        y = other_y + (other_height - self_height) / 2;
                    }
                    LeftAlignBottom | RightAlignBottom => {
                        y = other_y + (other_height - self_height);
                    }
                    _ => unreachable!(),
                }
            }

            self.set_location(Some(x), Some(y));

            Some(())
        };

        attempt_set_loc();
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

    // TODO: make a macro for the following or something

    pub fn make(&self) -> Option<String> {
        self.props().make
    }

    pub fn model(&self) -> Option<String> {
        self.props().model
    }

    pub fn x(&self) -> Option<i32> {
        self.props().x
    }

    pub fn y(&self) -> Option<i32> {
        self.props().y
    }

    pub fn pixel_width(&self) -> Option<u32> {
        self.props().pixel_width
    }

    pub fn pixel_height(&self) -> Option<u32> {
        self.props().pixel_height
    }

    pub fn refresh_rate(&self) -> Option<u32> {
        self.props().refresh_rate
    }

    pub fn physical_width(&self) -> Option<u32> {
        self.props().physical_width
    }

    pub fn physical_height(&self) -> Option<u32> {
        self.props().physical_height
    }

    pub fn focused(&self) -> Option<bool> {
        self.props().focused
    }

    pub fn tags(&self) -> Vec<TagHandle> {
        self.props().tags
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
