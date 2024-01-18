use futures_lite::future::block_on;
use num_enum::TryFromPrimitive;
use pinnacle_api_defs::pinnacle::tag::{
    self,
    v0alpha1::{
        tag_service_client::TagServiceClient, RemoveRequest, SetActiveRequest, SetLayoutRequest,
    },
};
use tonic::transport::Channel;

#[derive(Clone, Debug)]
pub struct Tag {
    client: TagServiceClient<Channel>,
}

impl Tag {
    pub(crate) fn new(client: TagServiceClient<Channel>) -> Self {
        Self { client }
    }
}

#[derive(Debug, Clone)]
pub struct TagHandle {
    pub(crate) client: TagServiceClient<Channel>,
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

    pub fn props(&self) -> TagProperties {
        todo!()
    }
}

pub struct TagProperties {
    pub active: Option<bool>,
    pub name: Option<String>,
    pub output: (),
}
