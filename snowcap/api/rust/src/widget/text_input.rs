//! TextInput widget

use std::sync::Arc;

use snowcap_api_defs::snowcap::widget;

use crate::widget::{
    Alignment, Background, Border, Color, Length, LineHeight, Padding, font::Font,
};

use super::{Widget, WidgetId};

/// A field that can be filled with text.
#[derive(Clone)]
pub struct TextInput<Msg> {
    pub placeholder: String,
    pub value: String,
    pub id: Option<String>,
    pub secure: bool,
    pub font: Option<Font>,
    pub icon: Option<Icon>,
    pub width: Option<Length>,
    pub padding: Option<Padding>,
    pub line_height: Option<LineHeight>,
    pub horizontal_alignment: Option<Alignment>,
    pub style: Option<Styles>,
    pub(crate) callbacks: Callbacks<Msg>,
    pub(crate) widget_id: Option<WidgetId>,
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
            icon: None,
            width: None,
            padding: None,
            line_height: None,
            horizontal_alignment: None,
            style: None,
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

    pub fn icon(self, icon: Icon) -> Self {
        Self {
            icon: Some(icon),
            ..self
        }
    }

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

    pub fn line_height(self, line_height: LineHeight) -> Self {
        Self {
            line_height: Some(line_height),
            ..self
        }
    }

    pub fn horizontal_alignment(self, horizontal_alignment: Alignment) -> Self {
        Self {
            horizontal_alignment: Some(horizontal_alignment),
            ..self
        }
    }

    pub fn style(self, style: Styles) -> Self {
        Self {
            style: Some(style),
            ..self
        }
    }
}

impl<Msg: std::fmt::Debug> std::fmt::Debug for TextInput<Msg> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextInput")
            .field("placeholder", &self.placeholder)
            .field("value", &self.value)
            .field("id", &self.id)
            .field("secure", &self.secure)
            .field("font", &self.font)
            .field("icon", &self.icon)
            .field("width", &self.width)
            .field("padding", &self.padding)
            .field("line_height", &self.line_height)
            .field("horizontal_alignment", &self.horizontal_alignment)
            .field("style", &self.style)
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
            && self.icon == other.icon
            && self.width == other.width
            && self.padding == other.padding
            && self.line_height == other.line_height
            && self.horizontal_alignment == other.horizontal_alignment
            && self.style == other.style
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
            icon,
            width,
            padding,
            line_height,
            horizontal_alignment,
            style,
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
            icon: icon.map(From::from),
            width: width.map(From::from),
            padding: padding.map(From::from),
            line_height: line_height.map(From::from),
            horizontal_alignment: None,
            style: style.map(From::from),
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

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Side {
    Left,
    Right,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Icon {
    pub font: Font,
    pub code_point: char,
    pub pixels: Option<f32>,
    pub spacing: f32,
    pub side: Side,
}

impl From<Icon> for widget::v1::text_input::Icon {
    fn from(value: Icon) -> Self {
        let Icon {
            font,
            code_point,
            pixels,
            spacing,
            side,
        } = value;

        Self {
            font: Some(font.into()),
            code_point: code_point.into(),
            pixels,
            spacing,
            right_side: matches!(side, Side::Right),
        }
    }
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct Styles {
    pub active: Option<Style>,
    pub hovered: Option<Style>,
    pub focused: Option<Style>,
    pub hover_focused: Option<Style>,
    pub disabled: Option<Style>,
}

impl Styles {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn active(self, style: Style) -> Self {
        Self {
            active: Some(style),
            ..self
        }
    }

    pub fn hovered(self, style: Style) -> Self {
        Self {
            hovered: Some(style),
            ..self
        }
    }

    pub fn focused(self, style: Style) -> Self {
        Self {
            focused: Some(style),
            ..self
        }
    }

    pub fn hover_focused(self, style: Style) -> Self {
        Self {
            hover_focused: Some(style),
            ..self
        }
    }

    pub fn disabled(self, style: Style) -> Self {
        Self {
            disabled: Some(style),
            ..self
        }
    }
}

impl From<Styles> for widget::v1::text_input::Style {
    fn from(value: Styles) -> Self {
        let Styles {
            active,
            hovered,
            focused,
            hover_focused,
            disabled,
        } = value;

        Self {
            active: active.map(From::from),
            hovered: hovered.map(From::from),
            focused: focused.map(From::from),
            hover_focused: hover_focused.map(From::from),
            disabled: disabled.map(From::from),
        }
    }
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct Style {
    pub background: Option<Background>,
    pub border: Option<Border>,
    pub icon: Option<Color>,
    pub placeholder: Option<Color>,
    pub value: Option<Color>,
    pub selection: Option<Color>,
}

impl Style {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn background(self, background: Background) -> Self {
        Self {
            background: Some(background),
            ..self
        }
    }

    pub fn border(self, border: Border) -> Self {
        Self {
            border: Some(border),
            ..self
        }
    }

    pub fn icon(self, color: Color) -> Self {
        Self {
            icon: Some(color),
            ..self
        }
    }

    pub fn placeholder(self, color: Color) -> Self {
        Self {
            placeholder: Some(color),
            ..self
        }
    }

    pub fn value(self, color: Color) -> Self {
        Self {
            value: Some(color),
            ..self
        }
    }

    pub fn selection(self, color: Color) -> Self {
        Self {
            selection: Some(color),
            ..self
        }
    }
}

impl From<Style> for widget::v1::text_input::style::Inner {
    fn from(value: Style) -> Self {
        let Style {
            background,
            border,
            icon,
            placeholder,
            value,
            selection,
        } = value;

        Self {
            background: background.map(From::from),
            border: border.map(From::from),
            icon: icon.map(From::from),
            placeholder: placeholder.map(From::from),
            value: value.map(From::from),
            selection: selection.map(From::from),
        }
    }
}
