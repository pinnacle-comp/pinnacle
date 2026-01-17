//! Widget definitions.

#![allow(missing_docs)] // TODO:

pub mod button;
pub mod column;
pub mod container;
pub mod font;
pub mod image;
pub mod input_region;
pub mod row;
pub mod scrollable;
pub mod text;
pub mod utils;

use std::{
    collections::HashMap,
    sync::atomic::{AtomicU32, Ordering},
};

use button::Button;
use column::Column;
use container::Container;
use image::Image;
use row::Row;
use scrollable::Scrollable;
use snowcap_api_defs::snowcap::widget;
use text::Text;

use crate::widget::{input_region::InputRegion, utils::Radians};

/// A unique identifier for a widget.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Hash)]
pub struct WidgetId(pub u32);

static WIDGET_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

impl WidgetId {
    /// Get the raw u32.
    pub fn to_inner(self) -> u32 {
        self.0
    }

    pub fn next() -> Self {
        Self(WIDGET_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl From<u32> for WidgetId {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Palette {
    pub background: Color,
    pub text: Color,
    pub primary: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
}

impl From<Palette> for widget::v1::Palette {
    fn from(value: Palette) -> Self {
        Self {
            background: Some(value.background.into()),
            text: Some(value.text.into()),
            primary: Some(value.primary.into()),
            success: Some(value.success.into()),
            warning: Some(value.warning.into()),
            danger: Some(value.danger.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Theme {
    pub palette: Option<Palette>,

    pub text_style: Option<text::Style>,
    pub scrollable_style: Option<scrollable::Style>,
    pub container_style: Option<container::Style>,
    pub button_style: Option<button::Styles>,
}

impl From<Theme> for widget::v1::Theme {
    fn from(value: Theme) -> Self {
        Self {
            palette: value.palette.map(From::from),
            text_style: value.text_style.map(From::from),
            scrollable_style: value.scrollable_style.map(From::from),
            container_style: value.container_style.map(From::from),
            button_style: value.button_style.map(From::from),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WidgetDef<Msg> {
    pub theme: Option<Theme>,
    pub id: Option<String>,
    pub widget: Widget<Msg>,
}

impl<Msg> WidgetDef<Msg> {
    pub(crate) fn collect_messages(
        &self,
        callbacks: &mut HashMap<WidgetId, Msg>,
        with_widget: fn(&WidgetDef<Msg>, &mut HashMap<WidgetId, Msg>),
    ) {
        with_widget(self, callbacks);
        match &self.widget {
            Widget::Text(_) => (),
            Widget::Column(column) => {
                for widget in column.children.iter() {
                    widget.collect_messages(callbacks, with_widget);
                }
            }
            Widget::Row(row) => {
                for widget in row.children.iter() {
                    widget.collect_messages(callbacks, with_widget);
                }
            }
            Widget::Scrollable(scrollable) => {
                scrollable.child.collect_messages(callbacks, with_widget);
            }
            Widget::Container(container) => {
                container.child.collect_messages(callbacks, with_widget);
            }
            Widget::Button(button) => {
                button.child.collect_messages(callbacks, with_widget);
            }
            Widget::Image(_) => (),
            Widget::InputRegion(input_region) => {
                input_region.child.collect_messages(callbacks, with_widget);
            }
        }
    }
}

impl<Msg> From<WidgetDef<Msg>> for widget::v1::WidgetDef {
    fn from(value: WidgetDef<Msg>) -> Self {
        Self {
            theme: value.theme.map(From::from),
            widget: Some(value.widget.into()),
        }
    }
}

/// A widget definition.
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, from_variants::FromVariants)]
pub enum Widget<Msg> {
    Text(Text),
    Column(Column<Msg>),
    Row(Row<Msg>),
    Scrollable(Box<Scrollable<Msg>>),
    Container(Box<Container<Msg>>),
    Button(Box<Button<Msg>>),
    Image(Image),
    InputRegion(Box<InputRegion<Msg>>),
}

impl<Msg, T: Into<Widget<Msg>>> From<T> for WidgetDef<Msg> {
    fn from(value: T) -> Self {
        Self {
            theme: None,
            id: None,
            widget: value.into(),
        }
    }
}

impl<Msg> From<Widget<Msg>> for widget::v1::widget_def::Widget {
    fn from(value: Widget<Msg>) -> widget::v1::widget_def::Widget {
        match value {
            Widget::Text(text) => widget::v1::widget_def::Widget::Text(text.into()),
            Widget::Column(column) => widget::v1::widget_def::Widget::Column(column.into()),
            Widget::Row(row) => widget::v1::widget_def::Widget::Row(row.into()),
            Widget::Scrollable(scrollable) => {
                widget::v1::widget_def::Widget::Scrollable(Box::new((*scrollable).into()))
            }
            Widget::Container(container) => {
                widget::v1::widget_def::Widget::Container(Box::new((*container).into()))
            }
            Widget::Button(button) => {
                widget::v1::widget_def::Widget::Button(Box::new((*button).into()))
            }
            Widget::Image(image) => widget::v1::widget_def::Widget::Image(image.into()),
            Widget::InputRegion(input_region) => {
                widget::v1::widget_def::Widget::InputRegion(Box::new((*input_region).into()))
            }
        }
    }
}

/// A color.
///
/// All channels are ranges from [0.0, 1.0].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Color {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

impl Color {
    /// Creates a `Color` from red, green, and blue channels.
    ///
    /// Values range from 0.0 to 1.0, inclusive.
    ///
    /// The alpha channel is set to 1.0.
    pub fn rgb(red: f32, green: f32, blue: f32) -> Self {
        Self::rgba(red, green, blue, 1.0)
    }

    /// Creates a `Color` from red, green, blue, and alpha channels.
    ///
    /// Values range from 0.0 to 1.0, inclusive.
    pub fn rgba(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self {
            red: red.clamp(0.0, 1.0),
            green: green.clamp(0.0, 1.0),
            blue: blue.clamp(0.0, 1.0),
            alpha: alpha.clamp(0.0, 1.0),
        }
    }
}

impl From<[f32; 4]> for Color {
    fn from([red, green, blue, alpha]: [f32; 4]) -> Self {
        Color::rgba(red, green, blue, alpha)
    }
}

impl From<[f32; 3]> for Color {
    fn from([red, green, blue]: [f32; 3]) -> Self {
        Color::rgb(red, green, blue)
    }
}

impl From<Color> for widget::v1::Color {
    fn from(value: Color) -> Self {
        widget::v1::Color {
            red: value.red,
            green: value.green,
            blue: value.blue,
            alpha: value.alpha,
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

impl From<f32> for Padding {
    fn from(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

impl From<Padding> for widget::v1::Padding {
    fn from(value: Padding) -> Self {
        widget::v1::Padding {
            top: value.top,
            right: value.right,
            bottom: value.bottom,
            left: value.left,
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

impl From<Alignment> for widget::v1::Alignment {
    fn from(value: Alignment) -> Self {
        match value {
            Alignment::Start => widget::v1::Alignment::Start,
            Alignment::Center => widget::v1::Alignment::Center,
            Alignment::End => widget::v1::Alignment::End,
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

impl From<Length> for widget::v1::Length {
    fn from(value: Length) -> Self {
        widget::v1::Length {
            strategy: Some(match value {
                Length::Fill => widget::v1::length::Strategy::Fill(()),
                Length::FillPortion(portion) => {
                    widget::v1::length::Strategy::FillPortion(portion as u32)
                }
                Length::Shrink => widget::v1::length::Strategy::Shrink(()),
                Length::Fixed(size) => widget::v1::length::Strategy::Fixed(size),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Border {
    pub color: Option<Color>,
    pub width: Option<f32>,
    pub radius: Option<Radius>,
}

impl From<Border> for widget::v1::Border {
    fn from(value: Border) -> Self {
        Self {
            color: value.color.map(From::from),
            width: value.width,
            radius: value.radius.map(From::from),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Radius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl From<f32> for Radius {
    fn from(value: f32) -> Self {
        Self {
            top_left: value,
            top_right: value,
            bottom_right: value,
            bottom_left: value,
        }
    }
}

impl From<Radius> for widget::v1::Radius {
    fn from(value: Radius) -> Self {
        Self {
            top_left: value.top_left,
            top_right: value.top_right,
            bottom_right: value.bottom_right,
            bottom_left: value.bottom_left,
        }
    }
}

/// The background of some element.
#[derive(Debug, Clone, PartialEq)]
pub enum Background {
    /// A solid color.
    Color(Color),
    /// Interpolate between several colors.
    Gradient(Gradient),
}

impl From<Color> for Background {
    fn from(color: Color) -> Self {
        Self::Color(color)
    }
}

impl From<Gradient> for Background {
    fn from(gradient: Gradient) -> Self {
        Self::Gradient(gradient)
    }
}

impl From<Linear> for Background {
    fn from(linear: Linear) -> Self {
        Self::Gradient(Gradient::Linear(linear))
    }
}

impl From<Background> for widget::v1::Background {
    fn from(value: Background) -> Self {
        let background = match value {
            Background::Color(c) => widget::v1::background::Background::Color(c.into()),
            Background::Gradient(g) => widget::v1::background::Background::Gradient(g.into()),
        };

        Self {
            background: Some(background),
        }
    }
}

/// A fill which transitions colors progressively.
#[derive(Debug, Clone, PartialEq)]
pub enum Gradient {
    /// A linear gradient that interpolates colors along a direction at a specific angle.
    Linear(Linear),
}

impl From<Gradient> for widget::v1::Gradient {
    fn from(value: Gradient) -> Self {
        let gradient = match value {
            Gradient::Linear(l) => widget::v1::gradient::Gradient::Linear(l.into()),
        };

        Self {
            gradient: Some(gradient),
        }
    }
}

/// A linear gradient
#[derive(Debug, Clone, PartialEq)]
pub struct Linear {
    /// How the [`Gradient`] is angled.
    pub radians: Radians,
    /// [`ColorStop`] to interpolates.
    pub stops: Vec<ColorStop>,
}

impl Linear {
    /// Create a new [`Linear`] gradient with the given angle in [`Radians`].
    pub fn new(angle: impl Into<Radians>) -> Self {
        Self {
            radians: angle.into(),
            stops: Default::default(),
        }
    }

    /// Adds a new [`ColorStop`], defined by an offset and a color.
    ///
    /// `offset`s not within the 0.0..=1.0 range will be ignored.
    /// Any stop added after the 8th will be ignored.
    ///
    /// If a new stop with an equivalent offset is added, it will replace the previous one.
    /// Equivalence is defined by `||old - new|| <= f32::EPSILON`
    pub fn add_stop(self, offset: f32, color: Color) -> Self {
        let Self { radians, mut stops } = self;

        if offset.is_finite() && (0.0..=1.0).contains(&offset) {
            let search = stops.binary_search_by(|stop| {
                if (stop.offset - offset).abs() <= f32::EPSILON {
                    std::cmp::Ordering::Equal
                } else {
                    stop.offset.partial_cmp(&offset).unwrap()
                }
            });

            let stop = ColorStop { offset, color };

            match search {
                Ok(pos) => stops[pos] = stop,
                Err(pos) => {
                    if stops.len() < 8 {
                        stops.insert(pos, stop)
                    } else {
                        tracing::warn!("Linear::stops is full. Ignoring {stop:?}");
                    }
                }
            }
        } else {
            tracing::warn!("Offset should be in the range 0.0..=1.0");
        }

        Self { radians, stops }
    }

    /// Adds multiple [`ColorStop`] to the gradient.
    pub fn add_stops(mut self, stops: impl IntoIterator<Item = ColorStop>) -> Self {
        for ColorStop { offset, color } in stops {
            self = self.add_stop(offset, color);
        }

        self
    }
}

impl From<Linear> for widget::v1::gradient::Linear {
    fn from(value: Linear) -> Self {
        let Linear { radians, stops } = value;

        Self {
            radians: radians.0,
            stops: stops.into_iter().map(From::from).collect(),
        }
    }
}

/// A point along a gradient vector where the specified [`Color`] is unmixed.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ColorStop {
    /// Offset along the gradient vector.
    pub offset: f32,
    /// The color of the gradient at the specified `offset`.
    pub color: Color,
}

impl From<ColorStop> for widget::v1::gradient::ColorStop {
    fn from(value: ColorStop) -> Self {
        let ColorStop { offset, color } = value;

        Self {
            offset,
            color: Some(color.into()),
        }
    }
}

// INFO: experimentation

pub trait Program {
    type Message;

    fn update(&mut self, msg: Self::Message);

    fn view(&self) -> WidgetDef<Self::Message>;
}
