//! Widget definitions.

#![allow(missing_docs)] // TODO:

pub mod font;

use font::Font;
use snowcap_api_defs::snowcap::widget;

/// A unique identifier for a widget.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Hash)]
pub struct WidgetId(u32);

impl WidgetId {
    /// Get the raw u32.
    pub fn into_inner(self) -> u32 {
        self.0
    }
}

impl From<u32> for WidgetId {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

/// A widget definition.
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, from_variants::FromVariants)]
pub enum WidgetDef {
    Text(Text),
    Column(Column),
    Row(Row),
    Scrollable(Box<Scrollable>),
    Container(Box<Container>),
}

impl From<Scrollable> for WidgetDef {
    fn from(value: Scrollable) -> Self {
        Self::Scrollable(Box::new(value))
    }
}

impl From<Container> for WidgetDef {
    fn from(value: Container) -> Self {
        Self::Container(Box::new(value))
    }
}

impl From<WidgetDef> for widget::v0alpha1::WidgetDef {
    fn from(value: WidgetDef) -> widget::v0alpha1::WidgetDef {
        widget::v0alpha1::WidgetDef {
            widget: Some(match value {
                WidgetDef::Text(text) => widget::v0alpha1::widget_def::Widget::Text(text.into()),
                WidgetDef::Column(column) => {
                    widget::v0alpha1::widget_def::Widget::Column(column.into())
                }
                WidgetDef::Row(row) => widget::v0alpha1::widget_def::Widget::Row(row.into()),
                WidgetDef::Scrollable(scrollable) => {
                    widget::v0alpha1::widget_def::Widget::Scrollable(Box::new((*scrollable).into()))
                }
                WidgetDef::Container(container) => {
                    widget::v0alpha1::widget_def::Widget::Container(Box::new((*container).into()))
                }
            }),
        }
    }
}

/// A text widget definition.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Text {
    pub text: String,
    pub size: Option<f32>,
    pub width: Option<Length>,
    pub height: Option<Length>,
    pub horizontal_alignment: Option<Alignment>,
    pub vertical_alignment: Option<Alignment>,
    pub color: Option<Color>,
    pub font: Option<Font>,
}

impl Text {
    pub fn new(text: impl ToString) -> Self {
        Self {
            text: text.to_string(),
            ..Default::default()
        }
    }

