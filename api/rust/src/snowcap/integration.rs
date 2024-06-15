//! Pinnacle-specific integrations with Snowcap.
//!
//! This module includes builtin widgets like the exit prompt and keybind list.

use std::sync::OnceLock;

use snowcap_api::{
    layer::{ExclusiveZone, KeyboardInteractivity, ZLayer},
    widget::{
        font::{Family, Font, Weight},
        Alignment, Color, Column, Container, Length, Text,
    },
};
use xkbcommon::xkb::Keysym;

use crate::ApiModules;

/// Builtin widgets for Pinnacle.
pub struct Integration {
    api: OnceLock<ApiModules>,
}

impl Integration {
    pub(crate) fn new() -> Self {
        Self {
            api: OnceLock::new(),
        }
    }

    pub(crate) fn finish_init(&self, api: ApiModules) {
        self.api.set(api).unwrap();
    }

    /// Create the default quit prompt.
    ///
    /// Some of its characteristics can be changed by setting its fields.
    pub fn quit_prompt(&self) -> QuitPrompt {
        QuitPrompt {
            api: self.api.get().cloned().unwrap(),
            border_radius: 12.0,
            border_thickness: 6.0,
            background_color: [0.15, 0.03, 0.1, 0.65].into(),
            border_color: [0.8, 0.2, 0.4].into(),
            font: Font::new_with_family(Family::Name("Ubuntu".into())),
            width: 220,
            height: 120,
        }
    }
}

/// A quit prompt.
///
/// When opened, pressing ENTER will quit the compositor.
pub struct QuitPrompt {
    api: ApiModules,
    /// The radius of the prompt's corners.
    pub border_radius: f32,
    /// The thickness of the prompt border.
    pub border_thickness: f32,
    /// The color of the prompt background.
    pub background_color: Color,
    /// The color of the prompt border.
    pub border_color: Color,
    /// The font of the prompt.
    pub font: Font,
    /// The height of the prompt.
    pub width: u32,
    /// The width of the prompt.
    pub height: u32,
}

impl QuitPrompt {
    /// Show this quit prompt.
    pub fn show(&self) {
        let widget = Container::new(Column::new_with_children([
            Text::new("Quit Pinnacle?")
                .font(self.font.clone().weight(Weight::Bold))
                .size(20.0)
                .into(),
            Text::new("").size(8.0).into(), // Spacing because I haven't impl'd that yet
            Text::new("Press ENTER to confirm, or\nany other key to close this")
                .font(self.font.clone())
                .size(14.0)
                .into(),
        ]))
        .width(Length::Fill)
        .height(Length::Fill)
        .vertical_alignment(Alignment::Center)
        .horizontal_alignment(Alignment::Center)
        .border_radius(self.border_radius)
        .border_thickness(self.border_thickness)
        .border_color(self.border_color)
        .background_color(self.background_color);

        self.api
            .snowcap
            .layer
            .new_widget(
                widget,
                self.width,
                self.height,
                None,
                KeyboardInteractivity::Exclusive,
                ExclusiveZone::Respect,
                ZLayer::Overlay,
            )
            .on_key_press(|handle, key, _mods| {
                if key == Keysym::Return {
                    self.api.pinnacle.quit();
                } else {
                    handle.close();
                }
            });
    }
}
