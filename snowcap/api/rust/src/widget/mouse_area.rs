//! Mouse Event handling

use std::sync::Arc;

use snowcap_api_defs::snowcap::widget;

use super::{Widget, WidgetDef, WidgetId};

/// Emits messages on mouse events.
#[derive(Debug, Clone, PartialEq)]
pub struct MouseArea<Msg> {
    pub child: WidgetDef<Msg>,
    pub interaction: Option<Interaction>,
    pub(crate) widget_id: Option<WidgetId>,
    pub(crate) callbacks: Callbacks<Msg>,
}

impl<Msg> MouseArea<Msg> {
    /// Create a [`MouseArea`] with the given content
    pub fn new(child: impl Into<WidgetDef<Msg>>) -> Self {
        Self {
            child: child.into(),
            interaction: None,
            widget_id: None,
            callbacks: Callbacks {
                on_press: None,
                on_release: None,
                on_double_click: None,
                on_right_press: None,
                on_right_release: None,
                on_middle_press: None,
                on_middle_release: None,
                on_scroll: None,
                on_enter: None,
                on_move: None,
                on_exit: None,
            },
        }
    }

    /// [`Interaction`] to use when hovering the area.
    pub fn interaction(self, interaction: Interaction) -> Self {
        Self {
            interaction: Some(interaction),
            ..self
        }
    }

    /// Message to emit on a left button press.
    pub fn on_press(self, on_press: Msg) -> Self {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            callbacks: Callbacks {
                on_press: Some(on_press),
                ..self.callbacks
            },
            ..self
        }
    }

    /// Message to emit on a left button release.
    pub fn on_release(self, on_release: Msg) -> Self {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            callbacks: Callbacks {
                on_release: Some(on_release),
                ..self.callbacks
            },
            ..self
        }
    }

    /// Message to emit on a left button double click.
    pub fn on_double_click(self, on_double_click: Msg) -> Self {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            callbacks: Callbacks {
                on_double_click: Some(on_double_click),
                ..self.callbacks
            },
            ..self
        }
    }

    /// Message to emit on a right button press.
    pub fn on_right_press(self, on_right_press: Msg) -> Self {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            callbacks: Callbacks {
                on_right_press: Some(on_right_press),
                ..self.callbacks
            },
            ..self
        }
    }

    /// Message to emit on a right button release.
    pub fn on_right_release(self, on_right_release: Msg) -> Self {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            callbacks: Callbacks {
                on_right_release: Some(on_right_release),
                ..self.callbacks
            },
            ..self
        }
    }

    /// Message to emit on a middle button press.
    pub fn on_middle_press(self, on_middle_press: Msg) -> Self {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            callbacks: Callbacks {
                on_middle_press: Some(on_middle_press),
                ..self.callbacks
            },
            ..self
        }
    }

    /// Message to emit on a middle button release.
    pub fn on_middle_release(self, on_middle_release: Msg) -> Self {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            callbacks: Callbacks {
                on_middle_release: Some(on_middle_release),
                ..self.callbacks
            },
            ..self
        }
    }

    /// Message to emit when the scroll wheel is used.
    pub fn on_scroll<F>(self, on_scroll: F) -> Self
    where
        F: Fn(ScrollDelta) -> Msg + Sync + Send + 'static,
    {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            callbacks: Callbacks {
                on_scroll: Some(Arc::new(on_scroll)),
                ..self.callbacks
            },
            ..self
        }
    }

    /// Message to emit when the mouse pointer enter the area.
    pub fn on_enter(self, on_enter: Msg) -> Self {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            callbacks: Callbacks {
                on_enter: Some(on_enter),
                ..self.callbacks
            },
            ..self
        }
    }

    /// Message to emit when the mouse move in the area.
    pub fn on_move<F>(self, on_move: F) -> Self
    where
        F: Fn(Point) -> Msg + Sync + Send + 'static,
    {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            callbacks: Callbacks {
                on_move: Some(Arc::new(on_move)),
                ..self.callbacks
            },
            ..self
        }
    }

    /// Message to emit when the mouse pointer exit the area.
    pub fn on_exit(self, on_exit: Msg) -> Self {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            callbacks: Callbacks {
                on_exit: Some(on_exit),
                ..self.callbacks
            },
            ..self
        }
    }
}

