use futures::{channel::mpsc::UnboundedSender, future::BoxFuture};
use num_enum::TryFromPrimitive;
use pinnacle_api_defs::pinnacle::{
    output::v0alpha1::output_service_client::OutputServiceClient,
    tag::{
        self,
        v0alpha1::{
            tag_service_client::TagServiceClient, AddRequest, RemoveRequest, SetActiveRequest,
            SetLayoutRequest, SwitchToRequest,
        },
    },
};
use tonic::transport::Channel;

use crate::{
    block_on,
    output::{Output, OutputHandle},
};

#[derive(Clone, Debug)]
pub struct Tag {
    channel: Channel,
    fut_sender: UnboundedSender<BoxFuture<'static, ()>>,
}

impl Tag {
    pub fn new(channel: Channel, fut_sender: UnboundedSender<BoxFuture<'static, ()>>) -> Self {
        Self {
            channel,
            fut_sender,
        }
    }

    pub fn create_tag_client(&self) -> TagServiceClient<Channel> {
        TagServiceClient::new(self.channel.clone())
    }

    pub fn create_output_client(&self) -> OutputServiceClient<Channel> {
        OutputServiceClient::new(self.channel.clone())
    }

    pub fn add(
        &self,
        output: &OutputHandle,
        tag_names: impl IntoIterator<Item = impl Into<String>>,
    ) -> impl Iterator<Item = TagHandle> {
        let mut client = self.create_tag_client();
        let output_client = self.create_output_client();

        let tag_names = tag_names.into_iter().map(Into::into).collect();

        let response = block_on(client.add(AddRequest {
            output_name: Some(output.name.clone()),
            tag_names,
        }))
        .unwrap()
        .into_inner();

        response.tag_ids.into_iter().map(move |id| TagHandle {
            client: client.clone(),
            output_client: output_client.clone(),
            id,
        })
    }

    pub fn get_all(&self) -> impl Iterator<Item = TagHandle> {
        let mut client = self.create_tag_client();
        let output_client = self.create_output_client();

        let response = block_on(client.get(tag::v0alpha1::GetRequest {}))
            .unwrap()
            .into_inner();

        response.tag_ids.into_iter().map(move |id| TagHandle {
            client: client.clone(),
            output_client: output_client.clone(),
            id,
        })
    }

    pub fn get(&self, name: impl Into<String>, output: Option<&OutputHandle>) -> Option<TagHandle> {
        let name = name.into();
        let output_module = Output::new(self.channel.clone(), self.fut_sender.clone());

        self.get_all().find(|tag| {
            let props = tag.props();

            let same_tag_name = props.name.as_ref() == Some(&name);
            let same_output = props.output.is_some_and(|op| {
                Some(op.name)
                    == output
                        .map(|o| o.name.clone())
                        .or_else(|| output_module.get_focused().map(|o| o.name))
            });

            same_tag_name && same_output
        })
    }
}

#[derive(Debug, Clone)]
pub struct TagHandle {
    pub(crate) client: TagServiceClient<Channel>,
    pub(crate) output_client: OutputServiceClient<Channel>,
    pub(crate) id: u32,
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, TryFromPrimitive)]
pub enum Layout {
    MasterStack = 1,
    Dwindle,
    Spiral,
    CornerTopLeft,
    CornerTopRight,
    CornerBottomLeft,
    CornerBottomRight,
}

impl TagHandle {
    pub fn set_active(&self, set: bool) {
        let mut client = self.client.clone();
        block_on(client.set_active(SetActiveRequest {
            tag_id: Some(self.id),
            set_or_toggle: Some(tag::v0alpha1::set_active_request::SetOrToggle::Set(set)),
        }))
        .unwrap();
    }

    pub fn toggle_active(&self) {
        let mut client = self.client.clone();
        block_on(client.set_active(SetActiveRequest {
            tag_id: Some(self.id),
            set_or_toggle: Some(tag::v0alpha1::set_active_request::SetOrToggle::Toggle(())),
        }))
        .unwrap();
    }

    pub fn remove(mut self) {
        block_on(self.client.remove(RemoveRequest {
            tag_ids: vec![self.id],
        }))
        .unwrap();
    }

    pub fn set_layout(&self, layout: Layout) {
        let mut client = self.client.clone();
        block_on(client.set_layout(SetLayoutRequest {
            tag_id: Some(self.id),
            layout: Some(layout as i32),
        }))
        .unwrap();
    }

    pub fn switch_to(&self) {
        let mut client = self.client.clone();
        block_on(client.switch_to(SwitchToRequest {
            tag_id: Some(self.id),
        }))
        .unwrap();
    }

    pub fn props(&self) -> TagProperties {
        let mut client = self.client.clone();
        let output_client = self.output_client.clone();

        let response = block_on(client.get_properties(tag::v0alpha1::GetPropertiesRequest {
            tag_id: Some(self.id),
        }))
        .unwrap()
        .into_inner();

        TagProperties {
            active: response.active,
            name: response.name,
            output: response.output_name.map(|name| OutputHandle {
                client: output_client,
                tag_client: client,
                name,
            }),
        }
    }
}

pub struct TagProperties {
    pub active: Option<bool>,
    pub name: Option<String>,
    pub output: Option<OutputHandle>,
}
