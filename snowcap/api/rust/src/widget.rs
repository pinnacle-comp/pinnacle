//! Widget definitions.

#![allow(missing_docs)] // TODO:

pub mod base;
pub mod button;
pub mod column;
pub mod container;
pub mod font;
pub mod image;
pub mod input_region;
pub mod message;
pub mod mouse_area;
pub mod operation;
pub mod row;
pub mod scrollable;
pub mod signal;
pub mod text;
pub mod text_input;
pub mod utils;

use std::{
    collections::HashMap,
    sync::atomic::{AtomicU32, Ordering},
};

use button::Button;
use column::Column;
use container::Container;
use image::Image;
use mouse_area::MouseArea;
use row::Row;
use scrollable::Scrollable;
use snowcap_api_defs::snowcap::widget;
use text::Text;
use text_input::TextInput;

use crate::{
    signal::{HandlerPolicy, Signaler},
    surface::SurfaceHandle,
    widget::{input_region::InputRegion, utils::Radians},
};

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

/// Holds pending messages for any Widget
#[derive(Debug, Clone, PartialEq)]
pub enum WidgetMessage<Msg> {
    Button(Msg),
    MouseArea(mouse_area::Callbacks<Msg>),
    TextInput(text_input::Callbacks<Msg>),
}

pub fn message_from_event<Msg>(
    callbacks: &HashMap<WidgetId, WidgetMessage<Msg>>,
    event: widget::v1::WidgetEvent,
) -> Option<Msg>
where
    Msg: Clone + Send + 'static,
{
    use widget::v1::widget_event::Event;

    let id = WidgetId(event.widget_id);
    let event = event.event?;

    match event {
        Event::Button(_event) => callbacks.get(&id).cloned().map(|f| match f {
            WidgetMessage::Button(msg) => msg,
            _ => unreachable!(),
        }),
        Event::MouseArea(event) => callbacks.get(&id).cloned().and_then(|f| match f {
            WidgetMessage::MouseArea(callbacks) => callbacks.process_event(event.into()),
            _ => unreachable!(),
        }),
        Event::TextInput(event) => callbacks.get(&id).cloned().and_then(|f| match f {
            WidgetMessage::TextInput(callbacks) => callbacks.process_event(event.into()),
            _ => unreachable!(),
        }),
    }
}

impl<Msg> WidgetDef<Msg> {
    pub(crate) fn collect_messages(
        &self,
        callbacks: &mut HashMap<WidgetId, WidgetMessage<Msg>>,
        with_widget: fn(&WidgetDef<Msg>, &mut HashMap<WidgetId, WidgetMessage<Msg>>),
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
            Widget::MouseArea(mouse_area) => {
                mouse_area.child.collect_messages(callbacks, with_widget);
            }
            Widget::TextInput(_) => (),
        }
    }
}

