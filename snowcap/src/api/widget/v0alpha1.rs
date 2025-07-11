use std::{any::Any, collections::HashMap};

use iced::widget::{Column, Container, Row, Scrollable, scrollable::Scrollbar};
use snowcap_api_defs::snowcap::widget::{
    self,
    v0alpha1::{WidgetDef, widget_def},
};

use crate::{util::convert::FromApi, widget::WidgetFn};

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
                if let Some(width) = width {
                    text = text.width(iced::Length::from_api(width));
                }
                if let Some(height) = height {
                    text = text.height(iced::Length::from_api(height));
                }
                if let Some(color) = color {
                    text = text.style(move |_| iced::widget::text::Style {
                        color: Some(iced::Color::from_api(color)),
                    });
                }

                match horizontal_alignment {
                    widget::v0alpha1::Alignment::Unspecified => (),
                    widget::v0alpha1::Alignment::Start => {
                        text = text.align_x(iced::alignment::Horizontal::Left)
                    }
                    widget::v0alpha1::Alignment::Center => {
                        text = text.align_x(iced::alignment::Horizontal::Center)
                    }
                    widget::v0alpha1::Alignment::End => {
                        text = text.align_x(iced::alignment::Horizontal::Right)
                    }
                }

                match vertical_alignment {
                    widget::v0alpha1::Alignment::Unspecified => (),
                    widget::v0alpha1::Alignment::Start => {
                        text = text.align_y(iced::alignment::Vertical::Top)
                    }
                    widget::v0alpha1::Alignment::Center => {
                        text = text.align_y(iced::alignment::Vertical::Center)
                    }
                    widget::v0alpha1::Alignment::End => {
                        text = text.align_y(iced::alignment::Vertical::Bottom)
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

                if let Some(width) = width {
                    column = column.width(iced::Length::from_api(width));
                }
                if let Some(height) = height {
                    column = column.height(iced::Length::from_api(height));
                }
                if let Some(max_width) = max_width {
                    column = column.max_width(max_width);
                }
                if let Some(clip) = clip {
                    column = column.clip(clip);
                }

                if let Some(padding) = padding {
                    column = column.padding(iced::Padding::from_api(padding));
                }

                if let Some(alignment) = item_alignment {
                    column = column.align_x(match alignment {
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

                if let Some(width) = width {
                    row = row.width(iced::Length::from_api(width));
                }
                if let Some(height) = height {
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
                    row = row.padding(iced::Padding {
                        top: top.unwrap_or_default(),
                        right: right.unwrap_or_default(),
                        bottom: bottom.unwrap_or_default(),
                        left: left.unwrap_or_default(),
                    });
                }

                if let Some(alignment) = item_alignment {
                    row = row.align_y(match alignment {
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

                if let Some(width) = width {
                    scrollable = scrollable.width(iced::Length::from_api(width));
                }
                if let Some(height) = height {
                    scrollable = scrollable.height(iced::Length::from_api(height));
                }
                if let Some(direction) = direction {
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

                if let Some(width) = width {
                    container = container.width(iced::Length::from_api(width));
                }
                if let Some(height) = height {
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
                if let Some(padding) = padding {
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

                let text_color_clone = text_color;
                let background_color_clone = background_color;
                let border_color_clone = border_color;

                let style = move |theme: &iced::Theme| {
                    let palette = theme.extended_palette();

                    let mut style = iced::widget::container::Style {
                        text_color: None,
                        background: Some(palette.background.weak.color.into()),
                        border: iced::Border {
                            color: palette.background.base.color,
                            width: 0.0,
                            radius: 2.0.into(),
                        },
                        shadow: iced::Shadow::default(),
                        snap: false,
                    };

                    if let Some(text_color) = text_color_clone {
                        style.text_color = Some(iced::Color::from_api(text_color));
                    }

                    if let Some(background_color) = background_color_clone {
                        style.background = Some(iced::Color::from_api(background_color).into());
                    }

                    if let Some(border_color) = border_color_clone {
                        style.border.color = iced::Color::from_api(border_color);
                    }

                    if let Some(border_radius) = border_radius {
                        style.border.radius = border_radius.into();
                    }

                    if let Some(border_thickness) = border_thickness {
                        style.border.width = border_thickness;
                    }

                    style
                };

                container = container.style(style);

                container.into()
            });

            Some(f)
        }
    }
}

impl FromApi<widget::v0alpha1::Length> for iced::Length {
    fn from_api(length: widget::v0alpha1::Length) -> Self {
        use widget::v0alpha1::length::Strategy;
        match length.strategy.unwrap_or(Strategy::Fill(())) {
            Strategy::Fill(_) => iced::Length::Fill,
            Strategy::FillPortion(portion) => iced::Length::FillPortion(portion as u16),
            Strategy::Shrink(_) => iced::Length::Shrink,
            Strategy::Fixed(size) => iced::Length::Fixed(size),
        }
    }
}

impl FromApi<widget::v0alpha1::Alignment> for iced::Alignment {
    fn from_api(api_type: widget::v0alpha1::Alignment) -> Self {
        match api_type {
            widget::v0alpha1::Alignment::Unspecified => iced::Alignment::Start,
            widget::v0alpha1::Alignment::Start => iced::Alignment::Start,
            widget::v0alpha1::Alignment::Center => iced::Alignment::Center,
            widget::v0alpha1::Alignment::End => iced::Alignment::End,
        }
    }
}

impl FromApi<widget::v0alpha1::ScrollableProperties> for iced::widget::scrollable::Scrollbar {
    fn from_api(api_type: widget::v0alpha1::ScrollableProperties) -> Self {
        let mut properties = iced::widget::scrollable::Scrollbar::new();
        if let Some(width) = api_type.width {
            properties = properties.width(width);
        }
        if let Some(margin) = api_type.margin {
            properties = properties.margin(margin);
        }
        if let Some(scroller_width) = api_type.scroller_width {
            properties = properties.scroller_width(scroller_width);
        }
        properties
    }
}

impl FromApi<widget::v0alpha1::ScrollableDirection> for iced::widget::scrollable::Direction {
    fn from_api(api_type: widget::v0alpha1::ScrollableDirection) -> Self {
        match (api_type.vertical, api_type.horizontal) {
            (Some(vertical), Some(horizontal)) => Self::Both {
                vertical: Scrollbar::from_api(vertical),
                horizontal: Scrollbar::from_api(horizontal),
            },
            (Some(vertical), None) => Self::Vertical(Scrollbar::from_api(vertical)),
            (None, Some(horizontal)) => Self::Horizontal(Scrollbar::from_api(horizontal)),
            (None, None) => Self::default(),
        }
    }
}

impl FromApi<widget::v0alpha1::Padding> for iced::Padding {
    fn from_api(api_type: widget::v0alpha1::Padding) -> Self {
        iced::Padding {
            top: api_type.top(),
            right: api_type.right(),
            bottom: api_type.bottom(),
            left: api_type.left(),
        }
    }
}

impl FromApi<widget::v0alpha1::Color> for iced::Color {
    fn from_api(api_type: widget::v0alpha1::Color) -> Self {
        iced::Color {
            r: api_type.red().clamp(0.0, 1.0),
            g: api_type.green().clamp(0.0, 1.0),
            b: api_type.blue().clamp(0.0, 1.0),
            a: api_type.alpha.unwrap_or(1.0).clamp(0.0, 1.0),
        }
    }
}

impl FromApi<widget::v0alpha1::font::Family> for iced::font::Family {
    fn from_api(api_type: widget::v0alpha1::font::Family) -> Self {
        match api_type.family {
            Some(family) => match family {
                widget::v0alpha1::font::family::Family::Name(name) => {
                    iced::font::Family::Name(name.leak()) // why does this take &'static str
                }
                widget::v0alpha1::font::family::Family::Serif(_) => iced::font::Family::Serif,
                widget::v0alpha1::font::family::Family::SansSerif(_) => {
                    iced::font::Family::SansSerif
                }
                widget::v0alpha1::font::family::Family::Cursive(_) => iced::font::Family::Cursive,
                widget::v0alpha1::font::family::Family::Fantasy(_) => iced::font::Family::Fantasy,
                widget::v0alpha1::font::family::Family::Monospace(_) => {
                    iced::font::Family::Monospace
                }
            },
            None => Default::default(),
        }
    }
}

impl FromApi<widget::v0alpha1::font::Weight> for iced::font::Weight {
    fn from_api(api_type: widget::v0alpha1::font::Weight) -> Self {
        match api_type {
            widget::v0alpha1::font::Weight::Unspecified => Default::default(),
            widget::v0alpha1::font::Weight::Thin => iced::font::Weight::Thin,
            widget::v0alpha1::font::Weight::ExtraLight => iced::font::Weight::ExtraLight,
            widget::v0alpha1::font::Weight::Light => iced::font::Weight::Light,
            widget::v0alpha1::font::Weight::Normal => iced::font::Weight::Normal,
            widget::v0alpha1::font::Weight::Medium => iced::font::Weight::Medium,
            widget::v0alpha1::font::Weight::Semibold => iced::font::Weight::Semibold,
            widget::v0alpha1::font::Weight::Bold => iced::font::Weight::Bold,
            widget::v0alpha1::font::Weight::ExtraBold => iced::font::Weight::ExtraBold,
            widget::v0alpha1::font::Weight::Black => iced::font::Weight::Black,
        }
    }
}

impl FromApi<widget::v0alpha1::font::Stretch> for iced::font::Stretch {
    fn from_api(api_type: widget::v0alpha1::font::Stretch) -> Self {
        match api_type {
            widget::v0alpha1::font::Stretch::Unspecified => Default::default(),
            widget::v0alpha1::font::Stretch::UltraCondensed => iced::font::Stretch::UltraCondensed,
            widget::v0alpha1::font::Stretch::ExtraCondensed => iced::font::Stretch::ExtraCondensed,
            widget::v0alpha1::font::Stretch::Condensed => iced::font::Stretch::Condensed,
            widget::v0alpha1::font::Stretch::SemiCondensed => iced::font::Stretch::SemiCondensed,
            widget::v0alpha1::font::Stretch::Normal => iced::font::Stretch::Normal,
            widget::v0alpha1::font::Stretch::SemiExpanded => iced::font::Stretch::SemiExpanded,
            widget::v0alpha1::font::Stretch::Expanded => iced::font::Stretch::Expanded,
            widget::v0alpha1::font::Stretch::ExtraExpanded => iced::font::Stretch::ExtraExpanded,
            widget::v0alpha1::font::Stretch::UltraExpanded => iced::font::Stretch::UltraExpanded,
        }
    }
}

impl FromApi<widget::v0alpha1::font::Style> for iced::font::Style {
    fn from_api(api_type: widget::v0alpha1::font::Style) -> Self {
        match api_type {
            widget::v0alpha1::font::Style::Unspecified => Default::default(),
            widget::v0alpha1::font::Style::Normal => iced::font::Style::Normal,
            widget::v0alpha1::font::Style::Italic => iced::font::Style::Italic,
            widget::v0alpha1::font::Style::Oblique => iced::font::Style::Oblique,
        }
    }
}

impl FromApi<widget::v0alpha1::Font> for iced::Font {
    fn from_api(api_type: widget::v0alpha1::Font) -> Self {
        let weight = FromApi::from_api(api_type.weight());
        let stretch = FromApi::from_api(api_type.stretch());
        let style = FromApi::from_api(api_type.style());
        let family = api_type.family.map(FromApi::from_api).unwrap_or_default();

        iced::Font {
            family,
            weight,
            stretch,
            style,
        }
    }
}
