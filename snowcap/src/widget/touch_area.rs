//! A container for capturing touch events.
use iced_wgpu::core::{
    self as iced_core, Clipboard, Element, Event, Layout, Length, Point, Rectangle, Shell, Size,
    Vector, Widget, layout, mouse, overlay, renderer,
    touch::{self, Finger},
    widget::{Operation, Tree, tree},
};

/// Emit messages on touch events.
pub struct TouchArea<'a, Message, Theme = iced_core::Theme, Renderer = iced_renderer::Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    on_down: Option<Box<dyn Fn(Finger, Point) -> Message + 'a>>,
    on_up: Option<Box<dyn Fn(Finger) -> Message + 'a>>,
    on_enter: Option<Box<dyn Fn(Finger) -> Message + 'a>>,
    on_move: Option<Box<dyn Fn(Finger, Point) -> Message + 'a>>,
    on_exit: Option<Box<dyn Fn(Finger) -> Message + 'a>>,
    on_cancel: Option<Box<dyn Fn(Finger) -> Message + 'a>>,
}

impl<'a, Message, Theme, Renderer> TouchArea<'a, Message, Theme, Renderer> {
    /// The message to emit when a finger is pressed.
    #[must_use]
    pub fn on_down(mut self, on_down: impl Fn(Finger, Point) -> Message + 'a) -> Self {
        self.on_down = Some(Box::new(on_down));
        self
    }

    /// The message to emit when a finger is lifted.
    #[must_use]
    pub fn on_up(mut self, on_up: impl Fn(Finger) -> Message + 'a) -> Self {
        self.on_up = Some(Box::new(on_up));
        self
    }

    /// The message to emit when a finger move in the area.
    #[must_use]
    pub fn on_move(mut self, on_move: impl Fn(Finger, Point) -> Message + 'a) -> Self {
        self.on_move = Some(Box::new(on_move));
        self
    }

    /// The message to emit when a finger enter the area.
    #[must_use]
    pub fn on_enter(mut self, on_enter: impl Fn(Finger) -> Message + 'a) -> Self {
        self.on_enter = Some(Box::new(on_enter));
        self
    }

    /// The message to emit when the finger exits the area.
    #[must_use]
    pub fn on_exit(mut self, on_exit: impl Fn(Finger) -> Message + 'a) -> Self {
        self.on_exit = Some(Box::new(on_exit));
        self
    }

    /// The message to emit when a finger input gets canceled.
    #[must_use]
    pub fn on_cancel(mut self, on_cancel: impl Fn(Finger) -> Message + 'a) -> Self {
        self.on_cancel = Some(Box::new(on_cancel));
        self
    }
}

/// Local state of the [`TouchArea`].
#[derive(Default)]
struct State {
    tracked_finger: Vec<(Finger, Point)>,
    bounds: Rectangle,
}

impl<'a, Message, Theme, Renderer> TouchArea<'a, Message, Theme, Renderer> {
    /// Creates a [`TouchArea`] with the given content.
    pub fn new(content: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        TouchArea {
            content: content.into(),
            on_down: None,
            on_up: None,
            on_enter: None,
            on_move: None,
            on_exit: None,
            on_cancel: None,
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for TouchArea<'_, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
    Message: Clone,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if shell.is_event_captured() {
            return;
        }

        update(self, tree, event, layout, cursor, shell);
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        mouse::Interaction::None
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            renderer_style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<TouchArea<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Theme: 'a,
    Renderer: 'a + renderer::Renderer,
{
    fn from(
        area: TouchArea<'a, Message, Theme, Renderer>,
    ) -> Element<'a, Message, Theme, Renderer> {
        Element::new(area)
    }
}

/// Processes the given [`Event`] and updates the [`State`] of a [`TouchArea`]
/// accordingly.
fn update<Message: Clone, Theme, Renderer>(
    widget: &mut TouchArea<'_, Message, Theme, Renderer>,
    tree: &mut Tree,
    event: &Event,
    layout: Layout<'_>,
    _cursor: mouse::Cursor,
    shell: &mut Shell<'_, Message>,
) {
    let state: &mut State = tree.state.downcast_mut();

    let bounds = layout.bounds();

    if bounds != state.bounds {
        let prev_finger = state
            .tracked_finger
            .extract_if(.., |(_, pos)| !bounds.contains(*pos));

        if let Some(on_exit) = widget.on_exit.as_ref() {
            prev_finger.for_each(|(id, _)| shell.publish(on_exit(id)));
        } else {
            prev_finger.for_each(drop);
        }

        state.bounds = bounds;
    }

    let mut capture = false;

    match event {
        Event::Touch(touch::Event::FingerPressed { id, position }) => {
            let id = *id;
            let pos = *position;

            if bounds.contains(pos) {
                if let Some(on_enter) = widget.on_enter.as_ref() {
                    shell.publish(on_enter(id));
                    capture = true;
                }

                if let Some(on_down) = widget.on_down.as_ref() {
                    shell.publish(on_down(id, pos - Vector::new(bounds.x, bounds.y)));
                    capture = true;
                }

                state.tracked_finger.push((id, pos));
            }
        }
        Event::Touch(touch::Event::FingerLifted { id, position: _ }) => {
            if let Some((id, _)) = state
                .tracked_finger
                .extract_if(.., |(fid, _)| fid == id)
                .next()
            {
                if let Some(on_up) = widget.on_up.as_ref() {
                    shell.publish(on_up(id));
                    capture = true;
                }

                if let Some(on_exit) = widget.on_exit.as_ref() {
                    shell.publish(on_exit(id));
                    capture = true;
                }
            }
        }
        Event::Touch(touch::Event::FingerMoved { id, position }) => {
            let tracked = state.tracked_finger.iter().position(|(fid, _)| fid == id);
            let id = *id;
            let pos = *position;
            let is_over = bounds.contains(pos);

            let adj_pos = pos - Vector::new(bounds.x, bounds.y);

            if tracked.is_none() && is_over {
                state.tracked_finger.push((id, pos));

                if let Some(on_enter) = widget.on_enter.as_ref() {
                    shell.publish(on_enter(id));
                    capture = true;
                }

                if let Some(on_move) = widget.on_move.as_ref() {
                    shell.publish(on_move(id, adj_pos));
                    capture = true;
                }
            } else if let Some(idx) = tracked
                && is_over
            {
                if let Some(on_move) = widget.on_move.as_ref() {
                    shell.publish(on_move(id, adj_pos));
                    capture = true;
                };

                state.tracked_finger.get_mut(idx).unwrap().1 = pos;
            } else if let Some(idx) = tracked
                && !is_over
            {
                state.tracked_finger.swap_remove(idx);

                if let Some(on_exit) = widget.on_exit.as_ref() {
                    shell.publish(on_exit(id));
                    capture = true;
                }
            }
        }
        Event::Touch(touch::Event::FingerLost { id, position: _ }) => {
            if let Some((id, _)) = state
                .tracked_finger
                .extract_if(.., |(fid, _)| fid == id)
                .next()
                && let Some(on_cancel) = widget.on_cancel.as_ref()
            {
                shell.publish(on_cancel(id));
                capture = true;
            }
        }
        _ => {}
    }

    if capture {
        shell.capture_event();
    }
}
