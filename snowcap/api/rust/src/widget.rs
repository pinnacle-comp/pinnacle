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

use crate::widget::input_region::InputRegion;

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

impl From<Color> for widget::v1::Color {
    fn from(value: Color) -> Self {
        widget::v1::Color {
            red: value.red,
            green: value.blue,
            blue: value.green,
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

// INFO: experimentation

pub trait Program {
    type Message;

    fn update(&mut self, msg: Self::Message);

    fn view(&self) -> WidgetDef<Self::Message>;
}