    pub fn size(self, size: f32) -> Self {
        Self {
            size: Some(size),
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

    pub fn color(self, color: Color) -> Self {
        Self {
            color: Some(color),
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

impl From<Text> for widget::v0alpha1::Text {
    fn from(value: Text) -> Self {
        let mut text = widget::v0alpha1::Text {
            text: Some(value.text),
            pixels: value.size,
            width: value.width.map(From::from),
            height: value.height.map(From::from),
            horizontal_alignment: None,
            vertical_alignment: None,
            color: value.color.map(From::from),
            font: value.font.map(From::from),
        };
        if let Some(horizontal_alignment) = value.horizontal_alignment {
            text.set_horizontal_alignment(horizontal_alignment.into());
        }
        if let Some(vertical_alignment) = value.vertical_alignment {
            text.set_vertical_alignment(vertical_alignment.into());
        }
        text
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Color {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

impl From<[f32; 4]> for Color {
    fn from(value: [f32; 4]) -> Self {
        Self {
            red: value[0],
            blue: value[1],
            green: value[2],
            alpha: value[3],
        }
    }
}

impl From<[f32; 3]> for Color {
    fn from(value: [f32; 3]) -> Self {
        Self {
            red: value[0],
            blue: value[1],
            green: value[2],
            alpha: 1.0,
        }
    }
}

impl From<Color> for widget::v0alpha1::Color {
    fn from(value: Color) -> Self {
        widget::v0alpha1::Color {
            red: Some(value.red),
            green: Some(value.blue),
            blue: Some(value.green),
            alpha: Some(value.alpha),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Column {
    pub spacing: Option<f32>,
    pub padding: Option<Padding>,
    pub item_alignment: Option<Alignment>,
    pub width: Option<Length>,
    pub height: Option<Length>,
    pub max_width: Option<f32>,
    pub clip: Option<bool>,
    pub children: Vec<WidgetDef>,
}

impl Column {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_children(children: impl IntoIterator<Item = WidgetDef>) -> Self {
        Self {
            children: children.into_iter().collect(),
            ..Default::default()
        }
    }

    pub fn spacing(self, spacing: f32) -> Self {
        Self {
            spacing: Some(spacing),
            ..self
        }
    }

    pub fn item_alignment(self, item_alignment: Alignment) -> Self {
        Self {
            item_alignment: Some(item_alignment),
            ..self
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

    pub fn clip(self, clip: bool) -> Self {
        Self {
            clip: Some(clip),
            ..self
        }
    }

    pub fn push(self, child: impl Into<WidgetDef>) -> Self {
        let mut children = self.children;
        children.push(child.into());
        Self { children, ..self }
    }
}

impl From<Column> for widget::v0alpha1::Column {
    fn from(value: Column) -> Self {
        widget::v0alpha1::Column {
            spacing: value.spacing,
            padding: value.padding.map(From::from),
            item_alignment: value
                .item_alignment
                .map(|it| widget::v0alpha1::Alignment::from(it) as i32),
            width: value.width.map(From::from),
            height: value.height.map(From::from),
            max_width: value.max_width,
            clip: value.clip,
            children: value.children.into_iter().map(From::from).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Row {
    pub spacing: Option<f32>,
    pub padding: Option<Padding>,
    pub item_alignment: Option<Alignment>,
    pub width: Option<Length>,
    pub height: Option<Length>,
    pub clip: Option<bool>,
    pub children: Vec<WidgetDef>,
}

impl Row {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_children(children: impl IntoIterator<Item = WidgetDef>) -> Self {
        Self {
            children: children.into_iter().collect(),
            ..Default::default()
        }
    }

    pub fn spacing(self, spacing: f32) -> Self {
        Self {
            spacing: Some(spacing),
            ..self
        }
    }

    pub fn item_alignment(self, item_alignment: Alignment) -> Self {
        Self {
            item_alignment: Some(item_alignment),
            ..self
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

    pub fn clip(self, clip: bool) -> Self {
        Self {
            clip: Some(clip),
            ..self
        }
    }

    pub fn push(self, child: impl Into<WidgetDef>) -> Self {
        let mut children = self.children;
        children.push(child.into());
        Self { children, ..self }
    }
}

impl From<Row> for widget::v0alpha1::Row {
    fn from(value: Row) -> Self {
        widget::v0alpha1::Row {
            spacing: value.spacing,
            padding: value.padding.map(From::from),
            item_alignment: value
                .item_alignment
                .map(|it| widget::v0alpha1::Alignment::from(it) as i32),
            width: value.width.map(From::from),
            height: value.height.map(From::from),
            clip: value.clip,
            children: value.children.into_iter().map(From::from).collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Padding {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl From<Padding> for widget::v0alpha1::Padding {
    fn from(value: Padding) -> Self {
        widget::v0alpha1::Padding {
            top: Some(value.top),
            right: Some(value.right),
            bottom: Some(value.bottom),
            left: Some(value.left),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Alignment {
    #[default]
    Start,
    Center,
    End,
}

impl From<Alignment> for widget::v0alpha1::Alignment {
    fn from(value: Alignment) -> Self {
        match value {
            Alignment::Start => widget::v0alpha1::Alignment::Start,
            Alignment::Center => widget::v0alpha1::Alignment::Center,
            Alignment::End => widget::v0alpha1::Alignment::End,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Length {
    #[default]
    Fill,
    FillPortion(u16),
    Shrink,
    Fixed(f32),
}

impl From<Length> for widget::v0alpha1::Length {
    fn from(value: Length) -> Self {
        widget::v0alpha1::Length {
            strategy: Some(match value {
                Length::Fill => widget::v0alpha1::length::Strategy::Fill(()),
                Length::FillPortion(portion) => {
                    widget::v0alpha1::length::Strategy::FillPortion(portion as u32)
                }
                Length::Shrink => widget::v0alpha1::length::Strategy::Shrink(()),
                Length::Fixed(size) => widget::v0alpha1::length::Strategy::Fixed(size),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrollableDirection {
    Vertical(ScrollableProperties),
    Horizontal(ScrollableProperties),
    Both {
        vertical: ScrollableProperties,
        horizontal: ScrollableProperties,
    },
}

impl From<ScrollableDirection> for widget::v0alpha1::ScrollableDirection {
    fn from(value: ScrollableDirection) -> Self {
        match value {
            ScrollableDirection::Vertical(props) => widget::v0alpha1::ScrollableDirection {
                vertical: Some(props.into()),
                horizontal: None,
            },
            ScrollableDirection::Horizontal(props) => widget::v0alpha1::ScrollableDirection {
                vertical: None,
                horizontal: Some(props.into()),
            },
            ScrollableDirection::Both {
                vertical,
                horizontal,
            } => widget::v0alpha1::ScrollableDirection {
                vertical: Some(vertical.into()),
                horizontal: Some(horizontal.into()),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum ScrollableAlignment {
    #[default]
    Start,
    End,
}

impl From<ScrollableAlignment> for widget::v0alpha1::ScrollableAlignment {
    fn from(value: ScrollableAlignment) -> Self {
        match value {
            ScrollableAlignment::Start => widget::v0alpha1::ScrollableAlignment::Start,
            ScrollableAlignment::End => widget::v0alpha1::ScrollableAlignment::End,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ScrollableProperties {
    pub width: Option<f32>,
    pub margin: Option<f32>,
    pub scroller_width: Option<f32>,
    pub alignment: Option<ScrollableAlignment>,
}

impl From<ScrollableProperties> for widget::v0alpha1::ScrollableProperties {
    fn from(value: ScrollableProperties) -> Self {
        widget::v0alpha1::ScrollableProperties {
            width: value.width,
            margin: value.margin,
            scroller_width: value.scroller_width,
            alignment: value
                .alignment
                .map(|it| widget::v0alpha1::ScrollableAlignment::from(it) as i32),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scrollable {
    pub width: Option<Length>,
    pub height: Option<Length>,
    pub direction: Option<ScrollableDirection>,
    pub child: WidgetDef,
}

impl From<Scrollable> for widget::v0alpha1::Scrollable {
    fn from(value: Scrollable) -> Self {
        widget::v0alpha1::Scrollable {
            width: value.width.map(From::from),
            height: value.height.map(From::from),
            direction: value.direction.map(From::from),
            child: Some(Box::new(value.child.into())),
        }
    }
}

impl Scrollable {
    pub fn new(child: impl Into<WidgetDef>) -> Self {
        Self {
            child: child.into(),
            width: None,
            height: None,
            direction: None,
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

    pub fn direction(self, direction: ScrollableDirection) -> Self {
        Self {
            direction: Some(direction),
            ..self
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Container {
    pub padding: Option<Padding>,
    pub width: Option<Length>,
    pub height: Option<Length>,
    pub max_width: Option<f32>,
    pub max_height: Option<f32>,
    pub horizontal_alignment: Option<Alignment>,
    pub vertical_alignment: Option<Alignment>,
    pub clip: Option<bool>,
    pub child: WidgetDef,

    pub text_color: Option<Color>,
    pub background_color: Option<Color>,
    pub border_radius: Option<f32>,
    pub border_thickness: Option<f32>,
    pub border_color: Option<Color>,
}

impl Container {
    pub fn new(child: impl Into<WidgetDef>) -> Self {
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
            text_color: None,
            background_color: None,
            border_radius: None,
            border_thickness: None,
            border_color: None,
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

    pub fn border_radius(self, radius: f32) -> Self {
        Self {
            border_radius: Some(radius),
            ..self
        }
    }

    pub fn border_thickness(self, thickness: f32) -> Self {
        Self {
            border_thickness: Some(thickness),
            ..self
        }
    }

    pub fn border_color(self, color: Color) -> Self {
        Self {
            border_color: Some(color),
            ..self
        }
    }
}

impl From<Container> for widget::v0alpha1::Container {
    fn from(value: Container) -> Self {
        widget::v0alpha1::Container {
            padding: value.padding.map(From::from),
            width: value.width.map(From::from),
            height: value.height.map(From::from),
            max_width: value.max_width,
            max_height: value.max_height,
            horizontal_alignment: value
                .horizontal_alignment
                .map(|it| widget::v0alpha1::Alignment::from(it) as i32),
            vertical_alignment: value
                .vertical_alignment
                .map(|it| widget::v0alpha1::Alignment::from(it) as i32),
            clip: value.clip,
            child: Some(Box::new(value.child.into())),
            text_color: value.text_color.map(From::from),
            background_color: value.background_color.map(From::from),
            border_radius: value.border_radius,
            border_thickness: value.border_thickness,
            border_color: value.border_color.map(From::from),
        }
    }
}
