//! Integration with the
//! [Snowcap](https://github.com/pinnacle-comp/pinnacle/tree/main/snowcap) widget system.
//!
//! Snowcap is a really-early-in-development widget system, designed for Pinnacle.
//! This module contains preliminary widgets made with the system.

use indexmap::IndexMap;
use snowcap_api::{
    layer::{ExclusiveZone, KeyboardInteractivity, ZLayer},
    widget::{
        Alignment, Color, Length, Padding, Program, WidgetDef,
        column::Column,
        container::Container,
        font::{Family, Font, Weight},
        row::Row,
        scrollable::Scrollable,
        text::{self, Text},
    },
};
use xkbcommon::xkb::Keysym;

use crate::input::{BindInfoKind, Mod};

/// A quit prompt.
///
/// When opened, pressing ENTER will quit the compositor.
#[derive(Default, Clone, Debug)]
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
    /// The width of the prompt.
    pub width: u32,
    /// The height of the prompt.
    pub height: u32,
}

impl Program for QuitPrompt {
    type Message = ();

    fn update(&mut self, _msg: Self::Message) {}

    fn view(&self) -> WidgetDef<Self::Message> {
        let widget = Container::new(Column::new_with_children([
            Text::new("Quit Pinnacle?")
                .style(
                    text::Style::new()
                        .font(self.font.clone().weight(Weight::Bold))
                        .pixels(20.0),
                )
                .into(),
            Text::new("").style(text::Style::new().pixels(8.0)).into(), // Spacing
            Text::new("Press ENTER to confirm, or\nany other key to close this")
                .style(text::Style::new().font(self.font.clone()).pixels(14.0))
                .into(),
        ]))
        .width(Length::Fixed(self.width as f32))
        .height(Length::Fixed(self.height as f32))
        .vertical_alignment(Alignment::Center)
        .horizontal_alignment(Alignment::Center)
        .style(snowcap_api::widget::container::Style {
            text_color: None,
            background_color: Some(self.background_color),
            border: Some(snowcap_api::widget::Border {
                color: Some(self.border_color),
                width: Some(self.border_thickness),
                radius: Some(self.border_radius.into()),
            }),
        });

        widget.into()
    }
}

