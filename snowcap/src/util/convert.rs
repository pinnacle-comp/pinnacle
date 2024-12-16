//! Utilities for converting to and from API types

use snowcap_api_defs::snowcap::widget;

pub trait FromApi {
    type ApiType;
    fn from_api(api_type: Self::ApiType) -> Self;
}

impl FromApi for iced::Length {
    type ApiType = widget::v0alpha1::Length;

    fn from_api(length: Self::ApiType) -> Self {
        use widget::v0alpha1::length::Strategy;
        match length.strategy.unwrap_or(Strategy::Fill(())) {
            Strategy::Fill(_) => iced::Length::Fill,
            Strategy::FillPortion(portion) => iced::Length::FillPortion(portion as u16),
            Strategy::Shrink(_) => iced::Length::Shrink,
            Strategy::Fixed(size) => iced::Length::Fixed(size),
        }
    }
}

impl FromApi for iced::Alignment {
    type ApiType = widget::v0alpha1::Alignment;

    fn from_api(api_type: Self::ApiType) -> Self {
        match api_type {
            widget::v0alpha1::Alignment::Unspecified => iced::Alignment::Start,
            widget::v0alpha1::Alignment::Start => iced::Alignment::Start,
            widget::v0alpha1::Alignment::Center => iced::Alignment::Center,
            widget::v0alpha1::Alignment::End => iced::Alignment::End,
        }
    }
}

impl FromApi for iced::widget::scrollable::Alignment {
    type ApiType = widget::v0alpha1::ScrollableAlignment;

    fn from_api(api_type: Self::ApiType) -> Self {
        match api_type {
            widget::v0alpha1::ScrollableAlignment::Unspecified => Self::default(),
            widget::v0alpha1::ScrollableAlignment::Start => {
                iced::widget::scrollable::Alignment::Start
            }
            widget::v0alpha1::ScrollableAlignment::End => iced::widget::scrollable::Alignment::End,
        }
    }
}

impl FromApi for iced::widget::scrollable::Properties {
    type ApiType = widget::v0alpha1::ScrollableProperties;

    fn from_api(api_type: Self::ApiType) -> Self {
        let mut properties = iced::widget::scrollable::Properties::new();
        let alignment = api_type.alignment();
        properties = properties.alignment(iced::widget::scrollable::Alignment::from_api(alignment));
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

impl FromApi for iced::widget::scrollable::Direction {
    type ApiType = widget::v0alpha1::ScrollableDirection;

    fn from_api(api_type: Self::ApiType) -> Self {
        use iced::widget::scrollable::Properties;
        match (api_type.vertical, api_type.horizontal) {
            (Some(vertical), Some(horizontal)) => Self::Both {
                vertical: Properties::from_api(vertical),
                horizontal: Properties::from_api(horizontal),
            },
            (Some(vertical), None) => Self::Vertical(Properties::from_api(vertical)),
            (None, Some(horizontal)) => Self::Horizontal(Properties::from_api(horizontal)),
            (None, None) => Self::default(),
        }
    }
}

impl FromApi for iced::Padding {
    type ApiType = widget::v0alpha1::Padding;

    fn from_api(api_type: Self::ApiType) -> Self {
        iced::Padding {
            top: api_type.top(),
            right: api_type.right(),
            bottom: api_type.bottom(),
            left: api_type.left(),
        }
    }
}

impl FromApi for iced::Color {
    type ApiType = widget::v0alpha1::Color;

    fn from_api(api_type: Self::ApiType) -> Self {
        iced::Color {
            r: api_type.red().clamp(0.0, 1.0),
            g: api_type.green().clamp(0.0, 1.0),
            b: api_type.blue().clamp(0.0, 1.0),
            a: api_type.alpha.unwrap_or(1.0).clamp(0.0, 1.0),
        }
    }
}

impl FromApi for iced::font::Family {
    type ApiType = widget::v0alpha1::font::Family;

    fn from_api(api_type: Self::ApiType) -> Self {
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

impl FromApi for iced::font::Weight {
    type ApiType = widget::v0alpha1::font::Weight;

    fn from_api(api_type: Self::ApiType) -> Self {
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

impl FromApi for iced::font::Stretch {
    type ApiType = widget::v0alpha1::font::Stretch;

    fn from_api(api_type: Self::ApiType) -> Self {
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

impl FromApi for iced::font::Style {
    type ApiType = widget::v0alpha1::font::Style;

    fn from_api(api_type: Self::ApiType) -> Self {
        match api_type {
            widget::v0alpha1::font::Style::Unspecified => Default::default(),
            widget::v0alpha1::font::Style::Normal => iced::font::Style::Normal,
            widget::v0alpha1::font::Style::Italic => iced::font::Style::Italic,
            widget::v0alpha1::font::Style::Oblique => iced::font::Style::Oblique,
        }
    }
}

impl FromApi for iced::Font {
    type ApiType = widget::v0alpha1::Font;

    fn from_api(api_type: Self::ApiType) -> Self {
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
