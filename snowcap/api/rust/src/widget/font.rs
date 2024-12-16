//! Font utilities and types.

use snowcap_api_defs::snowcap::widget;

/// A font specification.
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct Font {
    /// The font family.
    pub family: Family,
    /// The font weight.
    pub weight: Weight,
    /// The font stretch.
    pub stretch: Stretch,
    /// The font style.
    pub style: Style,
}

impl Font {
    /// Create a new, empty font specification.
    pub fn new() -> Self {
        Default::default()
    }

    /// Create a new font specification with the given family.
    pub fn new_with_family(family: Family) -> Self {
        Self {
            family,
            ..Default::default()
        }
    }

    /// Set this font's family.
    pub fn family(self, family: Family) -> Self {
        Self { family, ..self }
    }

    /// Set this font's weight.
    pub fn weight(self, weight: Weight) -> Self {
        Self { weight, ..self }
    }

    /// Set this font's stretch.
    pub fn stretch(self, stretch: Stretch) -> Self {
        Self { stretch, ..self }
    }

    /// Set this font's style.
    pub fn style(self, style: Style) -> Self {
        Self { style, ..self }
    }
}

impl From<Font> for widget::v0alpha1::Font {
    fn from(value: Font) -> Self {
        Self {
            family: Some(value.family.into()),
            weight: Some(widget::v0alpha1::font::Weight::from(value.weight) as i32),
            stretch: Some(widget::v0alpha1::font::Stretch::from(value.stretch) as i32),
            style: Some(widget::v0alpha1::font::Style::from(value.style) as i32),
        }
    }
}

/// A font family.
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub enum Family {
    /// A named font, like JetBrainsMono or FreeSerif.
    Name(String),
    Serif,
    #[default]
    SansSerif,
    Cursive,
    Fantasy,
    Monospace,
}

impl From<Family> for widget::v0alpha1::font::Family {
    fn from(value: Family) -> Self {
        Self {
            family: Some(match value {
                Family::Name(name) => widget::v0alpha1::font::family::Family::Name(name),
                Family::Serif => widget::v0alpha1::font::family::Family::Serif(()),
                Family::SansSerif => widget::v0alpha1::font::family::Family::SansSerif(()),
                Family::Cursive => widget::v0alpha1::font::family::Family::Cursive(()),
                Family::Fantasy => widget::v0alpha1::font::family::Family::Fantasy(()),
                Family::Monospace => widget::v0alpha1::font::family::Family::Monospace(()),
            }),
        }
    }
}

/// A font weight.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Weight {
    Thin,
    ExtraLight,
    Light,
    #[default]
    Normal,
    Medium,
    Semibold,
    Bold,
    ExtraBold,
    Black,
}

impl From<Weight> for widget::v0alpha1::font::Weight {
    fn from(value: Weight) -> Self {
        match value {
            Weight::Thin => widget::v0alpha1::font::Weight::Thin,
            Weight::ExtraLight => widget::v0alpha1::font::Weight::ExtraLight,
            Weight::Light => widget::v0alpha1::font::Weight::Light,
            Weight::Normal => widget::v0alpha1::font::Weight::Normal,
            Weight::Medium => widget::v0alpha1::font::Weight::Medium,
            Weight::Semibold => widget::v0alpha1::font::Weight::Semibold,
            Weight::Bold => widget::v0alpha1::font::Weight::Bold,
            Weight::ExtraBold => widget::v0alpha1::font::Weight::ExtraBold,
            Weight::Black => widget::v0alpha1::font::Weight::Black,
        }
    }
}

/// A font stretch.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Stretch {
    UltraCondensed,
    ExtraCondensed,
    Condensed,
    SemiCondensed,
    #[default]
    Normal,
    SemiExpanded,
    Expanded,
    ExtraExpanded,
    UltraExpanded,
}

impl From<Stretch> for widget::v0alpha1::font::Stretch {
    fn from(value: Stretch) -> Self {
        match value {
            Stretch::UltraCondensed => widget::v0alpha1::font::Stretch::UltraCondensed,
            Stretch::ExtraCondensed => widget::v0alpha1::font::Stretch::ExtraCondensed,
            Stretch::Condensed => widget::v0alpha1::font::Stretch::Condensed,
            Stretch::SemiCondensed => widget::v0alpha1::font::Stretch::SemiCondensed,
            Stretch::Normal => widget::v0alpha1::font::Stretch::Normal,
            Stretch::SemiExpanded => widget::v0alpha1::font::Stretch::SemiExpanded,
            Stretch::Expanded => widget::v0alpha1::font::Stretch::Expanded,
            Stretch::ExtraExpanded => widget::v0alpha1::font::Stretch::ExtraExpanded,
            Stretch::UltraExpanded => widget::v0alpha1::font::Stretch::UltraExpanded,
        }
    }
}

/// A font style.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Style {
    #[default]
    Normal,
    Italic,
    Oblique,
}

impl From<Style> for widget::v0alpha1::font::Style {
    fn from(value: Style) -> Self {
        match value {
            Style::Normal => widget::v0alpha1::font::Style::Normal,
            Style::Italic => widget::v0alpha1::font::Style::Italic,
            Style::Oblique => widget::v0alpha1::font::Style::Oblique,
        }
    }
}