impl QuitPrompt {
    /// Creates a quit prompt with sane defaults.
    pub fn new() -> Self {
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

    /// Shows this quit prompt.
    pub fn show(self) {
        snowcap_api::layer::new_widget(
            self,
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

/// A bindings overlay.
#[derive(Default, Clone, Debug)]
pub struct BindOverlay {
    /// The radius of the overlay's corners.
    pub border_radius: f32,
    /// The thickness of the overlay border.
    pub border_thickness: f32,
    /// The color of the overlay background.
    pub background_color: Color,
    /// The color of the overlay border.
    pub border_color: Color,
    /// The font of the overlay.
    pub font: Font,
    /// The width of the overlay.
    pub width: u32,
    /// The height of the overlay.
    pub height: u32,
}

impl Program for BindOverlay {
    type Message = ();

    fn update(&mut self, _msg: Self::Message) {}

    fn view(&self) -> WidgetDef<Self::Message> {
        #[derive(PartialEq, Eq, Hash)]
        struct KeybindRepr {
            mods: Mod,
            key_name: String,
            layer: Option<String>,
        }

        impl std::fmt::Display for KeybindRepr {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mods = format_mods(self.mods);

                let layer = self
                    .layer
                    .as_ref()
                    .map(|layer| format!("[{layer}] "))
                    .unwrap_or_default();

                let bind = mods
                    .as_deref()
                    .into_iter()
                    .chain([self.key_name.as_str()])
                    .collect::<Vec<_>>()
                    .join(" + ");
                write!(f, "{layer}{bind}")
            }
        }

        #[derive(PartialEq, Eq, Hash)]
        struct MousebindRepr {
            mods: Mod,
            button_name: String,
            layer: Option<String>,
        }

        impl std::fmt::Display for MousebindRepr {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mods = format_mods(self.mods);

                let layer = self
                    .layer
                    .as_ref()
                    .map(|layer| format!("[{layer}] "))
                    .unwrap_or_default();

                let bind = mods
                    .as_deref()
                    .into_iter()
                    .chain([self.button_name.as_str()])
                    .collect::<Vec<_>>()
                    .join(" + ");
                write!(f, "{layer}{bind}")
            }
        }

        #[derive(Default)]
        struct GroupBinds {
            /// keybinds to descriptions
            keybinds: IndexMap<KeybindRepr, Vec<String>>,
            /// mousebinds to descriptions
            mousebinds: IndexMap<MousebindRepr, Vec<String>>,
        }

        let bind_infos = crate::input::bind_infos();

        let mut groups = IndexMap::<String, GroupBinds>::new();

        for bind_info in bind_infos {
            let mods = bind_info.mods;
            let group = bind_info.group;
            let desc = bind_info.description;
            let layer = bind_info.layer.name();

            let group = groups.entry(group).or_default();

            match bind_info.kind {
                BindInfoKind::Key {
                    key_code: _,
                    xkb_name,
                } => {
                    let repr = KeybindRepr {
                        mods,
                        key_name: xkb_name,
                        layer,
                    };
                    let descs = group.keybinds.entry(repr).or_default();
                    if !desc.is_empty() {
                        descs.push(desc);
                    }
                }
                BindInfoKind::Mouse { button } => {
                    let repr = MousebindRepr {
                        mods,
                        button_name: match button {
                            crate::input::MouseButton::Left => "Mouse Left",
                            crate::input::MouseButton::Right => "Mouse Right",
                            crate::input::MouseButton::Middle => "Mouse Middle",
                            crate::input::MouseButton::Side => "Mouse Side",
                            crate::input::MouseButton::Extra => "Mouse Extra",
                            crate::input::MouseButton::Forward => "Mouse Forward",
                            crate::input::MouseButton::Back => "Mouse Back",
                            crate::input::MouseButton::Other(_) => "Mouse Other",
                        }
                        .to_string(),
                        layer,
                    };
                    let descs = group.mousebinds.entry(repr).or_default();
                    if !desc.is_empty() {
                        descs.push(desc);
                    }
                }
            }
        }

        // List keybinds with no group last
        if let Some(data) = groups.shift_remove("") {
            groups.insert("".to_string(), data);
        }

        let sections = groups.into_iter().flat_map(|(group, data)| {
            let group_title = Text::new(if !group.is_empty() { group } else { "Other".into() })
                .style(
                    text::Style::new()
                        .font(self.font.clone().weight(Weight::Bold))
                        .pixels(19.0),
                );

            let keybinds = data.keybinds.into_iter().map(|(key, descs)| {
                if descs.is_empty() {
                    WidgetDef::from(
                        Text::new(key.to_string())
                            .style(text::Style::new().font(self.font.clone())),
                    )
                } else if descs.len() == 1 {
                    Row::new_with_children([
                        Text::new(key.to_string())
                            .width(Length::FillPortion(1))
                            .style(text::Style::new().font(self.font.clone()))
                            .into(),
                        Text::new(descs[0].clone())
                            .width(Length::FillPortion(2))
                            .style(text::Style::new().font(self.font.clone()))
                            .into(),
                    ])
                    .into()
                } else {
                    let mut children = Vec::<WidgetDef<()>>::new();
                    children.push(
                        Text::new(key.to_string() + ":")
                            .style(text::Style::new().font(self.font.clone()))
                            .into(),
                    );

                    for desc in descs {
                        children.push(
                            Text::new(format!("\t{desc}"))
                                .style(text::Style::new().font(self.font.clone()))
                                .into(),
                        );
                    }

                    Column::new_with_children(children).into()
                }
            });

            let mousebinds = data.mousebinds.into_iter().map(|(mouse, descs)| {
                if descs.is_empty() {
                    WidgetDef::from(
                        Text::new(mouse.to_string())
                            .style(text::Style::new().font(self.font.clone())),
                    )
                } else if descs.len() == 1 {
                    Row::new_with_children([
                        Text::new(mouse.to_string())
                            .width(Length::FillPortion(1))
                            .style(text::Style::new().font(self.font.clone()))
                            .into(),
                        Text::new(descs[0].clone())
                            .width(Length::FillPortion(2))
                            .style(text::Style::new().font(self.font.clone()))
                            .into(),
                    ])
                    .into()
                } else {
                    let mut children = Vec::<WidgetDef<()>>::new();
                    children.push(
                        Text::new(mouse.to_string() + ":")
                            .style(text::Style::new().font(self.font.clone()))
                            .into(),
                    );

                    for desc in descs {
                        children.push(
                            Text::new(format!("\t{desc}"))
                                .style(text::Style::new().font(self.font.clone()))
                                .into(),
                        );
                    }

                    Column::new_with_children(children).into()
                }
            });

            let mut children = Vec::<WidgetDef<()>>::new();
            children.push(group_title.into());
            children.extend(keybinds);
            children.extend(mousebinds);
            children.push(Text::new("").style(text::Style::new().pixels(8.0)).into()); // Spacing because I haven't impl'd that yet

            children
        });

        let scrollable = Scrollable::new(Column::new_with_children(sections))
            .width(Length::Fill)
            .height(Length::Fill);

        let widget = Container::new(Column::new_with_children([
            Text::new("Keybinds")
                .style(
                    text::Style::new()
                        .font(self.font.clone().weight(Weight::Bold))
                        .pixels(24.0),
                )
                .width(Length::Fill)
                .into(),
            Text::new("").style(text::Style::new().pixels(8.0)).into(), // Spacing
            scrollable.into(),
        ]))
        .width(Length::Fixed(self.width as f32))
        .height(Length::Fixed(self.height as f32))
        .padding(Padding {
            top: self.border_thickness + 10.0,
            right: self.border_thickness + 10.0,
            bottom: self.border_thickness + 10.0,
            left: self.border_thickness + 10.0,
        })
        .vertical_alignment(Alignment::Center)
        .horizontal_alignment(Alignment::Center)
        .style(snowcap_api::widget::container::Style {
            text_color: None,
            background_color: Some(self.background_color),
            border: Some(snowcap_api::widget::Border {
                color: Some(self.border_color),
                width: Some(self.border_thickness),
                radius: Some(self.border_radius.into()),
            }),
        });

        widget.into()
    }
}

impl BindOverlay {
    /// Creates the default bind overlay.
    ///
    /// Some of its characteristics can be changed by setting its fields.
    pub fn new() -> Self {
        BindOverlay {
            border_radius: 12.0,
            border_thickness: 6.0,
            background_color: [0.15, 0.15, 0.225, 0.8].into(),
            border_color: [0.4, 0.4, 0.7].into(),
            font: Font::new_with_family(Family::Name("Ubuntu".into())),
            width: 700,
            height: 500,
        }
    }

    /// Shows this bind overlay.
    pub fn show(self) {
        snowcap_api::layer::new_widget(
            self,
            None,
            KeyboardInteractivity::Exclusive,
            ExclusiveZone::Respect,
            ZLayer::Top,
        )
        .unwrap()
        .on_key_press(|handle, _key, _mods| {
            handle.close();
        });
    }
}

fn format_mods(mods: Mod) -> Option<String> {
    let mut parts = Vec::new();
    if mods.contains(Mod::SUPER) {
        parts.push("Super");
    }
    if mods.contains(Mod::CTRL) {
        parts.push("Ctrl");
    }
    if mods.contains(Mod::ALT) {
        parts.push("Alt");
    }
    if mods.contains(Mod::SHIFT) {
        parts.push("Shift");
    }
    if mods.contains(Mod::ISO_LEVEL3_SHIFT) {
        parts.push("ISO Level 3 Shift");
    }
    if mods.contains(Mod::ISO_LEVEL5_SHIFT) {
        parts.push("ISO Level 5 Shift");
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" + "))
    }
}

