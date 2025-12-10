use iced::widget::{
    Column, Container, Row, Scrollable, button, image::FilterMethod, scrollable::Scrollbar,
};
use snowcap_api_defs::snowcap::widget::{
    self,
    v1::{
        GetWidgetEventsRequest, GetWidgetEventsResponse, WidgetDef, get_widget_events_request,
        widget_def, widget_event, widget_service_server,
    },
};
use tonic::{Request, Response, Status};

use crate::{
    api::{ResponseStream, run_server_streaming_mapped},
    decoration::DecorationId,
    layer::LayerId,
    util::convert::FromApi,
    widget::{ViewFn, WidgetEvent, WidgetId},
};

#[tonic::async_trait]
impl widget_service_server::WidgetService for super::WidgetService {
    type GetWidgetEventsStream = ResponseStream<GetWidgetEventsResponse>;

    async fn get_widget_events(
        &self,
        request: Request<GetWidgetEventsRequest>,
    ) -> Result<Response<Self::GetWidgetEventsStream>, Status> {
        let Some(id) = request.into_inner().id else {
            return Err(Status::invalid_argument("no id specified"));
        };

        run_server_streaming_mapped(
            &self.sender,
            move |state, sender| match id {
                get_widget_events_request::Id::LayerId(layer_id) => {
                    if let Some(layer) = state.layer_for_id(LayerId(layer_id)) {
                        layer.surface.widget_event_sender = Some(sender);
                    }
                }
                get_widget_events_request::Id::DecorationId(decoration_id) => {
                    if let Some(deco) = state.decoration_for_id(DecorationId(decoration_id)) {
                        deco.surface.widget_event_sender = Some(sender);
                    }
                }
            },
            |events| {
                Ok(GetWidgetEventsResponse {
                    widget_events: events
                        .into_iter()
                        .map(|(id, event)| widget::v1::WidgetEvent {
                            widget_id: id.into_inner(),
                            event: Some(match event {
                                WidgetEvent::Button => {
                                    widget_event::Event::Button(widget::v1::button::Event {})
                                }
                            }),
                        })
                        .collect(),
                })
            },
        )
    }
}

