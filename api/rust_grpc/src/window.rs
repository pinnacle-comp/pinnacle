use futures_lite::future::block_on;
use num_enum::TryFromPrimitive;
use pinnacle_api_defs::pinnacle::{
    tag::v0alpha1::tag_service_client::TagServiceClient,
    window::v0alpha1::{
        window_service_client::WindowServiceClient, MoveToTagRequest, SetTagRequest,
    },
    window::{
        self,
        v0alpha1::{
            GetRequest, MoveGrabRequest, ResizeGrabRequest, SetFloatingRequest,
            SetFullscreenRequest, SetMaximizedRequest,
        },
    },
};
use tonic::transport::Channel;

use crate::{input::MouseButton, tag::TagHandle, util::Geometry};

#[derive(Debug, Clone)]
pub struct Window {
    client: WindowServiceClient<Channel>,
    tag_client: TagServiceClient<Channel>,
}

impl Window {
    pub(crate) fn new(
        client: WindowServiceClient<Channel>,
        tag_client: TagServiceClient<Channel>,
    ) -> Self {
        Self { client, tag_client }
    }

    /// Get all windows.
    pub fn get_all(&self) -> impl Iterator<Item = WindowHandle> {
        let mut client = self.client.clone();
        let tag_client = self.tag_client.clone();
        block_on(client.get(GetRequest {}))
            .unwrap()
            .into_inner()
            .window_ids
            .into_iter()
            .map(move |id| WindowHandle {
                client: client.clone(),
                id,
                tag_client: tag_client.clone(),
            })
    }

    /// Get the currently focused window.
    pub fn get_focused(&self) -> Option<WindowHandle> {
        self.get_all()
            .find(|window| matches!(window.props().focused, Some(true)))
    }
}

#[derive(Debug, Clone)]
pub struct WindowHandle {
    pub(crate) client: WindowServiceClient<Channel>,
    pub(crate) id: u32,
    pub(crate) tag_client: TagServiceClient<Channel>,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, TryFromPrimitive)]
pub enum FullscreenOrMaximized {
    Neither = 1,
    Fullscreen,
    Maximized,
}

#[derive(Debug, Clone)]
pub struct WindowProperties {
    pub geometry: Option<Geometry>,
    pub class: Option<String>,
    pub title: Option<String>,
    pub focused: Option<bool>,
    pub floating: Option<bool>,
    pub fullscreen_or_maximized: Option<FullscreenOrMaximized>,
    pub tags: Vec<TagHandle>,
}

impl WindowHandle {
    pub fn set_fullscreen(&mut self, set: bool) {
        let mut client = self.client.clone();
        block_on(client.set_fullscreen(SetFullscreenRequest {
            window_id: Some(self.id),
            set_or_toggle: Some(window::v0alpha1::set_fullscreen_request::SetOrToggle::Set(
                set,
            )),
        }))
        .unwrap();
    }

    pub fn toggle_fullscreen(&mut self) {
        let mut client = self.client.clone();
        block_on(client.set_fullscreen(SetFullscreenRequest {
            window_id: Some(self.id),
            set_or_toggle: Some(window::v0alpha1::set_fullscreen_request::SetOrToggle::Toggle(())),
        }))
        .unwrap();
    }

    pub fn set_maximized(&mut self, set: bool) {
        let mut client = self.client.clone();
        block_on(client.set_maximized(SetMaximizedRequest {
            window_id: Some(self.id),
            set_or_toggle: Some(window::v0alpha1::set_maximized_request::SetOrToggle::Set(
                set,
            )),
        }))
        .unwrap();
    }

    pub fn toggle_maximized(&mut self) {
        let mut client = self.client.clone();
        block_on(client.set_maximized(SetMaximizedRequest {
            window_id: Some(self.id),
            set_or_toggle: Some(window::v0alpha1::set_maximized_request::SetOrToggle::Toggle(())),
        }))
        .unwrap();
    }

    pub fn set_floating(&mut self, set: bool) {
        let mut client = self.client.clone();
        block_on(client.set_floating(SetFloatingRequest {
            window_id: Some(self.id),
            set_or_toggle: Some(window::v0alpha1::set_floating_request::SetOrToggle::Set(
                set,
            )),
        }))
        .unwrap();
    }

    pub fn toggle_floating(&mut self) {
        let mut client = self.client.clone();
        block_on(client.set_floating(SetFloatingRequest {
            window_id: Some(self.id),
            set_or_toggle: Some(window::v0alpha1::set_floating_request::SetOrToggle::Toggle(
                (),
            )),
        }))
        .unwrap();
    }

    pub fn move_to_tag(&self, tag: &TagHandle) {
        let mut client = self.client.clone();

        block_on(client.move_to_tag(MoveToTagRequest {
            window_id: Some(self.id),
            tag_id: Some(tag.id),
        }))
        .unwrap();
    }

    pub fn set_tag(&self, tag: &TagHandle, set: bool) {
        let mut client = self.client.clone();

        block_on(client.set_tag(SetTagRequest {
            window_id: Some(self.id),
            tag_id: Some(tag.id),
            set_or_toggle: Some(window::v0alpha1::set_tag_request::SetOrToggle::Set(set)),
        }))
        .unwrap();
    }

    pub fn toggle_tag(&self, tag: &TagHandle) {
        let mut client = self.client.clone();

        block_on(client.set_tag(SetTagRequest {
            window_id: Some(self.id),
            tag_id: Some(tag.id),
            set_or_toggle: Some(window::v0alpha1::set_tag_request::SetOrToggle::Toggle(())),
        }))
        .unwrap();
    }

    pub fn begin_move(&self, button: MouseButton) {
        let mut client = self.client.clone();
        block_on(client.move_grab(MoveGrabRequest {
            button: Some(button as u32),
        }))
        .unwrap();
    }

    pub fn begin_resize(&self, button: MouseButton) {
        let mut client = self.client.clone();
        block_on(client.resize_grab(ResizeGrabRequest {
            button: Some(button as u32),
        }))
        .unwrap();
    }

    pub fn props(&self) -> WindowProperties {
        let mut client = self.client.clone();
        let tag_client = self.tag_client.clone();
        let response = block_on(
            client.get_properties(window::v0alpha1::GetPropertiesRequest {
                window_id: Some(self.id),
            }),
        )
        .unwrap()
        .into_inner();

        let fullscreen_or_maximized = response
            .fullscreen_or_maximized
            .unwrap_or_default()
            .try_into()
            .ok();

        let geometry = response.geometry.map(|geo| Geometry {
            x: geo.x(),
            y: geo.y(),
            width: geo.width() as u32,
            height: geo.height() as u32,
        });

        WindowProperties {
            geometry,
            class: response.class,
            title: response.title,
            focused: response.focused,
            floating: response.floating,
            fullscreen_or_maximized,
            tags: response
                .tag_ids
                .into_iter()
                .map(|id| TagHandle {
                    client: tag_client.clone(),
                    id,
                })
                .collect(),
        }
    }
}
