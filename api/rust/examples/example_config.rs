use pinnacle_api::prelude::*;
use pinnacle_api::*;

fn main() {
    pinnacle_api::connect().unwrap();

    let mod_key = Modifier::Ctrl;

    let terminal = "alacritty";

    process::set_env("MOZ_ENABLE_WAYLAND", "1");

    let mut callback_vec = CallbackVec::new();

    input::mousebind(
        &[mod_key],
        MouseButton::Left,
        MouseEdge::Press,
        move |_| {
            window::begin_move(MouseButton::Left);
        },
        &mut callback_vec,
    );

    input::mousebind(
        &[mod_key],
        MouseButton::Right,
        MouseEdge::Press,
        move |_| {
            window::begin_resize(MouseButton::Right);
        },
        &mut callback_vec,
    );

    input::keybind(
        &[mod_key, Modifier::Alt],
        'q',
        |_| pinnacle::quit(),
        &mut callback_vec,
    );

    input::keybind(
        &[mod_key, Modifier::Alt],
        'c',
        move |_| {
            if let Some(window) = window::get_focused() {
                window.close();
            }
        },
        &mut callback_vec,
    );

    input::keybind(
        &[mod_key],
        xkbcommon::xkb::keysyms::KEY_Return,
        move |_| {
            process::spawn(vec![terminal]).unwrap();
        },
        &mut callback_vec,
    );

    input::keybind(
        &[mod_key, Modifier::Alt],
        xkbcommon::xkb::keysyms::KEY_space,
        move |_| {
            if let Some(window) = window::get_focused() {
                window.toggle_floating();
            }
        },
        &mut callback_vec,
    );

    input::keybind(
        &[mod_key],
        'f',
        move |_| {
            if let Some(window) = window::get_focused() {
                window.toggle_fullscreen();
            }
        },
        &mut callback_vec,
    );

    input::keybind(
        &[mod_key],
        'm',
        move |_| {
            if let Some(window) = window::get_focused() {
                window.toggle_maximized();
            }
        },
        &mut callback_vec,
    );

    let tags = ["1", "2", "3", "4", "5"];

    output::connect_for_all(
        move |output, _| {
            tag::add(&output, tags.as_slice());
            tag::get("1", Some(&output)).unwrap().toggle();
        },
        &mut callback_vec,
    );

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
        input::keybind(
            &[mod_key],
            tag_name.chars().next().unwrap(),
            move |_| {
                tag::get(&t, None).unwrap().switch_to();
            },
            &mut callback_vec,
        );
        let t = tag_name.clone();
        input::keybind(
            &[mod_key, Modifier::Shift],
            tag_name.chars().next().unwrap(),
            move |_| {
                tag::get(&t, None).unwrap().toggle();
            },
            &mut callback_vec,
        );
        let t = tag_name.clone();
        input::keybind(
            &[mod_key, Modifier::Alt],
            tag_name.chars().next().unwrap(),
            move |_| {
                if let Some(window) = window::get_focused() {
                    window.move_to_tag(&tag::get(&t, None).unwrap());
                }
            },
            &mut callback_vec,
        );
        let t = tag_name.clone();
        input::keybind(
            &[mod_key, Modifier::Shift, Modifier::Alt],
            tag_name.chars().next().unwrap(),
            move |_| {
                if let Some(window) = window::get_focused() {
                    window.toggle_tag(&tag::get(&t, None).unwrap());
                }
            },
            &mut callback_vec,
        );
    }

    pinnacle_api::listen(callback_vec);
}