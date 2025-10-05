//! TextInput widget

use std::sync::Arc;

use snowcap_api_defs::snowcap::widget;

use crate::widget::{Alignment, Length, Padding, font::Font};

use super::{Widget, WidgetId};

/// A field that can be filled with text.
#[derive(Clone)]
pub struct TextInput<Msg> {
    pub(crate) placeholder: String,
    pub(crate) value: String,
    pub(crate) id: Option<String>,
    pub(crate) secure: bool,
    pub(crate) font: Option<Font>,
    //pub(crate) icon: Option<Icon>,
    pub(crate) width: Option<Length>,
    pub(crate) padding: Option<Padding>,
    //pub(crate) line_height: Option<LineHeight>,
    pub(crate) horizontal_alignment: Option<Alignment>,
    //pub(crate) style: Option<Style>,
    pub(crate) widget_id: Option<WidgetId>,
    pub(crate) callbacks: Callbacks<Msg>,
}

impl<Msg> TextInput<Msg> {
    pub fn new(placeholder: &str, value: &str) -> Self {
        let placeholder = placeholder.into();
        let value = value.into();

        Self {
            placeholder,
            value,
            id: None,
            secure: false,
            font: None,
            // icon: None,
            width: None,
            padding: None,
            // line_height: None
            horizontal_alignment: None,
            // style: None,
            widget_id: None,
            callbacks: Callbacks {
                on_input: None,
                on_submit: None,
                on_paste: None,
            },
        }
    }

    pub fn id(self, id: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            ..self
        }
    }

    pub fn secure(self, is_secure: bool) -> Self {
        Self {
            secure: is_secure,
            ..self
        }
    }

    pub fn on_input<F>(self, on_input: F) -> Self
    where
        F: Fn(String) -> Msg + Sync + Send + 'static,
    {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            callbacks: Callbacks {
                on_input: Some(Arc::new(on_input)),
                ..self.callbacks
            },
            ..self
        }
    }

    pub fn on_submit(self, on_submit: Msg) -> Self {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            callbacks: Callbacks {
                on_submit: Some(on_submit),
                ..self.callbacks
            },
            ..self
        }
    }

    pub fn on_paste<F>(self, on_paste: F) -> Self
    where
        F: Fn(String) -> Msg + Sync + Send + 'static,
    {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            callbacks: Callbacks {
                on_paste: Some(Arc::new(on_paste)),
                ..self.callbacks
            },
            ..self
        }
    }

    pub fn font(self, font: Font) -> Self {
        Self {
            font: Some(font),
            ..self
        }
    }

    //pub fn icon(self, icon: Icon) -> Self {
    //    Self {
    //        icon: Some(icon),
    //        ..self
    //    }
    //}

    pub fn width(self, width: Length) -> Self {
        Self {
            width: Some(width),
            ..self
        }
    }

    pub fn padding(self, padding: Padding) -> Self {
        Self {
            padding: Some(padding),
            ..self
        }
    }

    //pub fn line_height(self, line_height: LineHeight) -> Self {
    //    Self {
    //        line_height: Some(line_height),
    //        ..self
    //    }
    //}

    pub fn horizontal_alignment(self, horizontal_alignment: Alignment) -> Self {
        Self {
            horizontal_alignment: Some(horizontal_alignment),
            ..self
        }
    }

    //pub fn style(self, style: Style) -> Self {
    //    Self {
    //        style: Some(style),
    //        ..self
    //    }
    //}
}

impl<Msg: std::fmt::Debug> std::fmt::Debug for TextInput<Msg> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextInput")
            .field("placeholder", &self.placeholder)
            .field("value", &self.value)
            .field("id", &self.id)
            .field("secure", &self.secure)
            .field("font", &self.font)
            //.field("icon", &self.icon)
            .field("width", &self.width)
            .field("padding", &self.padding)
            //.field("line_height", &self.line_height)
            .field("horizontal_alignment", &self.horizontal_alignment)
            //.field("style", &self.field)
            .field("widget_id", &self.widget_id)
            .field("callbacks", &self.callbacks)
            .finish()
    }
}

