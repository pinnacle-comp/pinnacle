//! TextInput display fields that can be filled with text.
//!
//! # Example
//!
//! Create a simple Layer with an automatically focused [`TextInput`]:
//!
//! ```
//! use snowcap_api::{
//!     layer,
//!     widget::{self, container::Container, operation, text_input::TextInput, Length, Program},
//! };
//!
//! /// Example Program for [`TextInput`]
//! #[derive(Default)]
//! pub struct TextInputProgram {
//!     input_value: String
//! }
//!
//! /// Messages for [`TextInputProgram`]
//! #[derive(Debug, Clone)]
//! pub enum Message {
//!     /// Something was input or paste in the [`TextInput`].
//!     ContentChanged(String),
//!     /// [`TextInput`] was submitted.
//!     Submit,
//! }
//!
//! impl TextInputProgram {
//!     const INPUT_ID: &str = "prompt";
//!
//!     /// Create a new [`TextInputProgram`].
//!     pub fn new() -> Self {
//!         Default::default()
//!     }
//!
//!     /// Display the [`TextInputProgram`] on a new layer.
//!     pub fn show(self) {
//!         let layer = layer::new_widget(
//!             self,
//!             None,
//!             layer::KeyboardInteractivity::Exclusive,
//!             layer::ExclusiveZone::Respect,
//!             layer::ZLayer::Overlay,
//!         ).unwrap();
//!
//!         /// Focus the input
//!         layer.operate(operation::focusable::focus(Self::INPUT_ID));
//!         layer.on_key_press(|handle, key, _mods| {
//!             use xkbcommon::xkb::Keysym;
//!
//!             if key == Keysym::Escape {
//!                 handle.close();
//!             }
//!
//!             if key == Keysym::i {
//!                 handle.operate(operation::focusable::focus(Self::INPUT_ID));
//!             }
//!         });
//!
//!     }
//! }
//!
//! impl Program for TextInputProgram {
//!     type Message = Message;
//!
//!     fn update(&mut self, msg: Self::Message) {
//!         match msg {
//!             Message::ContentChanged(data) => self.input_value = data,
//!             Message::Submit => {
//!                 // do something with the input_value
//!                 self.input_value.clear();
//!             },
//!         }
//!     }
//!
//!     fn view(&self) -> Option<widget::WidgetDef<Self::Message>> {
//!         let widget = TextInput::new("placeholder:", &self.input_value.clone())
//!             .id(Self::INPUT_ID)
//!             .on_input(Message::ContentChanged)
//!             .on_submit(Message::Submit)
//!             .width(Length::Fixed(220.0));
//!
//!         Some(widget.into())
//!     }
//! }
//! ```

use std::sync::Arc;

use snowcap_api_defs::snowcap::widget;

use crate::widget::{
    Alignment, Background, Border, Color, Length, LineHeight, Padding, font::Font,
};

use super::{Widget, WidgetId};

/// A field that can be filled with text.
#[derive(Debug, Clone, PartialEq)]
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
    /// Create a new TextInput Widget.
    ///
    /// # Parameters
    /// - `placeholder`: Text to display when the field is empty.
    /// - `value`: TextInput content.
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

    /// Set the TextInput Id.
    ///
    /// This id can then be used to target this widget with [`Operations`].
    ///
    /// [`Operations`]: crate::widget::operation::Operation
    pub fn id(self, id: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            ..self
        }
    }

    /// Convert the [`TextInput`] into a secure password input
    pub fn secure(self, is_secure: bool) -> Self {
        Self {
            secure: is_secure,
            ..self
        }
    }

    /// Sets the message that should be produced when some text is typed into the [`TextInput`].
    ///
    /// If the method is not called, the TextInput will be disabled.
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

    /// Sets the message that should be produced when the [`TextInput`] is focused and the enter
    /// key is pressed.
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

    /// Sets the message that should be produced when some text is pasted into the [`TextInput`].
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

    /// Sets the [`Font`] of the [`TextInput`].
    pub fn font(self, font: Font) -> Self {
        Self {
            font: Some(font),
            ..self
        }
    }

    /// Sets the [`Icon`] of the [`TextInput`].
    pub fn icon(self, icon: Icon) -> Self {
        Self {
            icon: Some(icon),
            ..self
        }
    }

    /// Sets the width of the [`TextInput`].
    pub fn width(self, width: Length) -> Self {
        Self {
            width: Some(width),
            ..self
        }
    }

    /// Sets the [`Padding`] of the [`TextInput`].
    pub fn padding(self, padding: Padding) -> Self {
        Self {
            padding: Some(padding),
            ..self
        }
    }

    /// Sets the [`LineHeight`] of the [`TextInput`].
    pub fn line_height(self, line_height: LineHeight) -> Self {
        Self {
            line_height: Some(line_height),
            ..self
        }
    }

    /// Sets the horizontal [`Alignment`] of the [`TextInput`].
    pub fn horizontal_alignment(self, horizontal_alignment: Alignment) -> Self {
        Self {
            horizontal_alignment: Some(horizontal_alignment),
            ..self
        }
    }

    /// Sets the style of the [`TextInput`]
    pub fn style(self, style: Styles) -> Self {
        Self {
            style: Some(style),
            ..self
        }
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
        use widget::v1::text_input::event::Data;

        let data = value.data.expect("Invalid EventType");
        match data {
            Data::Input(data) => Self::Input(data),
            Data::Submit(()) => Self::Submit,
            Data::Paste(data) => Self::Paste(data),
        }
    }
}

