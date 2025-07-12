use std::{any::Any, collections::HashMap};

use iced_runtime::{UserInterface, user_interface};
use iced_wgpu::core::Element;

use crate::handlers::keyboard::KeyboardKey;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct WidgetId(pub u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct WidgetIdCounter(WidgetId);

impl WidgetIdCounter {
    pub fn next_and_increment(&mut self) -> WidgetId {
        let ret = self.0;
        self.0.0 += 1;
        ret
    }
}

impl WidgetId {
    pub fn into_inner(self) -> u32 {
        self.0
    }
}

impl From<u32> for WidgetId {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

pub struct SnowcapWidgetProgram {
    pub widgets: WidgetFn,
    pub widget_state: HashMap<u32, Box<dyn Any + Send>>,
    pub user_interface:
        Option<UserInterface<'static, SnowcapMessage, iced::Theme, iced_renderer::Renderer>>,
    pub queued_events: Vec<iced::Event>,
    pub queued_messages: Vec<SnowcapMessage>,
    pub mouse_interaction: iced::mouse::Interaction,
}

impl SnowcapWidgetProgram {
    pub fn new(
        widgets: WidgetFn,
        widget_state: HashMap<u32, Box<dyn Any + Send>>,
        bounds: iced::Size,
        renderer: &mut iced_renderer::Renderer,
    ) -> Self {
        let user_interface = {
            let view = widgets(&widget_state);
            UserInterface::build(view, bounds, user_interface::Cache::default(), renderer)
        };

        Self {
            widgets,
            widget_state,
            user_interface: Some(user_interface),
            queued_events: Vec::new(),
            queued_messages: Vec::new(),
            mouse_interaction: iced::mouse::Interaction::None,
        }
    }
}

pub type WidgetFn = Box<
    dyn Fn(
        &HashMap<u32, Box<dyn Any + Send>>,
    ) -> Element<'static, SnowcapMessage, iced::Theme, iced_renderer::Renderer>,
>;

#[derive(Debug, Clone)]
pub enum SnowcapMessage {
    Noop,
    Close,
    KeyboardKey(KeyboardKey),
    WidgetEvent(WidgetId, WidgetEvent),
}

#[derive(Debug, Clone)]
pub enum WidgetEvent {
    Button,
}