pub fn widget_def_to_fn(def: WidgetDef) -> Option<ViewFn> {
    let def = def.widget?;
    match def {
        widget_def::Widget::Text(text_def) => {
            let horizontal_alignment = text_def.horizontal_alignment();
            let vertical_alignment = text_def.vertical_alignment();

            let widget::v1::Text {
                text,
                width,
                height,
                horizontal_alignment: _,
                vertical_alignment: _,
                style,
            } = text_def;

            let f: ViewFn = Box::new(move || {
                let mut text = iced::widget::Text::new(text.clone());
                if let Some(pixels) = style.as_ref().and_then(|style| style.pixels) {
                    text = text.size(pixels);
                }
                if let Some(width) = width {
                    text = text.width(iced::Length::from_api(width));
                }
                if let Some(height) = height {
                    text = text.height(iced::Length::from_api(height));
                }
                if let Some(color) = style.as_ref().and_then(|style| style.color) {
                    text = text.style(move |_| iced::widget::text::Style {
                        color: Some(iced::Color::from_api(color)),
                    });
                }

                match horizontal_alignment {
                    widget::v1::Alignment::Unspecified => (),
                    widget::v1::Alignment::Start => {
                        text = text.align_x(iced::alignment::Horizontal::Left)
                    }
                    widget::v1::Alignment::Center => {
                        text = text.align_x(iced::alignment::Horizontal::Center)
                    }
                    widget::v1::Alignment::End => {
                        text = text.align_x(iced::alignment::Horizontal::Right)
                    }
                }

                match vertical_alignment {
                    widget::v1::Alignment::Unspecified => (),
                    widget::v1::Alignment::Start => {
                        text = text.align_y(iced::alignment::Vertical::Top)
                    }
                    widget::v1::Alignment::Center => {
                        text = text.align_y(iced::alignment::Vertical::Center)
                    }
                    widget::v1::Alignment::End => {
                        text = text.align_y(iced::alignment::Vertical::Bottom)
                    }
                }

                if let Some(font) = style.as_ref().and_then(|s| s.font.clone()) {
                    text = text.font(iced::Font::from_api(font));
                }

                text.into()
            });
            Some(f)
        }
        widget_def::Widget::Column(widget::v1::Column {
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
                .flat_map(widget_def_to_fn)
                .collect::<Vec<_>>();

            let f: ViewFn = Box::new(move || {
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
                    column = column.push(child());
                }

                column.into()
            });

            Some(f)
        }
        widget_def::Widget::Row(widget::v1::Row {
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
                .flat_map(widget_def_to_fn)
                .collect::<Vec<_>>();

            let f: ViewFn = Box::new(move || {
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

                if let Some(widget::v1::Padding {
                    top,
                    right,
                    bottom,
                    left,
                }) = padding
                {
                    row = row.padding(iced::Padding {
                        top,
                        right,
                        bottom,
                        left,
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
                    row = row.push(child());
                }

                row.into()
            });

            Some(f)
        }
        widget_def::Widget::Scrollable(scrollable_def) => {
            let widget::v1::Scrollable {
                width,
                height,
                direction,
                child,
                style,
            } = *scrollable_def;

            let child_widget_fn = child.and_then(|def| widget_def_to_fn(*def));

            let f: ViewFn = Box::new(move || {
                let mut scrollable = Scrollable::new(
                    child_widget_fn
                        .as_ref()
                        .map(|child| child())
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
                scrollable = scrollable.style(move |theme, status| {
                    let mut s = iced::widget::scrollable::default(theme, status);
                    if let Some(container_style) = style.as_ref().and_then(|s| s.container_style) {
                        s.container = FromApi::from_api(container_style);
                    }
                    if let Some(v_rail) = style.as_ref().and_then(|s| s.vertical_rail) {
                        let widget::v1::scrollable::Rail {
                            background_color,
                            border,
                            scroller_color,
                            scroller_border,
                        } = v_rail;

                        if let Some(color) = background_color {
                            s.vertical_rail.background =
                                Some(iced::Background::Color(FromApi::from_api(color)));
                        }
                        if let Some(border) = border {
                            s.vertical_rail.border = FromApi::from_api(border);
                        }
                        if let Some(scroller_color) = scroller_color {
                            s.vertical_rail.scroller.background =
                                iced::Background::Color(FromApi::from_api(scroller_color));
                        }
                        if let Some(scroller_border) = scroller_border {
                            s.vertical_rail.scroller.border = FromApi::from_api(scroller_border);
                        }
                    }
                    if let Some(h_rail) = style.as_ref().and_then(|s| s.horizontal_rail) {
                        let widget::v1::scrollable::Rail {
                            background_color,
                            border,
                            scroller_color,
                            scroller_border,
                        } = h_rail;

                        if let Some(color) = background_color {
                            s.horizontal_rail.background =
                                Some(iced::Background::Color(FromApi::from_api(color)));
                        }
                        if let Some(border) = border {
                            s.horizontal_rail.border = FromApi::from_api(border);
                        }
                        if let Some(scroller_color) = scroller_color {
                            s.horizontal_rail.scroller.background =
                                iced::Background::Color(FromApi::from_api(scroller_color));
                        }
                        if let Some(scroller_border) = scroller_border {
                            s.horizontal_rail.scroller.border = FromApi::from_api(scroller_border);
                        }
                    }

                    s
                });

                scrollable.into()
            });

            Some(f)
        }
        widget_def::Widget::Container(container_def) => {
            let horizontal_alignment = container_def.horizontal_alignment();
            let vertical_alignment = container_def.vertical_alignment();

            let widget::v1::Container {
                padding,
                width,
                height,
                max_width,
                max_height,
                horizontal_alignment: _,
                vertical_alignment: _,
                clip,
                child,
                style,
            } = *container_def;

            let child_widget_fn = child.and_then(|def| widget_def_to_fn(*def));

            let f: ViewFn = Box::new(move || {
                let mut container = Container::new(
                    child_widget_fn
                        .as_ref()
                        .map(|child| child())
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
                    widget::v1::Alignment::Unspecified => iced::alignment::Horizontal::Left,
                    widget::v1::Alignment::Start => iced::alignment::Horizontal::Left,
                    widget::v1::Alignment::Center => iced::alignment::Horizontal::Center,
                    widget::v1::Alignment::End => iced::alignment::Horizontal::Right,
                });
                container = container.align_y(match vertical_alignment {
                    widget::v1::Alignment::Unspecified => iced::alignment::Vertical::Top,
                    widget::v1::Alignment::Start => iced::alignment::Vertical::Top,
                    widget::v1::Alignment::Center => iced::alignment::Vertical::Center,
                    widget::v1::Alignment::End => iced::alignment::Vertical::Bottom,
                });

                let text_color_clone = style.and_then(|s| s.text_color);
                let background_color_clone = style.and_then(|s| s.background_color);
                let border_color_clone = style.and_then(|s| s.border);

                let style = move |theme: &iced::Theme| {
                    let mut style =
                        <iced::Theme as iced::widget::container::Catalog>::default()(theme);

                    if let Some(text_color) = text_color_clone {
                        style.text_color = Some(iced::Color::from_api(text_color));
                    }

                    if let Some(background_color) = background_color_clone {
                        style.background = Some(iced::Color::from_api(background_color).into());
                    }

                    if let Some(border_color) = border_color_clone.and_then(|b| b.color) {
                        style.border.color = iced::Color::from_api(border_color);
                    }

                    if let Some(border_radius) = border_color_clone.and_then(|b| b.radius) {
                        style.border.radius = iced::border::Radius::from_api(border_radius);
                    }

                    if let Some(width) = border_color_clone.and_then(|b| b.width) {
                        style.border.width = width;
                    }

                    style
                };

                container = container.style(style);

                container.into()
            });

            Some(f)
        }
        widget_def::Widget::Button(button) => {
            let widget::v1::Button {
                child,
                width,
                height,
                padding,
                clip,
                style,
                widget_id,
            } = *button;

            let child_widget_fn = child.and_then(|def| widget_def_to_fn(*def));

            let f: ViewFn = Box::new(move || {
                let mut button = iced::widget::Button::new(
                    child_widget_fn
                        .as_ref()
                        .map(|child| child())
                        .unwrap_or_else(|| iced::widget::Text::new("NULL").into()),
                );

                if let Some(width) = width {
                    button = button.width(iced::Length::from_api(width));
                }
                if let Some(height) = height {
                    button = button.height(iced::Length::from_api(height));
                }
                if let Some(padding) = padding {
                    button = button.padding(iced::Padding::from_api(padding));
                }
                if let Some(clip) = clip {
                    button = button.clip(clip);
                }
                if let Some(widget_id) = widget_id {
                    button = button.on_press(crate::widget::SnowcapMessage::WidgetEvent(
                        WidgetId(widget_id),
                        WidgetEvent::Button,
                    ));
                }

                let style = move |theme: &iced::Theme, status| {
                    let mut s = <iced::Theme as button::Catalog>::default()(theme, status);

                    let inner = match status {
                        button::Status::Active => style.and_then(|s| s.active),
                        button::Status::Hovered => style.and_then(|s| s.hovered),
                        button::Status::Pressed => style.and_then(|s| s.pressed),
                        button::Status::Disabled => style.and_then(|s| s.disabled),
                    };

                    if let Some(style) = inner {
                        if let Some(text_color) = style.text_color {
                            s.text_color = FromApi::from_api(text_color);
                        }
                        if let Some(background_color) = style.background_color {
                            s.background =
                                Some(iced::Background::Color(FromApi::from_api(background_color)));
                        }
                        if let Some(border) = style.border {
                            s.border = FromApi::from_api(border);
                        }
                    }

                    s
                };

                button = button.style(style);

                button.into()
            });

            Some(f)
        }
        widget_def::Widget::Image(image) => {
            let content_fit = image.content_fit();

            let widget::v1::Image {
                nearest_neighbor,
                rotation_degrees,
                opacity,
                handle,
                width,
                height,
                expand,
                content_fit: _,
                scale,
            } = image;

            let handle = handle?;

            let f: ViewFn = Box::new(move || {
                // FIXME: don't clone the entire image
                let mut image = match handle.clone() {
                    widget::v1::image::Handle::Path(path) => {
                        iced::widget::Image::new(iced::widget::image::Handle::from_path(path))
                    }
                    widget::v1::image::Handle::Bytes(bytes) => {
                        iced::widget::Image::new(iced::widget::image::Handle::from_bytes(bytes))
                    }
                    widget::v1::image::Handle::Rgba(widget::v1::image::Rgba {
                        width,
                        height,
                        rgba,
                    }) => iced::widget::Image::new(iced::widget::image::Handle::from_rgba(
                        width, height, rgba,
                    )),
                };

                if let Some(true) = nearest_neighbor {
                    image = image.filter_method(FilterMethod::Nearest);
                }
                if let Some(degrees) = rotation_degrees {
                    image = image.rotation(iced::Radians::from(iced::Degrees::from(degrees)));
                }
                if let Some(opacity) = opacity {
                    image = image.opacity(opacity.clamp(0.0, 1.0));
                }
                if let Some(width) = width {
                    image = image.width(iced::Length::from_api(width));
                }
                if let Some(height) = height {
                    image = image.height(iced::Length::from_api(height));
                }
                if let Some(expand) = expand {
                    image = image.expand(expand);
                }

                let content_fit = match content_fit {
                    widget::v1::image::ContentFit::Unspecified => None,
                    widget::v1::image::ContentFit::Contain => Some(iced::ContentFit::Contain),
                    widget::v1::image::ContentFit::Cover => Some(iced::ContentFit::Cover),
                    widget::v1::image::ContentFit::Fill => Some(iced::ContentFit::Fill),
                    widget::v1::image::ContentFit::None => Some(iced::ContentFit::None),
                    widget::v1::image::ContentFit::ScaleDown => Some(iced::ContentFit::ScaleDown),
                };

                if let Some(content_fit) = content_fit {
                    image = image.content_fit(content_fit);
                }
                if let Some(scale) = scale {
                    image = image.scale(scale);
                }

                image.into()
            });

            Some(f)
        }
        widget_def::Widget::InputRegion(input_region) => {
            let widget::v1::InputRegion {
                add,
                width,
                height,
                child,
            } = *input_region;

            let child_widget_fn = child.and_then(|def| widget_def_to_fn(*def));

            let f: ViewFn = Box::new(move || {
                let mut input_region = crate::widget::input_region::InputRegion::new(
                    add,
                    child_widget_fn
                        .as_ref()
                        .map(|child| child())
                        .unwrap_or_else(|| iced::widget::Text::new("NULL").into()),
                );

                if let Some(width) = width {
                    input_region = input_region.width(iced::Length::from_api(width));
                }
                if let Some(height) = height {
                    input_region = input_region.height(iced::Length::from_api(height));
                }

                input_region.into()
            });

            Some(f)
        }
    }
}

impl FromApi<widget::v1::Length> for iced::Length {
    fn from_api(length: widget::v1::Length) -> Self {
        use widget::v1::length::Strategy;
        match length.strategy.unwrap_or(Strategy::Fill(())) {
            Strategy::Fill(_) => iced::Length::Fill,
            Strategy::FillPortion(portion) => iced::Length::FillPortion(portion as u16),
            Strategy::Shrink(_) => iced::Length::Shrink,
            Strategy::Fixed(size) => iced::Length::Fixed(size),
        }
    }
}

impl FromApi<widget::v1::Alignment> for iced::Alignment {
    fn from_api(api_type: widget::v1::Alignment) -> Self {
        match api_type {
            widget::v1::Alignment::Unspecified => iced::Alignment::Start,
            widget::v1::Alignment::Start => iced::Alignment::Start,
            widget::v1::Alignment::Center => iced::Alignment::Center,
            widget::v1::Alignment::End => iced::Alignment::End,
        }
    }
}

impl FromApi<widget::v1::scrollable::Scrollbar> for iced::widget::scrollable::Scrollbar {
    fn from_api(api_type: widget::v1::scrollable::Scrollbar) -> Self {
        let widget::v1::scrollable::Scrollbar {
            width_pixels,
            margin_pixels,
            scroller_width_pixels,
            anchor_to_end,
            embed_spacing,
        } = api_type;
        let mut scrollbar = iced::widget::scrollable::Scrollbar::new();
        if let Some(width) = width_pixels {
            scrollbar = scrollbar.width(width);
        }
        if let Some(margin) = margin_pixels {
            scrollbar = scrollbar.margin(margin);
        }
        if let Some(scroller_width) = scroller_width_pixels {
            scrollbar = scrollbar.scroller_width(scroller_width);
        }
        if let Some(true) = anchor_to_end {
            scrollbar = scrollbar.anchor(iced::widget::scrollable::Anchor::End);
        }
        if let Some(spacing) = embed_spacing {
            scrollbar = scrollbar.spacing(spacing);
        }
        scrollbar
    }
}

impl FromApi<widget::v1::scrollable::Direction> for iced::widget::scrollable::Direction {
    fn from_api(api_type: widget::v1::scrollable::Direction) -> Self {
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

impl FromApi<widget::v1::Padding> for iced::Padding {
    fn from_api(api_type: widget::v1::Padding) -> Self {
        iced::Padding {
            top: api_type.top,
            right: api_type.right,
            bottom: api_type.bottom,
            left: api_type.left,
        }
    }
}

impl FromApi<widget::v1::Color> for iced::Color {
    fn from_api(api_type: widget::v1::Color) -> Self {
        iced::Color {
            r: api_type.red.clamp(0.0, 1.0),
            g: api_type.green.clamp(0.0, 1.0),
            b: api_type.blue.clamp(0.0, 1.0),
            a: api_type.alpha.clamp(0.0, 1.0),
        }
    }
}

impl FromApi<widget::v1::font::Family> for iced::font::Family {
    fn from_api(api_type: widget::v1::font::Family) -> Self {
        match api_type.family {
            Some(family) => match family {
                widget::v1::font::family::Family::Name(name) => {
                    iced::font::Family::Name(name.leak()) // why does this take &'static str
                }
                widget::v1::font::family::Family::Serif(_) => iced::font::Family::Serif,
                widget::v1::font::family::Family::SansSerif(_) => iced::font::Family::SansSerif,
                widget::v1::font::family::Family::Cursive(_) => iced::font::Family::Cursive,
                widget::v1::font::family::Family::Fantasy(_) => iced::font::Family::Fantasy,
                widget::v1::font::family::Family::Monospace(_) => iced::font::Family::Monospace,
            },
            None => Default::default(),
        }
    }
}

impl FromApi<widget::v1::font::Weight> for iced::font::Weight {
    fn from_api(api_type: widget::v1::font::Weight) -> Self {
        match api_type {
            widget::v1::font::Weight::Unspecified => Default::default(),
            widget::v1::font::Weight::Thin => iced::font::Weight::Thin,
            widget::v1::font::Weight::ExtraLight => iced::font::Weight::ExtraLight,
            widget::v1::font::Weight::Light => iced::font::Weight::Light,
            widget::v1::font::Weight::Normal => iced::font::Weight::Normal,
            widget::v1::font::Weight::Medium => iced::font::Weight::Medium,
            widget::v1::font::Weight::Semibold => iced::font::Weight::Semibold,
            widget::v1::font::Weight::Bold => iced::font::Weight::Bold,
            widget::v1::font::Weight::ExtraBold => iced::font::Weight::ExtraBold,
            widget::v1::font::Weight::Black => iced::font::Weight::Black,
        }
    }
}

impl FromApi<widget::v1::font::Stretch> for iced::font::Stretch {
    fn from_api(api_type: widget::v1::font::Stretch) -> Self {
        match api_type {
            widget::v1::font::Stretch::Unspecified => Default::default(),
            widget::v1::font::Stretch::UltraCondensed => iced::font::Stretch::UltraCondensed,
            widget::v1::font::Stretch::ExtraCondensed => iced::font::Stretch::ExtraCondensed,
            widget::v1::font::Stretch::Condensed => iced::font::Stretch::Condensed,
            widget::v1::font::Stretch::SemiCondensed => iced::font::Stretch::SemiCondensed,
            widget::v1::font::Stretch::Normal => iced::font::Stretch::Normal,
            widget::v1::font::Stretch::SemiExpanded => iced::font::Stretch::SemiExpanded,
            widget::v1::font::Stretch::Expanded => iced::font::Stretch::Expanded,
            widget::v1::font::Stretch::ExtraExpanded => iced::font::Stretch::ExtraExpanded,
            widget::v1::font::Stretch::UltraExpanded => iced::font::Stretch::UltraExpanded,
        }
    }
}

impl FromApi<widget::v1::font::Style> for iced::font::Style {
    fn from_api(api_type: widget::v1::font::Style) -> Self {
        match api_type {
            widget::v1::font::Style::Unspecified => Default::default(),
            widget::v1::font::Style::Normal => iced::font::Style::Normal,
            widget::v1::font::Style::Italic => iced::font::Style::Italic,
            widget::v1::font::Style::Oblique => iced::font::Style::Oblique,
        }
    }
}

impl FromApi<widget::v1::Font> for iced::Font {
    fn from_api(api_type: widget::v1::Font) -> Self {
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

impl FromApi<widget::v1::Radius> for iced::border::Radius {
    fn from_api(api_type: widget::v1::Radius) -> Self {
        Self {
            top_left: api_type.top_left,
            top_right: api_type.top_right,
            bottom_right: api_type.bottom_right,
            bottom_left: api_type.bottom_left,
        }
    }
}

impl FromApi<widget::v1::container::Style> for iced::widget::container::Style {
    fn from_api(api_type: widget::v1::container::Style) -> Self {
        let mut ret = Self::default();

        let widget::v1::container::Style {
            text_color,
            background_color,
            border,
        } = api_type;

        if let Some(color) = text_color {
            ret.text_color = Some(FromApi::from_api(color));
        }
        if let Some(color) = background_color {
            ret.background = Some(iced::Background::Color(FromApi::from_api(color)));
        }
        if let Some(border) = border {
            ret.border = FromApi::from_api(border);
        }

        ret
    }
}

impl FromApi<widget::v1::Border> for iced::Border {
    fn from_api(api_type: widget::v1::Border) -> Self {
        let mut ret = Self::default();

        let widget::v1::Border {
            color,
            width,
            radius,
        } = api_type;

        if let Some(color) = color {
            ret.color = FromApi::from_api(color);
        }
        if let Some(width) = width {
            ret.width = width;
        }
        if let Some(radius) = radius {
            ret.radius = FromApi::from_api(radius);
        }

        ret
    }
}
