//! Touch Event handling

use std::sync::Arc;

use snowcap_api_defs::snowcap::widget;

use crate::widget::{Widget, WidgetDef, WidgetId};

pub use super::Point;

/// Emits messages on touch events.
#[derive(Debug, Clone, PartialEq)]
pub struct TouchArea<Msg> {
    pub child: WidgetDef<Msg>,
    pub(crate) widget_id: Option<WidgetId>,
    pub(crate) callbacks: Callbacks<Msg>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Finger {
    pub id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Down(Finger, Point),
    Up(Finger),
    Enter(Finger),
    Move(Finger, Point),
    Exit(Finger),
    Cancel(Finger),
}

#[derive(Clone)]
pub struct Callbacks<Msg> {
    pub(crate) on_down: Option<Arc<dyn Fn(Finger, Point) -> Msg + Sync + Send>>,
    pub(crate) on_up: Option<Arc<dyn Fn(Finger) -> Msg + Sync + Send>>,
    pub(crate) on_enter: Option<Arc<dyn Fn(Finger) -> Msg + Sync + Send>>,
    pub(crate) on_move: Option<Arc<dyn Fn(Finger, Point) -> Msg + Sync + Send>>,
    pub(crate) on_exit: Option<Arc<dyn Fn(Finger) -> Msg + Sync + Send>>,
    pub(crate) on_cancel: Option<Arc<dyn Fn(Finger) -> Msg + Sync + Send>>,
}

impl<Msg> TouchArea<Msg> {
    /// Create a [`TouchArea`] with the given content.
    pub fn new(child: impl Into<WidgetDef<Msg>>) -> Self {
        Self {
            child: child.into(),
            widget_id: None,
            callbacks: Callbacks {
                on_down: None,
                on_up: None,
                on_enter: None,
                on_move: None,
                on_exit: None,
                on_cancel: None,
            },
        }
    }

    /// Message to emit on a finger press.
    pub fn on_down<F>(self, on_down: F) -> Self
    where
        F: Fn(Finger, Point) -> Msg + Sync + Send + 'static,
    {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            callbacks: Callbacks {
                on_down: Some(Arc::new(on_down)),
                ..self.callbacks
            },
            ..self
        }
    }

    /// Message to emit when a finger is lifted.
    pub fn on_up<F>(self, on_up: F) -> Self
    where
        F: Fn(Finger) -> Msg + Sync + Send + 'static,
    {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            callbacks: Callbacks {
                on_up: Some(Arc::new(on_up)),
                ..self.callbacks
            },
            ..self
        }
    }

    /// Message to emit when a finger enter the area.
    pub fn on_enter<F>(self, on_enter: F) -> Self
    where
        F: Fn(Finger) -> Msg + Sync + Send + 'static,
    {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            callbacks: Callbacks {
                on_enter: Some(Arc::new(on_enter)),
                ..self.callbacks
            },
            ..self
        }
    }

    /// Message to emit when a finger moves on the area.
    pub fn on_move<F>(self, on_move: F) -> Self
    where
        F: Fn(Finger, Point) -> Msg + Sync + Send + 'static,
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

    /// Message to emit when a finger leaves the area.
    pub fn on_exit<F>(self, on_exit: F) -> Self
    where
        F: Fn(Finger) -> Msg + Sync + Send + 'static,
    {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            callbacks: Callbacks {
                on_exit: Some(Arc::new(on_exit)),
                ..self.callbacks
            },
            ..self
        }
    }

    /// Message to emit when the touch stream is cancelled.
    pub fn on_cancel<F>(self, on_cancel: F) -> Self
    where
        F: Fn(Finger) -> Msg + Sync + Send + 'static,
    {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            callbacks: Callbacks {
                on_cancel: Some(Arc::new(on_cancel)),
                ..self.callbacks
            },
            ..self
        }
    }
}

impl<Msg> From<TouchArea<Msg>> for Widget<Msg> {
    fn from(value: TouchArea<Msg>) -> Self {
        Widget::TouchArea(Box::new(value))
    }
}

