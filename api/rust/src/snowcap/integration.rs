//! Pinnacle-specific integrations with Snowcap.
//!
//! This module includes builtin widgets like the exit prompt and keybind list.

use indexmap::IndexMap;
use snowcap_api::{
    layer::{ExclusiveZone, KeyboardInteractivity, ZLayer},
    widget::{
        font::{Family, Font, Weight},
        Alignment, Color, Column, Container, Length, Padding, Row, Scrollable, Text, WidgetDef,
    },
};
use xkbcommon::xkb::Keysym;

use crate::input::{KeybindDescription, Mod};

/// Builtin widgets for Pinnacle.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Integration;

impl Integration {
    /// Create the default quit prompt.
    ///
    /// Some of its characteristics can be changed by setting its fields.
    pub fn quit_prompt(&self) -> QuitPrompt {
        QuitPrompt {
            border_radius: 12.0,
            border_thickness: 6.0,
            background_color: [0.15, 0.03, 0.1, 0.65].into(),
            border_color: [0.8, 0.2, 0.4].into(),
            font: Font::new_with_family(Family::Name("Ubuntu".into())),
            width: 220,
            height: 120,
        }
    }

    /// Create the default keybind overlay.
    ///
    /// Some of its characteristics can be changed by setting its fields.
    pub fn keybind_overlay(&self) -> KeybindOverlay {
        KeybindOverlay {
            border_radius: 12.0,
            border_thickness: 6.0,
            background_color: [0.15, 0.15, 0.225, 0.8].into(),
            border_color: [0.4, 0.4, 0.7].into(),
            font: Font::new_with_family(Family::Name("Ubuntu".into())),
            width: 700,
            height: 500,
        }
    }
}

/// A quit prompt.
///
/// When opened, pressing ENTER will quit the compositor.
pub struct QuitPrompt {
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

        snowcap_api::layer::Layer
            .new_widget(
                widget,
                self.width,
                self.height,
                None,
                KeyboardInteractivity::Exclusive,
                ExclusiveZone::Respect,
                ZLayer::Overlay,
            )
            .unwrap()
            .on_key_press(|handle, key, _mods| {
                if key == Keysym::Return {
                    crate::pinnacle::quit();
                } else {
                    handle.close();
                }
            });
    }
}

/// A keybind overlay.
pub struct KeybindOverlay {
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

impl KeybindOverlay {
    /// Show this keybind overlay.
    pub fn show(&self) {
        // TODO:
        // FIXME:
        // let descriptions = Input.keybind_descriptions();
        //
        // #[derive(PartialEq, Eq, Hash)]
        // struct KeybindRepr {
        //     mods: Vec<Mod>,
        //     name: String,
        // }
        //
        // impl std::fmt::Display for KeybindRepr {
        //     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //         let mut parts = Vec::new();
        //         if self.mods.contains(&Mod::Super) {
        //             parts.push("Super");
        //         }
        //         if self.mods.contains(&Mod::Ctrl) {
        //             parts.push("Ctrl");
        //         }
        //         if self.mods.contains(&Mod::Alt) {
        //             parts.push("Alt");
        //         }
        //         if self.mods.contains(&Mod::Shift) {
        //             parts.push("Shift");
        //         }
        //
        //         parts.push(self.name.as_str());
        //
        //         let bind = parts.join(" + ");
        //         write!(f, "{bind}")
        //     }
        // }
        //
        // #[derive(Default)]
        // struct GroupData {
        //     binds: IndexMap<KeybindRepr, Vec<String>>,
        // }
        //
        // let mut groups = IndexMap::<Option<String>, GroupData>::new();
        //
        // for desc in descriptions {
        //     let KeybindDescription {
        //         modifiers,
        //         key_code: _,
        //         xkb_name,
        //         group,
        //         description,
        //     } = desc;
        //
        //     let repr = KeybindRepr {
        //         mods: modifiers,
        //         name: xkb_name,
        //     };
        //
        //     let group = groups.entry(group).or_default();
        //
        //     let descs = group.binds.entry(repr).or_default();
        //
        //     if let Some(desc) = description {
        //         descs.push(desc);
        //     }
        // }
        //
        // // List keybinds with no group last
        // if let Some(data) = groups.shift_remove(&None) {
        //     groups.insert(None, data);
        // }
        //
        // let sections = groups.into_iter().flat_map(|(group, data)| {
        //     let group_title = Text::new(group.unwrap_or("Other".into()))
        //         .font(self.font.clone().weight(Weight::Bold))
        //         .size(19.0);
        //
        //     let binds = data.binds.into_iter().map(|(key, descs)| {
        //         if descs.is_empty() {
        //             WidgetDef::from(Text::new(key.to_string()).font(self.font.clone()))
        //         } else if descs.len() == 1 {
        //             Row::new_with_children([
        //                 Text::new(key.to_string())
        //                     .width(Length::FillPortion(1))
        //                     .font(self.font.clone())
        //                     .into(),
        //                 Text::new(descs[0].clone())
        //                     .width(Length::FillPortion(2))
        //                     .font(self.font.clone())
        //                     .into(),
        //             ])
        //             .into()
        //         } else {
        //             let mut children = Vec::<WidgetDef>::new();
        //             children.push(
        //                 Text::new(key.to_string() + ":")
        //                     .font(self.font.clone())
        //                     .into(),
        //             );
        //
        //             for desc in descs {
        //                 children.push(
        //                     Text::new(format!("\t{}", desc))
        //                         .font(self.font.clone())
        //                         .into(),
        //                 );
        //             }
        //
        //             Column::new_with_children(children).into()
        //         }
        //     });
        //
        //     let mut children = Vec::<WidgetDef>::new();
        //     children.push(group_title.into());
        //     for widget in binds {
        //         children.push(widget);
        //     }
        //     children.push(Text::new("").size(8.0).into()); // Spacing because I haven't impl'd that yet
        //
        //     children
        // });
        //
        // let scrollable = Scrollable::new(Column::new_with_children(sections))
        //     .width(Length::Fill)
        //     .height(Length::Fill);
        //
        // let widget = Container::new(Column::new_with_children([
        //     Text::new("Keybinds")
        //         .font(self.font.clone().weight(Weight::Bold))
        //         .size(24.0)
        //         .width(Length::Fill)
        //         .into(),
        //     Text::new("").size(8.0).into(), // Spacing because I haven't impl'd that yet
        //     scrollable.into(),
        // ]))
        // .width(Length::Fill)
        // .height(Length::Fill)
        // .padding(Padding {
        //     top: 16.0,
        //     right: 16.0,
        //     bottom: 16.0,
        //     left: 16.0,
        // })
        // .vertical_alignment(Alignment::Center)
        // .horizontal_alignment(Alignment::Center)
        // .border_radius(self.border_radius)
        // .border_thickness(self.border_thickness)
        // .border_color(self.border_color)
        // .background_color(self.background_color);
        //
        // snowcap_api::layer::Layer
        //     .new_widget(
        //         widget,
        //         self.width,
        //         self.height,
        //         None,
        //         KeyboardInteractivity::Exclusive,
        //         ExclusiveZone::Respect,
        //         ZLayer::Top,
        //     )
        //     .unwrap()
        //     .on_key_press(|handle, _key, _mods| {
        //         handle.close();
        //     });
    }
}
