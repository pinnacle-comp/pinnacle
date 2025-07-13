use snowcap_api_defs::snowcap::widget;

use super::{Border, Color, Length, Widget, WidgetDef, container};

#[derive(Debug, Clone, PartialEq)]
pub struct Scrollable<Msg> {
    pub width: Option<Length>,
    pub height: Option<Length>,
    pub direction: Option<Direction>,
    pub child: WidgetDef<Msg>,
    pub style: Option<Style>,
}

impl<Msg> From<Scrollable<Msg>> for Widget<Msg> {
    fn from(value: Scrollable<Msg>) -> Self {
        Self::Scrollable(Box::new(value))
    }
}

impl<Msg> From<Scrollable<Msg>> for widget::v1::Scrollable {
    fn from(value: Scrollable<Msg>) -> Self {
        widget::v1::Scrollable {
            width: value.width.map(From::from),
            height: value.height.map(From::from),
            direction: value.direction.map(From::from),
            child: Some(Box::new(value.child.into())),
            style: value.style.map(From::from),
        }
    }
}

impl<Msg> Scrollable<Msg> {
    pub fn new(child: impl Into<WidgetDef<Msg>>) -> Self {
        Self {
            child: child.into(),
            width: None,
            height: None,
            direction: None,
            style: None,
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

    pub fn direction(self, direction: Direction) -> Self {
        Self {
            direction: Some(direction),
            ..self
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    Vertical(Scrollbar),
    Horizontal(Scrollbar),
    Both {
        vertical: Scrollbar,
        horizontal: Scrollbar,
    },
}

impl From<Direction> for widget::v1::scrollable::Direction {
    fn from(value: Direction) -> Self {
        match value {
            Direction::Vertical(props) => widget::v1::scrollable::Direction {
                vertical: Some(props.into()),
                horizontal: None,
            },
            Direction::Horizontal(props) => widget::v1::scrollable::Direction {
                vertical: None,
                horizontal: Some(props.into()),
            },
            Direction::Both {
                vertical,
                horizontal,
            } => widget::v1::scrollable::Direction {
                vertical: Some(vertical.into()),
                horizontal: Some(horizontal.into()),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Alignment {
    #[default]
    Start,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Scrollbar {
    pub width: Option<f32>,
    pub margin: Option<f32>,
    pub scroller_width: Option<f32>,
    pub alignment: Option<Alignment>,
    pub embed_spacing: Option<f32>,
}

impl From<Scrollbar> for widget::v1::scrollable::Scrollbar {
    fn from(value: Scrollbar) -> Self {
        widget::v1::scrollable::Scrollbar {
            width_pixels: value.width,
            margin_pixels: value.margin,
            scroller_width_pixels: value.scroller_width,
            anchor_to_end: value.alignment.map(|align| match align {
                Alignment::Start => false,
                Alignment::End => true,
            }),
            embed_spacing: value.embed_spacing,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Style {
    pub container_style: Option<container::Style>,
    pub vertical_rail: Option<Rail>,
    pub horizontal_rail: Option<Rail>,
}

impl From<Style> for widget::v1::scrollable::Style {
    fn from(value: Style) -> Self {
        Self {
            container_style: value.container_style.map(From::from),
            vertical_rail: value.vertical_rail.map(From::from),
            horizontal_rail: value.horizontal_rail.map(From::from),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rail {
    pub background_color: Option<Color>,
    pub border: Option<Border>,
    pub scroller_color: Option<Color>,
    pub scroller_border: Option<Border>,
}

impl From<Rail> for widget::v1::scrollable::Rail {
    fn from(value: Rail) -> Self {
        Self {
            background_color: value.background_color.map(From::from),
            border: value.border.map(From::from),
            scroller_color: value.scroller_color.map(From::from),
            scroller_border: value.scroller_border.map(From::from),
        }
    }
}
