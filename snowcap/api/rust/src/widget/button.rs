use snowcap_api_defs::snowcap::widget;

use super::{Border, Color, Length, Padding, Widget, WidgetDef, WidgetId};

#[derive(Clone)]
pub struct Button<Msg> {
    pub child: WidgetDef<Msg>,
    pub width: Option<Length>,
    pub height: Option<Length>,
    pub padding: Option<Padding>,
    pub clip: Option<bool>,
    pub style: Option<Styles>,
    pub(crate) on_press: Option<(WidgetId, Msg)>,
}

impl<Msg: std::fmt::Debug> std::fmt::Debug for Button<Msg> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Button")
            .field("child", &self.child)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("padding", &self.padding)
            .field("clip", &self.clip)
            .field("style", &self.style)
            .field("on_press", &"...")
            .finish()
    }
}

impl<Msg: PartialEq> PartialEq for Button<Msg> {
    fn eq(&self, other: &Self) -> bool {
        self.child == other.child
            && self.width == other.width
            && self.height == other.height
            && self.padding == other.padding
            && self.clip == other.clip
            && self.style == other.style
            && self.on_press == other.on_press
    }
}

impl<Msg> Button<Msg> {
    pub fn new(child: impl Into<WidgetDef<Msg>>) -> Self {
        Self {
            child: child.into(),
            width: None,
            height: None,
            padding: None,
            clip: None,
            style: None,
            on_press: None,
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

    pub fn padding(self, padding: Padding) -> Self {
        Self {
            padding: Some(padding),
            ..self
        }
    }

    pub fn clip(self, clip: bool) -> Self {
        Self {
            clip: Some(clip),
            ..self
        }
    }

    pub fn on_press(self, message: Msg) -> Self {
        Self {
            on_press: Some((WidgetId::next(), message)),
            ..self
        }
    }

    pub fn style(self, styles: Styles) -> Self {
        Self {
            style: Some(styles),
            ..self
        }
    }
}

impl<Msg> From<Button<Msg>> for Widget<Msg> {
    fn from(value: Button<Msg>) -> Self {
        Widget::Button(Box::new(value))
    }
}

impl<Msg> From<Button<Msg>> for widget::v1::Button {
    fn from(value: Button<Msg>) -> Self {
        Self {
            child: Some(Box::new(value.child.into())),
            width: value.width.map(From::from),
            height: value.height.map(From::from),
            padding: value.padding.map(From::from),
            clip: value.clip,
            style: value.style.map(From::from),
            widget_id: value.on_press.map(|(id, _)| id.to_inner()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Styles {
    pub active: Option<Style>,
    pub hovered: Option<Style>,
    pub pressed: Option<Style>,
    pub disabled: Option<Style>,
}

impl Styles {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn border(mut self, border: Border) -> Self {
        self.active.get_or_insert_default().border = Some(border);
        self.hovered.get_or_insert_default().border = Some(border);
        self.pressed.get_or_insert_default().border = Some(border);
        self.disabled.get_or_insert_default().border = Some(border);
        self
    }
}

impl From<Styles> for widget::v1::button::Style {
    fn from(value: Styles) -> Self {
        Self {
            active: value.active.map(From::from),
            hovered: value.hovered.map(From::from),
            pressed: value.pressed.map(From::from),
            disabled: value.disabled.map(From::from),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Style {
    pub text_color: Option<Color>,
    pub background_color: Option<Color>,
    pub border: Option<Border>,
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

    pub fn background_color(self, color: Color) -> Self {
        Self {
            background_color: Some(color),
            ..self
        }
    }

    pub fn border(self, border: Border) -> Self {
        Self {
            border: Some(border),
            ..self
        }
    }
}

impl From<Style> for widget::v1::button::style::Inner {
    fn from(value: Style) -> Self {
        Self {
            text_color: value.text_color.map(From::from),
            background_color: value.background_color.map(From::from),
            border: value.border.map(From::from),
        }
    }
}
