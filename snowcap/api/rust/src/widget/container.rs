use snowcap_api_defs::snowcap::widget;

use crate::widget::Background;

use super::{Alignment, Border, Color, Length, Padding, Widget, WidgetDef};

#[derive(Debug, Clone, PartialEq)]
pub struct Container<Msg> {
    pub padding: Option<Padding>,
    pub width: Option<Length>,
    pub height: Option<Length>,
    pub max_width: Option<f32>,
    pub max_height: Option<f32>,
    pub horizontal_alignment: Option<Alignment>,
    pub vertical_alignment: Option<Alignment>,
    pub clip: Option<bool>,
    pub child: WidgetDef<Msg>,
    pub style: Option<Style>,
}

impl<Msg> Container<Msg> {
    pub fn new(child: impl Into<WidgetDef<Msg>>) -> Self {
        Self {
            child: child.into(),
            padding: None,
            width: None,
            height: None,
            max_width: None,
            max_height: None,
            horizontal_alignment: None,
            vertical_alignment: None,
            clip: None,
            style: None,
        }
    }

    pub fn padding(self, padding: Padding) -> Self {
        Self {
            padding: Some(padding),
            ..self
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

    pub fn max_width(self, max_width: f32) -> Self {
        Self {
            max_width: Some(max_width),
            ..self
        }
    }

    pub fn max_height(self, max_height: f32) -> Self {
        Self {
            max_height: Some(max_height),
            ..self
        }
    }

    pub fn horizontal_alignment(self, horizontal_alignment: Alignment) -> Self {
        Self {
            horizontal_alignment: Some(horizontal_alignment),
            ..self
        }
    }

    pub fn vertical_alignment(self, vertical_alignment: Alignment) -> Self {
        Self {
            vertical_alignment: Some(vertical_alignment),
            ..self
        }
    }

    pub fn clip(self, clip: bool) -> Self {
        Self {
            clip: Some(clip),
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

impl<Msg> From<Container<Msg>> for widget::v1::Container {
    fn from(value: Container<Msg>) -> Self {
        Self {
            padding: value.padding.map(From::from),
            width: value.width.map(From::from),
            height: value.height.map(From::from),
            max_width: value.max_width,
            max_height: value.max_height,
            horizontal_alignment: value
                .horizontal_alignment
                .map(|it| widget::v1::Alignment::from(it) as i32),
            vertical_alignment: value
                .vertical_alignment
                .map(|it| widget::v1::Alignment::from(it) as i32),
            clip: value.clip,
            child: Some(Box::new(value.child.into())),
            style: value.style.map(From::from),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Style {
    pub text_color: Option<Color>,
    pub border: Option<Border>,
    pub background: Option<Background>,
}

impl Style {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn text_color(self, color: Color) -> Self {
        Self {
            text_color: Some(color),
            ..self
        }
    }

    pub fn border(self, border: Border) -> Self {
        Self {
            border: Some(border),
            ..self
        }
    }

    pub fn background(self, background: Background) -> Self {
        Self {
            background: Some(background),
            ..self
        }
    }
}

impl From<Style> for widget::v1::container::Style {
    fn from(value: Style) -> Self {
        Self {
            text_color: value.text_color.map(From::from),
            border: value.border.map(From::from),
            background: value.background.map(From::from),
            ..Default::default()
        }
    }
}

impl<Msg> From<Container<Msg>> for Widget<Msg> {
    fn from(value: Container<Msg>) -> Self {
        Self::Container(Box::new(value))
    }
}