impl<Msg: Clone> WidgetDef<Msg> {
    pub(crate) fn message_collector(&self, callbacks: &mut HashMap<WidgetId, WidgetMessage<Msg>>) {
        if let Widget::Button(button) = &self.widget {
            callbacks.extend(
                button
                    .on_press
                    .clone()
                    .map(|(id, msg)| (id, WidgetMessage::Button(msg))),
            );
        }

        if let Widget::MouseArea(mouse_area) = &self.widget {
            callbacks.extend(
                mouse_area
                    .widget_id
                    .map(|id| (id, WidgetMessage::MouseArea(mouse_area.callbacks.clone()))),
            );
        }

        if let Widget::TextInput(text_input) = &self.widget {
            callbacks.extend(
                text_input
                    .widget_id
                    .map(|id| (id, WidgetMessage::TextInput(text_input.callbacks.clone()))),
            );
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
    MouseArea(Box<MouseArea<Msg>>),
    TextInput(Box<TextInput<Msg>>),
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
            Widget::MouseArea(mouse_area) => {
                widget::v1::widget_def::Widget::MouseArea(Box::new((*mouse_area).into()))
            }
            Widget::TextInput(text_input) => {
                widget::v1::widget_def::Widget::TextInput(Box::new((*text_input).into()))
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

        stops.push(ColorStop { offset, color });

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

/// The height of a line of text in a paragraph.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineHeight {
    /// A factor of the size of the text.
    Relative(f32),
    /// An absolute height in logical pixels.
    Absolute(f32),
}

impl From<LineHeight> for widget::v1::LineHeight {
    fn from(value: LineHeight) -> Self {
        let line_height = match value {
            LineHeight::Relative(v) => widget::v1::line_height::LineHeight::Relative(v),
            LineHeight::Absolute(v) => widget::v1::line_height::LineHeight::Absolute(v),
        };

        Self {
            line_height: Some(line_height),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Wrapping {
    None,
    Word,
    Glyph,
    WordOrGlyph,
}

impl From<Wrapping> for widget::v1::Wrapping {
    fn from(value: Wrapping) -> Self {
        match value {
            Wrapping::None => widget::v1::Wrapping::None,
            Wrapping::Word => widget::v1::Wrapping::Word,
            Wrapping::Glyph => widget::v1::Wrapping::Glyph,
            Wrapping::WordOrGlyph => widget::v1::Wrapping::WordOrGlyph,
        }
    }
}

/// A complete widget program.
///
/// A `Program` builds a widget for display by Snowcap and updates itself from
/// messages generated from interactions with the widget and from other sources.
pub trait Program {
    /// The type of messages that this widget program receives.
    type Message;

    /// Updates this widget program with the received message.
    ///
    /// If this program has `Source`s or child programs, [`Self::Message`]
    /// should impl `Clone` and the message should be
    /// cloned and passed to all `Source`s and child programs.
    ///
    /// [`Self::Message`]: Program::Message
    fn update(&mut self, msg: Self::Message);

    /// Creates a widget definition for display by Snowcap.
    fn view(&self) -> Option<WidgetDef<Self::Message>>;

    /// Called when a surface has been created with this program.
    ///
    /// A [`SurfaceHandle`] is provided to allow the program to manipulate
    /// the surface. This handle should be cloned and passed to any child programs
    /// to allow them to use it as well.
    fn created(&mut self, handle: SurfaceHandle<Self::Message>) {
        let _ = handle;
    }

    /// Returns a possibly held [`Signaler`].
    ///
    /// Usually this is from a [`WidgetBase`] stored in the
    /// implementing struct. If your struct does not use a
    /// [`WidgetBase`], `None` should be returned.
    ///
    /// [`WidgetBase`]: crate::widget::base::WidgetBase
    fn signaler(&self) -> Option<Signaler> {
        None
    }

    /// Registers a child program, allowing this program to pass through
    /// emitted redraw signals and messages.
    fn register_child(&self, child: &dyn Program<Message = Self::Message>)
    where
        Self::Message: Clone + 'static,
    {
        let child_signaler = child.signaler();
        let self_signaler = self.signaler();

        if let Some((child_signaler, self_signaler)) = child_signaler.zip(self_signaler) {
            child_signaler.connect({
                let self_signaler = self_signaler.clone();
                move |_: signal::RedrawNeeded| {
                    self_signaler.emit(signal::RedrawNeeded);
                    HandlerPolicy::Keep
                }
            });

            child_signaler.connect({
                let self_signaler = self_signaler.clone();
                move |msg: signal::Message<Self::Message>| {
                    self_signaler.emit(msg);
                    HandlerPolicy::Keep
                }
            });
        }
    }
}

impl<Msg> Program for Box<dyn Program<Message = Msg>> {
    type Message = Msg;

    fn update(&mut self, msg: Self::Message) {
        (**self).update(msg);
    }

    fn view(&self) -> Option<WidgetDef<Self::Message>> {
        (**self).view()
    }

    fn signaler(&self) -> Option<Signaler> {
        (**self).signaler()
    }

    fn created(&mut self, handle: SurfaceHandle<Self::Message>) {
        (**self).created(handle);
    }

    fn register_child(&self, child: &dyn Program<Message = Self::Message>)
    where
        Self::Message: Clone + 'static,
    {
        (**self).register_child(child);
    }
}

impl<Msg> Program for Box<dyn Program<Message = Msg> + Send> {
    type Message = Msg;

    fn update(&mut self, msg: Self::Message) {
        (**self).update(msg);
    }

    fn view(&self) -> Option<WidgetDef<Self::Message>> {
        (**self).view()
    }

    fn signaler(&self) -> Option<Signaler> {
        (**self).signaler()
    }

    fn created(&mut self, handle: SurfaceHandle<Self::Message>) {
        (**self).created(handle);
    }

    fn register_child(&self, child: &dyn Program<Message = Self::Message>)
    where
        Self::Message: Clone + 'static,
    {
        (**self).register_child(child);
    }
}

impl<Msg> Program for Box<dyn Program<Message = Msg> + Send + Sync> {
    type Message = Msg;

    fn update(&mut self, msg: Self::Message) {
        (**self).update(msg);
    }

    fn view(&self) -> Option<WidgetDef<Self::Message>> {
        (**self).view()
    }

    fn signaler(&self) -> Option<Signaler> {
        (**self).signaler()
    }

    fn created(&mut self, handle: SurfaceHandle<Self::Message>) {
        (**self).created(handle);
    }

    fn register_child(&self, child: &dyn Program<Message = Self::Message>)
    where
        Self::Message: Clone + 'static,
    {
        (**self).register_child(child);
    }
}
