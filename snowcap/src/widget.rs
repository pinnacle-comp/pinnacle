use std::{any::Any, collections::HashMap};

use iced::{
    widget::{Column, Container, Row, Scrollable},
    Command,
};
use iced_runtime::Program;
use iced_wgpu::core::Element;
use snowcap_api_defs::snowcap::widget::{
    self,
    v0alpha1::{widget_def, WidgetDef},
};

use crate::{layer::SnowcapLayer, state::State, util::convert::FromApi};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct WidgetId(u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct WidgetIdCounter(WidgetId);

impl WidgetIdCounter {
    pub fn next_and_increment(&mut self) -> WidgetId {
        let ret = self.0;
        self.0 .0 += 1;
        ret
    }
}

impl WidgetId {
    pub fn into_inner(self) -> u32 {
        self.0
    }

    pub fn layer_for_mut<'a>(&self, state: &'a mut State) -> Option<&'a mut SnowcapLayer> {
        state
            .layers
            .iter_mut()
            .find(|sn_layer| &sn_layer.widget_id == self)
    }
}

impl From<u32> for WidgetId {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

pub struct SnowcapWidgetProgram {
    pub widgets: WidgetFn,
    pub widget_state: HashMap<u32, Box<dyn Any + Send>>,
}

pub type WidgetFn = Box<
    dyn for<'a> Fn(
        &'a HashMap<u32, Box<dyn Any + Send>>,
    ) -> Element<'a, SnowcapMessage, iced::Theme, iced_wgpu::Renderer>,
>;

#[derive(Debug)]
pub enum SnowcapMessage {
    Noop,
    Close,
    Update(u32, Box<dyn Any + Send>),
}

impl Program for SnowcapWidgetProgram {
    type Renderer = iced_wgpu::Renderer;

    type Theme = iced::Theme;

