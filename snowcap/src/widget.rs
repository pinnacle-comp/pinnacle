use iced::{Color, Theme, event::Status};
use iced_runtime::user_interface;
use iced_wgpu::core::Clipboard;

use crate::handlers::keyboard::KeyboardKey;

pub type Element = iced::Element<'static, SnowcapMessage, iced::Theme, crate::compositor::Renderer>;
pub type UserInterface =
    iced_runtime::UserInterface<'static, SnowcapMessage, iced::Theme, crate::compositor::Renderer>;

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
    view: ViewFn,
    user_interface: Option<UserInterface>,
    queued_events: Vec<iced::Event>,
}

impl SnowcapWidgetProgram {
    pub fn new(view: ViewFn, bounds: iced::Size, renderer: &mut iced_renderer::Renderer) -> Self {
        let user_interface =
            UserInterface::build(view(), bounds, user_interface::Cache::default(), renderer);

        Self {
            view,
            user_interface: Some(user_interface),
            queued_events: Vec::new(),
        }
    }

    pub fn rebuild_ui(&mut self, bounds: iced::Size, renderer: &mut iced_renderer::Renderer) {
        let cache = self.user_interface.take().unwrap().into_cache();
        let view = (self.view)();
        self.user_interface = Some(UserInterface::build(view, bounds, cache, renderer));
    }

    pub fn draw(&mut self, renderer: &mut iced_renderer::Renderer, cursor: iced::mouse::Cursor) {
        self.user_interface.as_mut().unwrap().draw(
            renderer,
            &Theme::CatppuccinFrappe,
            &iced_wgpu::core::renderer::Style {
                text_color: Color::WHITE,
            },
            cursor,
        );
    }

    pub fn update(
        &mut self,
        cursor: iced::mouse::Cursor,
        renderer: &mut iced_renderer::Renderer,
        clipboard: &mut dyn Clipboard,
        messages: &mut Vec<SnowcapMessage>,
    ) -> (iced_runtime::user_interface::State, Vec<Status>) {
        self.user_interface.as_mut().unwrap().update(
            &self.queued_events,
            cursor,
            renderer,
            clipboard,
            messages,
        )
    }

    pub fn update_view(
        &mut self,
        new_view: ViewFn,
        bounds: iced::Size,
        renderer: &mut iced_renderer::Renderer,
    ) {
        self.view = new_view;
        self.rebuild_ui(bounds, renderer);
    }

    pub fn queue_event(&mut self, event: iced::Event) {
        self.queued_events.push(event);
    }

    pub fn drain_events(&mut self) -> std::vec::Drain<'_, iced::Event> {
        self.queued_events.drain(..)
    }

    pub fn has_events_queued(&self) -> bool {
        !self.queued_events.is_empty()
    }
}

pub type ViewFn = Box<dyn Fn() -> Element>;

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
