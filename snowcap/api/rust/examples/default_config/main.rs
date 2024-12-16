#![allow(missing_docs)]

use snowcap_api::{
    layer::{ExclusiveZone, KeyboardInteractivity, ZLayer},
    widget::{
        font::{Family, Font, Weight},
        Alignment, Color, Column, Container, Length, Padding, Row, Text,
    },
};

#[tokio::main]
async fn main() {
    let layer = snowcap_api::connect().await.unwrap();

    let test_key_descs = [
        ("Super + Enter", "Open alacritty"),
        ("Super + M", "Toggle maximized"),
        ("Super + F", "Toggle fullscreen"),
        ("Super + Shift + Q", "Exit Pinnacle"),
    ];

    let widget = Container::new(Row::new_with_children([
        Column::new_with_children(
            test_key_descs
                .iter()
                .map(|(keys, _)| Text::new(keys).into()),
        )
        .width(Length::FillPortion(1))
        .into(),
        Column::new_with_children(
            test_key_descs
                .iter()
                .map(|(_, desc)| {
                    Text::new(desc)
                        .horizontal_alignment(Alignment::End)
                        .width(Length::Fill)
                        .font(
                            Font::new_with_family(Family::Name(
                                "JetBrainsMono Nerd Font".to_string(),
                            ))
                            .weight(Weight::Semibold),
                        )
                        .into()
                })
                .chain([Row::new_with_children([
                    Text::new("first")
                        .horizontal_alignment(Alignment::End)
                        .into(),
                    Container::new(Text::new("alacritty").horizontal_alignment(Alignment::End))
                        .background_color(Color {
                            red: 0.5,
                            green: 0.0,
                            blue: 0.0,
                            alpha: 1.0,
                        })
                        .width(Length::Shrink)
                        .horizontal_alignment(Alignment::End)
                        .into(),
                ])
                .into()]),
        )
        .width(Length::FillPortion(1))
        .item_alignment(Alignment::End)
        .into(),
    ]))
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(Padding {
        top: 12.0,
        right: 12.0,
        bottom: 12.0,
        left: 12.0,
    })
    .border_radius(64.0)
    .border_thickness(6.0);

    layer
        .new_widget(
            widget,
            400,
            500,
            None,
            KeyboardInteractivity::Exclusive,
            ExclusiveZone::Respect,
            ZLayer::Top,
        )
        .unwrap()
        .on_key_press(|handle, _key, _mods| {
            dbg!(_key);
            if _key == xkbcommon::xkb::Keysym::Escape {
                println!("closing");
                handle.close();
            }
        });

    snowcap_api::listen().await;

    // let widget = layer.new_widget(...);
    //
    // widget.close();

    // layer.new_widget(...)
    //     .on_key_press(|widget, key, mods| {
    //         if key == Key::Escape {
    //             widget.close();
    //         }
    //     })
    //
    // OR
    //
    // let widget = layer.new_widget(...);
    //
    // widget.on_key_press(|key, mods| {
    //     if key == Key::Escape {
    //         widget.close();
    //     }
    // })
    //
}
