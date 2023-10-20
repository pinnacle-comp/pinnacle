use pinnacle_api::prelude::*;
use pinnacle_api::*;

fn main() {
    pinnacle::setup(|| {
        let mod_key = Modifier::Ctrl;

        let terminal = "alacritty";

        process::set_env("MOZ_ENABLE_WAYLAND", "1");

        input::mousebind(&[mod_key], MouseButton::Left, MouseEdge::Press, move || {
            window::begin_move(MouseButton::Left);
        });

        input::mousebind(
            &[mod_key],
            MouseButton::Right,
            MouseEdge::Press,
            move || {
                window::begin_resize(MouseButton::Right);
            },
        );

        input::keybind(&[mod_key, Modifier::Alt], 'q', pinnacle::quit);

        input::keybind(&[mod_key, Modifier::Alt], 'c', move || {
            if let Some(window) = window::get_focused() {
                window.close();
            }
        });

        input::keybind(&[mod_key], xkbcommon::xkb::keysyms::KEY_Return, move || {
            process::spawn(vec![terminal]).unwrap();
        });

        input::keybind(
            &[mod_key, Modifier::Alt],
            xkbcommon::xkb::keysyms::KEY_space,
            move || {
                if let Some(window) = window::get_focused() {
                    window.toggle_floating();
                }
            },
        );

        input::keybind(&[mod_key], 'f', move || {
            if let Some(window) = window::get_focused() {
                window.toggle_fullscreen();
            }
        });

        input::keybind(&[mod_key], 'm', move || {
            if let Some(window) = window::get_focused() {
                window.toggle_maximized();
            }
        });

        let tags = ["1", "2", "3", "4", "5"];

        output::connect_for_all(move |output| {
            tag::add(&output, tags.as_slice());
            tag::get("1", Some(&output)).unwrap().toggle();
        });

        // let layout_cycler = tag.layout_cycler(&[
        //     Layout::MasterStack,
        //     Layout::Dwindle,
        //     Layout::Spiral,
        //     Layout::CornerTopLeft,
        //     Layout::CornerTopRight,
        //     Layout::CornerBottomLeft,
        //     Layout::CornerBottomRight,
        // ]);
        //
        // input.keybind(&[mod_key], xkbcommon::xkb::keysyms::KEY_space, move || {
        //     layout_cycler.next(None);
        // });

        for tag_name in tags.iter().map(|t| t.to_string()) {
            let t = tag_name.clone();
            input::keybind(&[mod_key], tag_name.chars().next().unwrap(), move || {
                tag::get(&t, None).unwrap().switch_to();
            });
            let t = tag_name.clone();
            input::keybind(
                &[mod_key, Modifier::Shift],
                tag_name.chars().next().unwrap(),
                move || {
                    tag::get(&t, None).unwrap().toggle();
                },
            );
            let t = tag_name.clone();
            input::keybind(
                &[mod_key, Modifier::Alt],
                tag_name.chars().next().unwrap(),
                move || {
                    if let Some(window) = window::get_focused() {
                        window.move_to_tag(&tag::get(&t, None).unwrap());
                    }
                },
            );
            let t = tag_name.clone();
            input::keybind(
                &[mod_key, Modifier::Shift, Modifier::Alt],
                tag_name.chars().next().unwrap(),
                move || {
                    if let Some(window) = window::get_focused() {
                        window.toggle_tag(&tag::get(&t, None).unwrap());
                    }
                },
            );
        }
    })
    .unwrap();
}