    type Message = SnowcapMessage;

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            SnowcapMessage::Noop => (),
            SnowcapMessage::Close => (),
            SnowcapMessage::Update(id, data) => {
                self.widget_state.insert(id, data);
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Theme, Self::Renderer> {
        (self.widgets)(&self.widget_state)
    }
}

pub fn widget_def_to_fn(def: WidgetDef) -> Option<(WidgetFn, HashMap<u32, Box<dyn Any + Send>>)> {
    let mut states = HashMap::new();
    let mut current_id = 0;

    let f = widget_def_to_fn_inner(def, &mut current_id, &mut states);

    f.map(|f| (f, states))
}

fn widget_def_to_fn_inner(
    def: WidgetDef,
    current_id: &mut u32,
    _states: &mut HashMap<u32, Box<dyn Any + Send>>,
) -> Option<WidgetFn> {
    let def = def.widget?;
    match def {
        widget_def::Widget::Text(text_def) => {
            let horizontal_alignment = text_def.horizontal_alignment();
            let vertical_alignment = text_def.vertical_alignment();

            let widget::v0alpha1::Text {
                text,
                pixels,
                width,
                height,
                horizontal_alignment: _,
                vertical_alignment: _,
                color,
                font,
            } = text_def;

            let f: WidgetFn = Box::new(move |_states| {
                let mut text = iced::widget::Text::new(text.clone().unwrap_or_default());
                if let Some(pixels) = pixels {
                    text = text.size(pixels);
                }
                if let Some(width) = width.clone() {
                    text = text.width(iced::Length::from_api(width));
                }
                if let Some(height) = height.clone() {
                    text = text.height(iced::Length::from_api(height));
                }
                if let Some(color) = color.clone() {
                    text = text.style(iced::theme::Text::Color(iced::Color::from_api(color)));
                }

                match horizontal_alignment {
                    widget::v0alpha1::Alignment::Unspecified => (),
                    widget::v0alpha1::Alignment::Start => {
                        text = text.horizontal_alignment(iced::alignment::Horizontal::Left)
                    }
                    widget::v0alpha1::Alignment::Center => {
                        text = text.horizontal_alignment(iced::alignment::Horizontal::Center)
                    }
                    widget::v0alpha1::Alignment::End => {
                        text = text.horizontal_alignment(iced::alignment::Horizontal::Right)
                    }
                }

                match vertical_alignment {
                    widget::v0alpha1::Alignment::Unspecified => (),
                    widget::v0alpha1::Alignment::Start => {
                        text = text.vertical_alignment(iced::alignment::Vertical::Top)
                    }
                    widget::v0alpha1::Alignment::Center => {
                        text = text.vertical_alignment(iced::alignment::Vertical::Center)
                    }
                    widget::v0alpha1::Alignment::End => {
                        text = text.vertical_alignment(iced::alignment::Vertical::Bottom)
                    }
                }

                if let Some(font) = font.clone() {
                    text = text.font(iced::Font::from_api(font));
                }

                text.into()
            });
            Some(f)
        }
        widget_def::Widget::Column(widget::v0alpha1::Column {
            spacing,
            padding,
            item_alignment,
            width,
            height,
            max_width,
            clip,
            children,
        }) => {
            let children_widget_fns = children
                .into_iter()
                .flat_map(|def| {
                    *current_id += 1;
                    widget_def_to_fn_inner(def, current_id, _states)
                })
                .collect::<Vec<_>>();

            let f: WidgetFn = Box::new(move |states| {
                let mut column = Column::new();

                if let Some(spacing) = spacing {
                    column = column.spacing(spacing);
                }

                if let Some(width) = width.clone() {
                    column = column.width(iced::Length::from_api(width));
                }
                if let Some(height) = height.clone() {
                    column = column.height(iced::Length::from_api(height));
                }
                if let Some(max_width) = max_width {
                    column = column.max_width(max_width);
                }
                if let Some(clip) = clip {
                    column = column.clip(clip);
                }

                if let Some(padding) = padding.clone() {
                    column = column.padding(iced::Padding::from_api(padding));
                }

                if let Some(alignment) = item_alignment {
                    column = column.align_items(match alignment {
                        // FIXME: actual conversion logic
                        1 => iced::Alignment::Start,
                        2 => iced::Alignment::Center,
                        3 => iced::Alignment::End,
                        _ => iced::Alignment::Start,
                    });
                }

                for child in children_widget_fns.iter() {
                    column = column.push(child(states));
                }

                column.into()
            });

            Some(f)
        }
        widget_def::Widget::Row(widget::v0alpha1::Row {
            spacing,
            padding,
            item_alignment,
            width,
            height,
            clip,
            children,
        }) => {
            let children_widget_fns = children
                .into_iter()
                .flat_map(|def| {
                    *current_id += 1;
                    widget_def_to_fn_inner(def, current_id, _states)
                })
                .collect::<Vec<_>>();

            let f: WidgetFn = Box::new(move |states| {
                let mut row = Row::new();

                if let Some(spacing) = spacing {
                    row = row.spacing(spacing);
                }

                if let Some(width) = width.clone() {
                    row = row.width(iced::Length::from_api(width));
                }
                if let Some(height) = height.clone() {
                    row = row.height(iced::Length::from_api(height));
                }
                if let Some(clip) = clip {
                    row = row.clip(clip);
                }

                if let Some(widget::v0alpha1::Padding {
                    top,
                    right,
                    bottom,
                    left,
                }) = padding
                {
                    row = row.padding([
                        top.unwrap_or_default(),
                        right.unwrap_or_default(),
                        bottom.unwrap_or_default(),
                        left.unwrap_or_default(),
                    ]);
                }

                if let Some(alignment) = item_alignment {
                    row = row.align_items(match alignment {
                        // FIXME: actual conversion logic
                        1 => iced::Alignment::Start,
                        2 => iced::Alignment::Center,
                        3 => iced::Alignment::End,
                        _ => iced::Alignment::Start,
                    });
                }

                for child in children_widget_fns.iter() {
                    row = row.push(child(states));
                }

                row.into()
            });

            Some(f)
        }
        widget_def::Widget::Scrollable(scrollable_def) => {
            let widget::v0alpha1::Scrollable {
                width,
                height,
                direction,
                child,
            } = *scrollable_def;

            let child_widget_fn = child.and_then(|def| {
                *current_id += 1;
                widget_def_to_fn_inner(*def, current_id, _states)
            });

            let f: WidgetFn = Box::new(move |states| {
                let mut scrollable = Scrollable::new(
                    child_widget_fn
                        .as_ref()
                        .map(|child| child(states))
                        .unwrap_or_else(|| iced::widget::Text::new("NULL").into()),
                );

                if let Some(width) = width.clone() {
                    scrollable = scrollable.width(iced::Length::from_api(width));
                }
                if let Some(height) = height.clone() {
                    scrollable = scrollable.height(iced::Length::from_api(height));
                }
                if let Some(direction) = direction.clone() {
                    scrollable = scrollable
                        .direction(iced::widget::scrollable::Direction::from_api(direction));
                }

                scrollable.into()
            });

            Some(f)
        }
        widget_def::Widget::Container(container_def) => {
            let horizontal_alignment = container_def.horizontal_alignment();
            let vertical_alignment = container_def.vertical_alignment();

            let widget::v0alpha1::Container {
                padding,
                width,
                height,
                max_width,
                max_height,
                horizontal_alignment: _,
                vertical_alignment: _,
                clip,
                child,

                text_color,
                background_color,
                border_radius,
                border_thickness,
                border_color,
            } = *container_def;

            let child_widget_fn = child.and_then(|def| {
                *current_id += 1;
                widget_def_to_fn_inner(*def, current_id, _states)
            });

            let f: WidgetFn = Box::new(move |states| {
                let mut container = Container::new(
                    child_widget_fn
                        .as_ref()
                        .map(|child| child(states))
                        .unwrap_or_else(|| iced::widget::Text::new("NULL").into()),
                );

                if let Some(width) = width.clone() {
                    container = container.width(iced::Length::from_api(width));
                }
                if let Some(height) = height.clone() {
                    container = container.height(iced::Length::from_api(height));
                }
                if let Some(max_width) = max_width {
                    container = container.max_width(max_width);
                }
                if let Some(max_height) = max_height {
                    container = container.max_height(max_height);
                }
                if let Some(clip) = clip {
                    container = container.clip(clip);
                }
                if let Some(padding) = padding.clone() {
                    container = container.padding(iced::Padding::from_api(padding));
                }
                container = container.align_x(match horizontal_alignment {
                    widget::v0alpha1::Alignment::Unspecified => iced::alignment::Horizontal::Left,
                    widget::v0alpha1::Alignment::Start => iced::alignment::Horizontal::Left,
                    widget::v0alpha1::Alignment::Center => iced::alignment::Horizontal::Center,
                    widget::v0alpha1::Alignment::End => iced::alignment::Horizontal::Right,
                });
                container = container.align_y(match vertical_alignment {
                    widget::v0alpha1::Alignment::Unspecified => iced::alignment::Vertical::Top,
                    widget::v0alpha1::Alignment::Start => iced::alignment::Vertical::Top,
                    widget::v0alpha1::Alignment::Center => iced::alignment::Vertical::Center,
                    widget::v0alpha1::Alignment::End => iced::alignment::Vertical::Bottom,
                });

                let text_color_clone = text_color.clone();
                let background_color_clone = background_color.clone();
                let border_color_clone = border_color.clone();

                let style = move |theme: &iced::Theme| {
                    use iced::widget::container::Appearance;

                    let palette = theme.extended_palette();

                    let mut appearance = Appearance {
                        text_color: None,
                        background: Some(palette.background.weak.color.into()),
                        border: iced::Border {
                            color: palette.background.base.color,
                            width: 0.0,
                            radius: 2.0.into(),
                        },
                        shadow: iced::Shadow::default(),
                    };

                    if let Some(text_color) = text_color_clone.clone() {
                        appearance.text_color = Some(iced::Color::from_api(text_color));
                    }

                    if let Some(background_color) = background_color_clone.clone() {
                        appearance.background =
                            Some(iced::Color::from_api(background_color).into());
                    }

                    if let Some(border_color) = border_color_clone.clone() {
                        appearance.border.color = iced::Color::from_api(border_color);
                    }

                    if let Some(border_radius) = border_radius {
                        appearance.border.radius = border_radius.into();
                    }

                    if let Some(border_thickness) = border_thickness {
                        appearance.border.width = border_thickness;
                    }

                    appearance
                };

                container = container.style(iced::theme::Container::Custom(Box::new(style)));

                container.into()
            });

            Some(f)
        }
    }
}