/// The [`TextInput`] callbacks.
#[derive(Clone)]
pub struct Callbacks<Msg> {
    /// Message to be sent when some text is typed in the [`TextInput`]
    pub(crate) on_input: Option<Arc<dyn Fn(String) -> Msg + Sync + Send>>,
    /// Message to be sent when enter is pressed while the [`TextInput`] is focused.
    pub(crate) on_submit: Option<Msg>,
    /// Message to be sent when some text is pasted in the [`TextInput`]
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

/// The side of a [`TextInput`]
#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub enum Side {
    /// The left side of a [`TextInput`],
    #[default]
    Left,
    /// The right side of a [`TextInput`],
    Right,
}

/// The content of the [`Icon`].
#[derive(Default, Clone, PartialEq, Debug)]
pub struct Icon {
    /// The [`Font`] that will be used to display the `code_point`.
    pub font: Font,
    /// The unicode code point that will be used as the icon.
    pub code_point: char,
    /// The font size of the content.
    pub pixels: Option<f32>,
    /// The spacing between the [`Icon`] and the text in a [`TextInput`]
    pub spacing: f32,
    /// The side of a [`TextInput`] the [`Icon`] should be displayed.
    pub side: Side,
}

impl Icon {
    /// Create a new [`Icon`]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the [`Font`] used to display the [`Icon`].
    pub fn font(self, font: Font) -> Self {
        Self { font, ..self }
    }

    /// Sets the [`Icon`]'s unicode character.
    pub fn code_point(self, code_point: char) -> Self {
        Self { code_point, ..self }
    }

    /// Sets the [`Icon`]'s font size, in pixels.
    pub fn pixels(self, pixels: f32) -> Self {
        Self {
            pixels: Some(pixels),
            ..self
        }
    }

    /// Sets the space between the [`Icon`] and the [`TextInput`] content.
    pub fn spacing(self, spacing: f32) -> Self {
        Self { spacing, ..self }
    }

    /// Sets the side of the [`TextInput`] the [`Icon`] should be displayed at.
    pub fn side(self, side: Side) -> Self {
        Self { side, ..self }
    }
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

/// Styles to apply to the [`TextInput`].
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Styles {
    /// Style to use when the [`TextInput`] is active.
    pub active: Option<Style>,
    /// Style to use when the [`TextInput`] is hovered.
    pub hovered: Option<Style>,
    /// Style to use when the [`TextInput`] is focused.
    pub focused: Option<Style>,
    /// Style to use when the [`TextInput`] is focused & hovered.
    pub hover_focused: Option<Style>,
    /// Style to use when the [`TextInput`] is disabled.
    pub disabled: Option<Style>,
}

impl Styles {
    /// Create a new [`Styles`] that doesn't set anything.
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    /// [`Style`] to apply when the [`TextInput`] is active.
    pub fn active(self, style: Style) -> Self {
        Self {
            active: Some(style),
            ..self
        }
    }

    /// [`Style`] to apply when the [`TextInput`] is hovered.
    pub fn hovered(self, style: Style) -> Self {
        Self {
            hovered: Some(style),
            ..self
        }
    }

    /// [`Style`] to apply when the [`TextInput`] is focused.
    pub fn focused(self, style: Style) -> Self {
        Self {
            focused: Some(style),
            ..self
        }
    }

    /// [`Style`] to apply when the [`TextInput`] is focused & hovered.
    pub fn hover_focused(self, style: Style) -> Self {
        Self {
            hover_focused: Some(style),
            ..self
        }
    }

    /// [`Style`] to apply when the [`TextInput`] is disabled.
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

/// Appearance of a [`TextInput`].
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Style {
    /// The [`Background`] style.
    pub background: Option<Background>,
    /// The [`Border`] of the [`TextInput`].
    pub border: Option<Border>,
    /// The [`Color`] of the [`Icon`].
    pub icon: Option<Color>,
    /// The [`Color`] of the placeholder.
    pub placeholder: Option<Color>,
    /// The [`Color`] of the content.
    pub value: Option<Color>,
    /// The [`Color`] to use for the selection's highlight.
    pub selection: Option<Color>,
}

impl Style {
    /// Create a [`Style`] with default values.
    pub fn new() -> Self {
        Default::default()
    }

    /// The [`Background`] style.
    pub fn background(self, background: Background) -> Self {
        Self {
            background: Some(background),
            ..self
        }
    }

    /// The [`Border`] of the [`TextInput`].
    pub fn border(self, border: Border) -> Self {
        Self {
            border: Some(border),
            ..self
        }
    }

    /// The [`Color`] of the [`Icon`].
    pub fn icon(self, color: Color) -> Self {
        Self {
            icon: Some(color),
            ..self
        }
    }

    /// The [`Color`] of the placeholder.
    pub fn placeholder(self, color: Color) -> Self {
        Self {
            placeholder: Some(color),
            ..self
        }
    }

    /// The [`Color`] of the content.
    pub fn value(self, color: Color) -> Self {
        Self {
            value: Some(color),
            ..self
        }
    }

    /// The [`Color`] to use for the selection's highlight.
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