impl<Msg> From<TouchArea<Msg>> for widget::v1::TouchArea {
    fn from(value: TouchArea<Msg>) -> Self {
        Self {
            child: Some(Box::new(value.child.into())),
            widget_id: value.widget_id.map(WidgetId::to_inner),
            on_down: value.callbacks.on_down.is_some(),
            on_up: value.callbacks.on_up.is_some(),
            on_enter: value.callbacks.on_enter.is_some(),
            on_move: value.callbacks.on_move.is_some(),
            on_exit: value.callbacks.on_exit.is_some(),
            on_cancel: value.callbacks.on_cancel.is_some(),
        }
    }
}

impl<Msg> Callbacks<Msg> {
    pub fn process_event(self, evt: Event) -> Option<Msg> {
        match evt {
            Event::Down(finger, point) => self.on_down.map(|handler| handler(finger, point)),
            Event::Up(finger) => self.on_up.map(|handler| handler(finger)),
            Event::Enter(finger) => self.on_enter.map(|handler| handler(finger)),
            Event::Move(finger, point) => self.on_move.map(|handler| handler(finger, point)),
            Event::Exit(finger) => self.on_exit.map(|handler| handler(finger)),
            Event::Cancel(finger) => self.on_cancel.map(|handler| handler(finger)),
        }
    }
}

impl<Msg> std::fmt::Debug for Callbacks<Msg> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Callbacks")
            .field(
                "on_down",
                &self
                    .on_down
                    .as_ref()
                    .map_or("None", |_| "Some(OnDownHandler)"),
            )
            .field(
                "on_up",
                &self.on_up.as_ref().map_or("None", |_| "Some(OnUpHandler)"),
            )
            .field(
                "on_enter",
                &self
                    .on_enter
                    .as_ref()
                    .map_or("None", |_| "Some(OnEnterHandler)"),
            )
            .field(
                "on_move",
                &self
                    .on_move
                    .as_ref()
                    .map_or("None", |_| "Some(OnMoveHandler)"),
            )
            .field(
                "on_exit",
                &self
                    .on_exit
                    .as_ref()
                    .map_or("None", |_| "Some(OnExitHandler)"),
            )
            .field(
                "on_cancel",
                &self
                    .on_cancel
                    .as_ref()
                    .map_or("None", |_| "Some(OnCancelHandler)"),
            )
            .finish()
    }
}

impl<Msg> PartialEq for Callbacks<Msg> {
    fn eq(&self, other: &Self) -> bool {
        fn compare<T: ?Sized>(lhs: &Option<Arc<T>>, rhs: &Option<Arc<T>>) -> bool {
            match (lhs, rhs) {
                (Some(lhs), Some(rhs)) => Arc::ptr_eq(lhs, rhs),
                (None, None) => true,
                _ => false,
            }
        }

        compare(&self.on_down, &other.on_down)
            && compare(&self.on_up, &other.on_up)
            && compare(&self.on_enter, &other.on_enter)
            && compare(&self.on_move, &other.on_move)
            && compare(&self.on_exit, &other.on_exit)
            && compare(&self.on_cancel, &other.on_cancel)
    }
}

impl From<widget::v1::touch_area::Finger> for Finger {
    fn from(value: widget::v1::touch_area::Finger) -> Self {
        Self { id: value.id }
    }
}

impl From<widget::v1::touch_area::Point> for Point {
    fn from(value: widget::v1::touch_area::Point) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

impl From<widget::v1::touch_area::Event> for Event {
    fn from(value: widget::v1::touch_area::Event) -> Self {
        use widget::v1::touch_area::{DownEvent, MoveEvent, event::Data};

        let data = value.data.expect("Event without data");

        match data {
            Data::Down(DownEvent { finger, point }) => Self::Down(
                finger
                    .expect("DownEvent should hold finger information")
                    .into(),
                point
                    .expect("DownEvent should hold location information")
                    .into(),
            ),
            Data::Up(finger) => Self::Up(finger.into()),
            Data::Enter(finger) => Self::Enter(finger.into()),
            Data::Move(MoveEvent { finger, point }) => Self::Move(
                finger
                    .expect("MoveEvent should hold finger information")
                    .into(),
                point
                    .expect("MoveEvent should hold location information")
                    .into(),
            ),
            Data::Exit(finger) => Self::Exit(finger.into()),
            Data::Cancel(finger) => Self::Cancel(finger.into()),
        }
    }
}
