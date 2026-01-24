pub mod input_region;
pub mod output;
pub mod wlr_tasklist;

use iced::{Color, Theme, event::Status};
use iced_graphics::Viewport;
use iced_wgpu::core::{Clipboard, layout::Limits, widget};
use smithay_client_toolkit::reexports::client::{QueueHandle, protocol::wl_surface::WlSurface};

use crate::{
    handlers::keyboard::KeyboardKey,
    state::State,
    widget::{input_region::Collect, wlr_tasklist::WlrTaskListEvent},
};

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
    size: iced::Size<u32>,
}

impl SnowcapWidgetProgram {
    /// Creates a new, unbuilt widget program.
    pub fn new(view: ViewFn) -> Self {
        Self {
            view,
            user_interface: None,
            queued_events: Vec::new(),
            size: iced::Size::default(),
        }
    }

    pub fn size(&self) -> iced::Size<u32> {
        self.size
    }

    #[must_use]
    pub fn rebuild_ui(
        &mut self,
        bounds: iced::Size<u32>,
        renderer: &mut iced_renderer::Renderer,
        new_view: Option<ViewFn>,
    ) -> InputRegion {
        if let Some(view) = new_view {
            self.view = view;
        }

        let cache = self
            .user_interface
            .take()
            .map(|ui| ui.into_cache())
            .unwrap_or_default();
        let mut view = (self.view)();
        let mut tree = iced_wgpu::core::widget::Tree::empty();
        tree.diff(&view);

        let bounds = iced::Size::new(bounds.width as f32, bounds.height as f32);

        let node = view.as_widget_mut().layout(
            &mut tree,
            renderer,
            &Limits::new(iced::Size::ZERO, bounds),
        );

        let mut ui = UserInterface::build(view, bounds, cache, renderer);

        let mut collect = Collect::new();
        ui.operate(renderer, &mut collect);

        self.user_interface = Some(ui);
        self.size = iced::Size {
            width: node.size().width.ceil() as u32,
            height: node.size().height.ceil() as u32,
        };
        collect.regions.insert(
            0,
            (
                true,
                iced::Rectangle {
                    x: 0,
                    y: 0,
                    width: self.size.width as i32,
                    height: self.size.height as i32,
                },
            ),
        );

        InputRegion {
            region: collect.regions,
        }
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
    ) -> Option<(iced_runtime::user_interface::State, Vec<Status>)> {
        if self.queued_events.is_empty() {
            return None;
        }

        Some(self.user_interface.as_mut().unwrap().update(
            &self.queued_events,
            cursor,
            renderer,
            clipboard,
            messages,
        ))
    }

    pub fn operate(
        &mut self,
        renderer: &mut iced_renderer::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.user_interface
            .as_mut()
            .unwrap()
            .operate(renderer, operation);
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

    pub fn viewport(&self, scale: f32) -> Viewport {
        let buffer_width = (self.size.width as f32 * scale).ceil() as u32;
        let buffer_height = (self.size.height as f32 * scale).ceil() as u32;
        Viewport::with_physical_size(iced::Size::new(buffer_width, buffer_height), scale)
    }
}

pub struct InputRegion {
    region: Vec<(bool, iced::Rectangle<i32>)>,
}

impl InputRegion {
    pub fn update(
        self,
        queue_handle: &QueueHandle<State>,
        compositor: &smithay_client_toolkit::compositor::CompositorState,
        surface: &WlSurface,
    ) {
        let region = compositor.wl_compositor().create_region(queue_handle, ());

        for (add, rect) in self.region.into_iter() {
            if add {
                region.add(rect.x, rect.y, rect.width, rect.height);
            } else {
                region.subtract(rect.x, rect.y, rect.width, rect.height);
            }
        }

        surface.set_input_region(Some(&region));

        region.destroy();
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
    MouseArea(MouseAreaEvent),
    TextInput(TextInputEvent),
    WlrTaskList(WlrTaskListEvent),
}

#[derive(Debug, Clone)]
pub enum MouseAreaEvent {
    Press,
    Release,
    DoubleClick,
    RightPress,
    RightRelease,
    MiddlePress,
    MiddleRelease,
    Scroll(iced::mouse::ScrollDelta),
    Enter,
    Move(iced::Point),
    Exit,
}

#[derive(Debug, Clone)]
pub enum TextInputEvent {
    Input(String),
    Submit,
    Paste(String),
}

pub(crate) mod text_input {
    #[derive(Debug, Default, Clone)]
    pub(crate) struct Styles {
        pub(crate) active: Option<Style>,
        pub(crate) hovered: Option<Style>,
        pub(crate) focused: Option<Style>,
        pub(crate) hover_focused: Option<Style>,
        pub(crate) disabled: Option<Style>,
    }

    #[derive(Debug, Default, Clone)]
    pub(crate) struct Style {
        pub(crate) background: Option<iced::Background>,
        pub(crate) border: Option<iced::Border>,
        pub(crate) icon: Option<iced::Color>,
        pub(crate) placeholder: Option<iced::Color>,
        pub(crate) value: Option<iced::Color>,
        pub(crate) selection: Option<iced::Color>,
    }
}
