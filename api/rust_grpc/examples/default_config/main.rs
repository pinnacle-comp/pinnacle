use pinnacle_api::xkbcommon::xkb::Keysym;
use pinnacle_api::{
    input::{Mod, MouseButton, MouseEdge},
    tag::{Layout, LayoutCycler},
    ApiModules,
};

#[pinnacle_api::config(modules)]
async fn main() {
    let ApiModules {
        pinnacle,
        process,
        window,
        input,
        output,
        tag,
    } = modules;

    let mod_key = Mod::Ctrl;

    input.mousebind([mod_key], MouseButton::Left, MouseEdge::Press, || {
        window.begin_move(MouseButton::Left);
    });

    input.mousebind([mod_key], MouseButton::Right, MouseEdge::Press, || {
        window.begin_resize(MouseButton::Right);
    });

    // Keybinds

    input.keybind([mod_key, Mod::Alt], 'q', || {
        pinnacle.quit();
    });

    input.keybind([mod_key, Mod::Alt], 'c', || {
        if let Some(window) = window.get_focused() {
            window.close();
        }
    });

    input.keybind([mod_key], Keysym::Return, || {
        process.spawn(["alacritty"]);
    });

    input.keybind([mod_key, Mod::Alt], Keysym::space, || {
        if let Some(window) = window.get_focused() {
            window.toggle_floating();
        }
    });

    input.keybind([mod_key], 'f', || {
        if let Some(window) = window.get_focused() {
            window.toggle_fullscreen();
        }
    });

    input.keybind([mod_key], 'm', || {
        if let Some(window) = window.get_focused() {
            window.toggle_maximized();
        }
    });

    // Tags

    let tag_names = ["1", "2", "3", "4", "5"];

    output.connect_for_all(move |op| {
        let mut tags = tag.add(&op, tag_names);
        tags.next().unwrap().set_active(true);
    });

    process.spawn_once(["alacritty"]);

    let LayoutCycler {
        prev: layout_prev,
        next: layout_next,
    } = tag.new_layout_cycler([
        Layout::MasterStack,
        Layout::Dwindle,
        Layout::Spiral,
        Layout::CornerTopLeft,
        Layout::CornerTopRight,
        Layout::CornerBottomLeft,
        Layout::CornerBottomRight,
    ]);

    input.keybind([mod_key], Keysym::space, move || {
        layout_next(None);
    });

    input.keybind([mod_key, Mod::Shift], Keysym::space, move || {
        layout_prev(None);
    });

    for tag_name in tag_names {
        input.keybind([mod_key], tag_name, move || {
            if let Some(tg) = tag.get(tag_name, None) {
                tg.switch_to();
            }
        });

        input.keybind([mod_key, Mod::Shift], tag_name, move || {
            if let Some(tg) = tag.get(tag_name, None) {
                tg.toggle_active();
            }
        });

        input.keybind([mod_key, Mod::Alt], tag_name, move || {
            if let Some(tg) = tag.get(tag_name, None) {
                if let Some(win) = window.get_focused() {
                    win.move_to_tag(&tg);
                }
            }
        });

        input.keybind([mod_key, Mod::Shift, Mod::Alt], tag_name, move || {
            if let Some(tg) = tag.get(tag_name, None) {
                if let Some(win) = window.get_focused() {
                    win.toggle_tag(&tg);
                }
            }
        });
    }
}