impl<Msg: PartialEq> PartialEq for TextInput<Msg> {
    fn eq(&self, other: &Self) -> bool {
        self.placeholder == other.placeholder
            && self.value == other.value
            && self.id == other.id
            && self.secure == other.secure
            && self.font == other.font
            //&& self.icon == other.icon
            && self.width == other.width
            && self.padding == other.padding
            //&& self.line_height == other.line_height
            && self.horizontal_alignment == other.horizontal_alignment
            //&& self.style == other.style
            && self.widget_id == other.widget_id
            && self.callbacks == other.callbacks
    }
}

impl<Msg> From<TextInput<Msg>> for Widget<Msg> {
    fn from(value: TextInput<Msg>) -> Self {
        Widget::TextInput(Box::new(value))
    }
}

impl<Msg> From<TextInput<Msg>> for widget::v1::TextInput {
    fn from(value: TextInput<Msg>) -> Self {
        let TextInput {
            placeholder,
            value,
            id,
            secure,
            font,
            width,
            padding,
            horizontal_alignment,
            widget_id,
            callbacks:
                Callbacks {
                    on_input,
                    on_submit,
                    on_paste,
                },
        } = value;

        let mut text_input = Self {
            placeholder,
            value,
            id,
            secure,
            on_input: on_input.is_some(),
            on_submit: on_submit.is_some(),
            on_paste: on_paste.is_some(),
            font: font.map(From::from),
            icon: None,
            width: width.map(From::from),
            padding: padding.map(From::from),
            line_height: None,
            horizontal_alignment: None,
            style: None,
            widget_id: widget_id.map(WidgetId::to_inner),
        };

        if let Some(horizontal_alignment) = horizontal_alignment {
            text_input.set_horizontal_alignment(horizontal_alignment.into());
        }

        text_input
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Event {
    Input(String),
    Submit,
    Paste(String),
}

impl From<widget::v1::text_input::Event> for Event {
    fn from(value: widget::v1::text_input::Event) -> Self {
        use widget::v1::text_input::EventType;

        let widget::v1::text_input::Event { event_type, data } = value;

        let event_type = event_type.try_into().expect("Invalid EventType");
        match event_type {
            EventType::EventInput => Self::Input(data.unwrap_or_default()),
            EventType::EventSubmit => Self::Submit,
            EventType::EventPaste => Self::Paste(data.unwrap_or_default()),
        }
    }
}

#[derive(Clone)]
pub struct Callbacks<Msg> {
    pub(crate) on_input: Option<Arc<dyn Fn(String) -> Msg + Sync + Send>>,
    pub(crate) on_submit: Option<Msg>,
    pub(crate) on_paste: Option<Arc<dyn Fn(String) -> Msg + Sync + Send>>,
}

impl<Msg> Callbacks<Msg> {
    pub(crate) fn process_event(self, evt: Event) -> Option<Msg> {
        match evt {
            Event::Input(data) => self.on_input.map(|handler| handler(data)),
            Event::Submit => self.on_submit,
            Event::Paste(data) => self.on_paste.map(|handler| handler(data)),
        }
    }
}

impl<Msg: std::fmt::Debug> std::fmt::Debug for Callbacks<Msg> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Callbacks")
            .field(
                "on_input",
                &self
                    .on_input
                    .as_ref()
                    .map_or("None", |_| "Some(OnInputHandler)"),
            )
            .field("on_submit", &self.on_submit)
            .field(
                "on_paste",
                &self
                    .on_paste
                    .as_ref()
                    .map_or("None", |_| "Some(OnPasteHandler)"),
            )
            .finish()
    }
}

impl<Msg: PartialEq> PartialEq for Callbacks<Msg> {
    fn eq(&self, other: &Self) -> bool {
        let on_input_eq = match (&self.on_input, &other.on_input) {
            (Some(lhs), Some(rhs)) => Arc::ptr_eq(lhs, rhs),
            (None, None) => true,
            _ => false,
        };

        let on_paste_eq = match (&self.on_paste, &other.on_paste) {
            (Some(lhs), Some(rhs)) => Arc::ptr_eq(lhs, rhs),
            (None, None) => true,
            _ => false,
        };

        on_input_eq && self.on_submit == other.on_submit && on_paste_eq
    }
}