impl<Msg> From<MouseArea<Msg>> for Widget<Msg> {
    fn from(value: MouseArea<Msg>) -> Self {
        Widget::MouseArea(Box::new(value))
    }
}

impl<Msg> From<MouseArea<Msg>> for widget::v1::MouseArea {
    fn from(value: MouseArea<Msg>) -> Self {
        let inter: Option<widget::v1::mouse_area::Interaction> = value.interaction.map(From::from);

        Self {
            child: Some(Box::new(value.child.into())),
            on_press: value.callbacks.on_press.is_some(),
            on_release: value.callbacks.on_release.is_some(),
            on_double_click: value.callbacks.on_double_click.is_some(),
            on_right_press: value.callbacks.on_right_press.is_some(),
            on_right_release: value.callbacks.on_right_release.is_some(),
            on_middle_press: value.callbacks.on_middle_press.is_some(),
            on_middle_release: value.callbacks.on_middle_release.is_some(),
            on_scroll: value.callbacks.on_scroll.is_some(),
            on_enter: value.callbacks.on_enter.is_some(),
            on_move: value.callbacks.on_move.is_some(),
            on_exit: value.callbacks.on_exit.is_some(),
            interaction: inter.map(From::from),
            widget_id: value.widget_id.map(WidgetId::to_inner),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Interaction {
    None,
    Idle,
    Pointer,
    Grab,
    Text,
    Crosshair,
    Grabbing,
    ResizeHorizontal,
    ResizeVertical,
    ResizeDiagonalUp,
    ResizeDiagonalDown,
    NotAllowed,
    ZoomIn,
    ZoomOut,
    Cell,
    Move,
    Copy,
    Help,
}

impl From<Interaction> for widget::v1::mouse_area::Interaction {
    fn from(value: Interaction) -> Self {
        match value {
            Interaction::None => Self::None,
            Interaction::Idle => Self::Idle,
            Interaction::Pointer => Self::Pointer,
            Interaction::Grab => Self::Grab,
            Interaction::Text => Self::Text,
            Interaction::Crosshair => Self::Crosshair,
            Interaction::Grabbing => Self::Grabbing,
            Interaction::ResizeHorizontal => Self::ResizeHorizontal,
            Interaction::ResizeVertical => Self::ResizeVertical,
            Interaction::ResizeDiagonalUp => Self::ResizeDiagonalUp,
            Interaction::ResizeDiagonalDown => Self::ResizeDiagonalDown,
            Interaction::NotAllowed => Self::NotAllowed,
            Interaction::ZoomIn => Self::ZoomIn,
            Interaction::ZoomOut => Self::ZoomOut,
            Interaction::Cell => Self::Cell,
            Interaction::Move => Self::Move,
            Interaction::Copy => Self::Copy,
            Interaction::Help => Self::Help,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Press,
    Release,
    DoubleClick,
    RightPress,
    RightRelease,
    MiddlePress,
    MiddleRelease,
    Scroll(ScrollDelta),
    Enter,
    Move(Point),
    Exit,
}

#[derive(Clone)]
pub struct Callbacks<Msg> {
    pub(crate) on_press: Option<Msg>,
    pub(crate) on_release: Option<Msg>,
    pub(crate) on_double_click: Option<Msg>,
    pub(crate) on_right_press: Option<Msg>,
    pub(crate) on_right_release: Option<Msg>,
    pub(crate) on_middle_press: Option<Msg>,
    pub(crate) on_middle_release: Option<Msg>,
    pub(crate) on_scroll: Option<Arc<dyn Fn(ScrollDelta) -> Msg + Sync + Send>>,
    pub(crate) on_enter: Option<Msg>,
    pub(crate) on_move: Option<Arc<dyn Fn(Point) -> Msg + Sync + Send>>,
    pub(crate) on_exit: Option<Msg>,
}

impl<Msg> Callbacks<Msg> {
    pub(crate) fn process_event(self, evt: Event) -> Option<Msg> {
        match evt {
            Event::Press => self.on_press,
            Event::Release => self.on_release,
            Event::DoubleClick => self.on_double_click,
            Event::RightPress => self.on_right_press,
            Event::RightRelease => self.on_right_release,
            Event::MiddlePress => self.on_middle_press,
            Event::MiddleRelease => self.on_middle_release,
            Event::Scroll(scroll_delta) => self.on_scroll.map(|handler| handler(scroll_delta)),
            Event::Enter => self.on_enter,
            Event::Move(point) => self.on_move.map(|handler| handler(point)),
            Event::Exit => self.on_exit,
        }
    }
}

impl<Msg: std::fmt::Debug> std::fmt::Debug for Callbacks<Msg> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Callbacks")
            .field("on_press", &self.on_press)
            .field("on_release", &self.on_release)
            .field("on_double_click", &self.on_double_click)
            .field("on_right_press", &self.on_right_press)
            .field("on_right_release", &self.on_right_release)
            .field("on_middle_press", &self.on_middle_press)
            .field("on_middle_release", &self.on_middle_release)
            .field(
                "on_scroll",
                &self
                    .on_scroll
                    .as_ref()
                    .map_or("None", |_| "Some(OnScrollHandler)"),
            )
            .field("on_enter", &self.on_enter)
            .field(
                "on_move",
                &self
                    .on_move
                    .as_ref()
                    .map_or("None", |_| "Some(OnMoveHandler)"),
            )
            .field("on_exit", &self.on_exit)
            .finish()
    }
}

impl<Msg: PartialEq> PartialEq for Callbacks<Msg> {
    fn eq(&self, other: &Self) -> bool {
        let on_scroll_eq = match (&self.on_scroll, &other.on_scroll) {
            (Some(lhs), Some(rhs)) => Arc::ptr_eq(lhs, rhs),
            (None, None) => true,
            _ => false,
        };

        let on_move_eq = match (&self.on_move, &other.on_move) {
            (Some(lhs), Some(rhs)) => Arc::ptr_eq(lhs, rhs),
            (None, None) => true,
            _ => false,
        };

        self.on_press == other.on_press
            && self.on_release == other.on_release
            && self.on_double_click == other.on_double_click
            && self.on_right_press == other.on_right_press
            && self.on_right_release == other.on_right_release
            && self.on_middle_press == other.on_middle_press
            && self.on_middle_release == other.on_middle_release
            && on_scroll_eq
            && self.on_enter == other.on_enter
            && on_move_eq
            && self.on_exit == other.on_exit
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ScrollDelta {
    Lines { x: f32, y: f32 },
    Pixels { x: f32, y: f32 },
}

impl From<widget::v1::mouse_area::ScrollEvent> for ScrollDelta {
    fn from(value: widget::v1::mouse_area::ScrollEvent) -> Self {
        use widget::v1::mouse_area::scroll_event::{Data, Lines, Pixels};

        let data = value.data.expect("ScrollEvent without data");

        match data {
            Data::Lines(Lines { x, y }) => Self::Lines { x, y },
            Data::Pixels(Pixels { x, y }) => Self::Pixels { x, y },
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Point {
    x: f32,
    y: f32,
}

impl From<widget::v1::mouse_area::MoveEvent> for Point {
    fn from(value: widget::v1::mouse_area::MoveEvent) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

impl From<widget::v1::mouse_area::Event> for Event {
    fn from(value: widget::v1::mouse_area::Event) -> Self {
        use widget::v1::mouse_area::event::Data;

        let data = value.data.expect("Event withoud data");

        match data {
            Data::Press(()) => Self::Press,
            Data::Release(()) => Self::Release,
            Data::DoubleClick(()) => Self::DoubleClick,
            Data::RightPress(()) => Self::RightPress,
            Data::RightRelease(()) => Self::RightRelease,
            Data::MiddlePress(()) => Self::MiddlePress,
            Data::MiddleRelease(()) => Self::MiddleRelease,
            Data::Scroll(scroll_event) => Self::Scroll(scroll_event.into()),
            Data::Enter(()) => Self::Enter,
            Data::Move(move_event) => Self::Move(move_event.into()),
            Data::Exit(()) => Self::Exit,
        }
    }
}
