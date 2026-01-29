use snowcap_api_defs::snowcap::widget;

use super::{Alignment, Color, Length, Wrapping, font::Font};

/// A text widget definition.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Text {
    pub text: String,
    pub width: Option<Length>,
    pub height: Option<Length>,
    pub horizontal_alignment: Option<Alignment>,
    pub vertical_alignment: Option<Alignment>,
    pub wrapping: Option<Wrapping>,
    pub style: Option<Style>,
}

impl Text {
    pub fn new(text: impl ToString) -> Self {
        Self {
            text: text.to_string(),
            ..Default::default()
        }
    }

    pub fn width(self, width: Length) -> Self {
        Self {
            width: Some(width),
            ..self
        }
    }

    pub fn height(self, height: Length) -> Self {
        Self {
            height: Some(height),
            ..self
        }
    }

    pub fn horizontal_alignment(self, alignment: Alignment) -> Self {
        Self {
            horizontal_alignment: Some(alignment),
            ..self
        }
    }

    pub fn vertical_alignment(self, alignment: Alignment) -> Self {
        Self {
            vertical_alignment: Some(alignment),
            ..self
        }
    }

    pub fn wrapping(self, wrapping: Wrapping) -> Self {
        Self {
            wrapping: Some(wrapping),
            ..self
        }
    }

    pub fn style(self, style: Style) -> Self {
        Self {
            style: Some(style),
            ..self
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Style {
    pub color: Option<Color>,
    pub pixels: Option<f32>,
    pub font: Option<Font>,
}

impl Style {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn color(self, color: Color) -> Self {
        Self {
            color: Some(color),
            ..self
        }
    }

    pub fn pixels(self, pixels: f32) -> Self {
        Self {
            pixels: Some(pixels),
            ..self
        }
    }

    pub fn font(self, font: Font) -> Self {
        Self {
            font: Some(font),
            ..self
        }
    }
}

impl From<Text> for widget::v1::Text {
    fn from(value: Text) -> Self {
        let mut text = widget::v1::Text {
            text: value.text,
            width: value.width.map(From::from),
            height: value.height.map(From::from),
            horizontal_alignment: None,
            vertical_alignment: None,
            wrapping: None,
            style: value.style.map(From::from),
        };

        if let Some(horizontal_alignment) = value.horizontal_alignment {
            text.set_horizontal_alignment(horizontal_alignment.into());
        }
        if let Some(vertical_alignment) = value.vertical_alignment {
            text.set_vertical_alignment(vertical_alignment.into());
        }
        if let Some(wrapping) = value.wrapping {
            text.set_wrapping(wrapping.into());
        }
        text
    }
}

impl From<Style> for widget::v1::text::Style {
    fn from(value: Style) -> Self {
        Self {
            color: value.color.map(From::from),
            pixels: value.pixels,
            font: value.font.map(From::from),
        }
    }
}