/// A message that the previous config crashed.
#[derive(Default, Clone, Debug)]
pub struct ConfigCrashedMessage {
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
    /// The width of the prompt.
    pub width: u32,
    /// The height of the prompt.
    pub height: u32,
    /// The error message.
    pub message: String,
}

impl Program for ConfigCrashedMessage {
    type Message = ();

    fn update(&mut self, _msg: Self::Message) {}

    fn view(&self) -> WidgetDef<Self::Message> {
        let widget = Container::new(Column::new_with_children([
            Text::new("Config crashed!")
                .style(
                    text::Style::new()
                        .font(self.font.clone().weight(Weight::Bold))
                        .pixels(20.0),
                )
                .into(),
            Text::new("").style(text::Style::new().pixels(8.0)).into(), // Spacing
            Text::new("The previous config crashed with the following error message:")
                .style(text::Style::new().font(self.font.clone()).pixels(14.0))
                .into(),
            Text::new("").style(text::Style::new().pixels(8.0)).into(), // Spacing
            Scrollable::new(
                Text::new(&self.message)
                    .style(text::Style::new().font(self.font.clone()).pixels(14.0)),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
            Text::new("").style(text::Style::new().pixels(8.0)).into(), // Spacing
            Text::new(
                "ESCAPE/ENTER: Close this window. MOD + S: Bring up the bind overlay.\n\
                    MOD + CTRL + R: Restart your config.",
            )
            .style(text::Style::new().font(self.font.clone()).pixels(14.0))
            .into(),
        ]))
        .width(Length::Fixed(self.width as f32))
        .height(Length::Fixed(self.height as f32))
        .padding(Padding {
            top: 16.0,
            right: 16.0,
            bottom: 16.0,
            left: 16.0,
        })
        .vertical_alignment(Alignment::Center)
        .horizontal_alignment(Alignment::Center)
        .style(snowcap_api::widget::container::Style {
            text_color: None,
            background_color: Some(self.background_color),
            border: Some(snowcap_api::widget::Border {
                color: Some(self.border_color),
                width: Some(self.border_thickness),
                radius: Some(self.border_radius.into()),
            }),
        });

        widget.into()
    }
}

impl ConfigCrashedMessage {
    /// Creates an error message.
    pub fn new(message: impl std::fmt::Display) -> Self {
        ConfigCrashedMessage {
            border_radius: 12.0,
            border_thickness: 6.0,
            background_color: [0.15, 0.03, 0.1, 0.65].into(),
            border_color: [0.8, 0.2, 0.4].into(),
            font: Font::new_with_family(Family::Name("Ubuntu".into())),
            width: 700,
            height: 400,
            message: message.to_string(),
        }
    }

    /// Shows an error message.
    pub fn show(self) {
        snowcap_api::layer::new_widget(
            self,
            None,
            KeyboardInteractivity::Exclusive,
            ExclusiveZone::Respect,
            ZLayer::Overlay,
        )
        .unwrap()
        .on_key_press(|handle, key, _mods| {
            if key == Keysym::Return || key == Keysym::Escape {
                handle.close();
            }
        });
    }
}
