use iced::{
    Element, Length,
    widget::{Container, container::Catalog},
};
use iced_wgpu::core::{Widget, widget::Operation};

pub struct InputRegion<
    'a,
    Message,
    Theme: Catalog = iced::Theme,
    Renderer: iced_renderer::core::Renderer = iced::Renderer,
> {
    container: Container<'a, Message, Theme, Renderer>,
    add: bool,
}

impl<'a, Message, Theme, Renderer> InputRegion<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: iced_renderer::core::Renderer,
{
    pub fn new(add: bool, content: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        Self {
            container: Container::new(content),
            add,
        }
    }

    pub fn width(self, width: impl Into<Length>) -> Self {
        Self {
            container: self.container.width(width),
            ..self
        }
    }

    pub fn height(self, height: impl Into<Length>) -> Self {
        Self {
            container: self.container.height(height),
            ..self
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct State {
    add: bool,
}

impl<'a, Message, Theme, Renderer> From<InputRegion<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: Catalog + 'a,
    Renderer: iced_renderer::core::Renderer + 'a,
{
    fn from(value: InputRegion<'a, Message, Theme, Renderer>) -> Self {
        Element::new(value)
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for InputRegion<'_, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: iced_renderer::core::Renderer,
{
    fn size(&self) -> iced::Size<iced::Length> {
        self.container.size()
    }

    fn layout(
        &self,
        tree: &mut iced_wgpu::core::widget::Tree,
        renderer: &Renderer,
        limits: &iced_wgpu::core::layout::Limits,
    ) -> iced_wgpu::core::layout::Node {
        self.container.layout(tree, renderer, limits)
    }

    fn draw(
        &self,
        tree: &iced_wgpu::core::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced_wgpu::core::renderer::Style,
        layout: iced_wgpu::core::Layout<'_>,
        cursor: iced_wgpu::core::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        self.container
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn size_hint(&self) -> iced::Size<iced::Length> {
        self.container.size_hint()
    }

    fn tag(&self) -> iced_wgpu::core::widget::tree::Tag {
        self.container.tag()
    }

    fn state(&self) -> iced_wgpu::core::widget::tree::State {
        self.container.state()
    }

    fn children(&self) -> Vec<iced_wgpu::core::widget::Tree> {
        self.container.children()
    }

    fn diff(&self, tree: &mut iced_wgpu::core::widget::Tree) {
        self.container.diff(tree);
    }

    fn operate(
        &self,
        state: &mut iced_wgpu::core::widget::Tree,
        layout: iced_wgpu::core::Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced_wgpu::core::widget::Operation,
    ) {
        let bounds = layout.bounds();

        operation.custom(None, bounds, &mut State { add: self.add });

        self.container.operate(state, layout, renderer, operation);
    }

    fn update(
        &mut self,
        state: &mut iced_wgpu::core::widget::Tree,
        event: &iced::Event,
        layout: iced_wgpu::core::Layout<'_>,
        cursor: iced_wgpu::core::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced_wgpu::core::Clipboard,
        shell: &mut iced_wgpu::core::Shell<'_, Message>,
        viewport: &iced::Rectangle,
    ) {
        self.container.update(
            state, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }

    fn mouse_interaction(
        &self,
        state: &iced_wgpu::core::widget::Tree,
        layout: iced_wgpu::core::Layout<'_>,
        cursor: iced_wgpu::core::mouse::Cursor,
        viewport: &iced::Rectangle,
        renderer: &Renderer,
    ) -> iced_wgpu::core::mouse::Interaction {
        self.container
            .mouse_interaction(state, layout, cursor, viewport, renderer)
    }

    fn overlay<'a>(
        &'a mut self,
        state: &'a mut iced_wgpu::core::widget::Tree,
        layout: iced_wgpu::core::Layout<'a>,
        renderer: &Renderer,
        viewport: &iced::Rectangle,
        translation: iced::Vector,
    ) -> Option<iced_wgpu::core::overlay::Element<'a, Message, Theme, Renderer>> {
        self.container
            .overlay(state, layout, renderer, viewport, translation)
    }
}

#[derive(Debug, Clone)]
pub struct Collect {
    pub regions: Vec<(bool, iced::Rectangle<i32>)>,
}

impl Collect {
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
        }
    }
}

impl Operation for Collect {
    fn container(
        &mut self,
        _id: Option<&iced_wgpu::core::widget::Id>,
        _bounds: iced::Rectangle,
        operate_on_children: &mut dyn FnMut(&mut dyn Operation<()>),
    ) {
        operate_on_children(self);
    }

    fn custom(
        &mut self,
        _id: Option<&iced_wgpu::core::widget::Id>,
        mut bounds: iced::Rectangle,
        state: &mut dyn std::any::Any,
    ) {
        let Some(state) = state.downcast_ref::<State>() else {
            return;
        };

        let x_diff = bounds.x - bounds.x.floor();
        let y_diff = bounds.y - bounds.y.floor();
        bounds.width += x_diff;
        bounds.height += y_diff;

        let rect = iced::Rectangle {
            x: bounds.x.floor() as i32,
            y: bounds.y.floor() as i32,
            width: bounds.width.ceil() as i32,
            height: bounds.height.ceil() as i32,
        };

        self.regions.push((state.add, rect));
    }
}
