use pinnacle_api::layout::{
    CornerLayout, DwindleLayout, FairLayout, LayoutManager, MasterStackLayout, SpiralLayout,
};
use pinnacle_api::signal::WindowSignal;
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
        layout,
    } = modules;

    let mod_key = Mod::Ctrl;

    let terminal = "alacritty";

    // Mousebinds

    // `mod_key + left click` starts moving a window
    input.mousebind([mod_key], MouseButton::Left, MouseEdge::Press, || {
        window.begin_move(MouseButton::Left);
    });

    // `mod_key + right click` starts resizing a window
    input.mousebind([mod_key], MouseButton::Right, MouseEdge::Press, || {
        window.begin_resize(MouseButton::Right);
    });

    // Keybinds

    // `mod_key + alt + q` quits Pinnacle
    input.keybind([mod_key, Mod::Alt], 'q', || {
        pinnacle.quit();
    });

    // `mod_key + alt + c` closes the focused window
    input.keybind([mod_key, Mod::Alt], 'c', || {
        if let Some(window) = window.get_focused() {
            window.close();
        }
    });

    // `mod_key + Return` spawns a terminal
    input.keybind([mod_key], Keysym::Return, move || {
        process.spawn([terminal]);
    });

    // `mod_key + alt + space` toggles floating
    input.keybind([mod_key, Mod::Alt], Keysym::space, || {
        if let Some(window) = window.get_focused() {
            window.toggle_floating();
        }
    });

    // `mod_key + f` toggles fullscreen
    input.keybind([mod_key], 'f', || {
        if let Some(window) = window.get_focused() {
            window.toggle_fullscreen();
        }
    });

    // `mod_key + m` toggles maximized
    input.keybind([mod_key], 'm', || {
        if let Some(window) = window.get_focused() {
            window.toggle_maximized();
        }
    });

    // Window rules
    //
    // You can define window rules to get windows to open with desired properties.
    // See `pinnacle_api::window::rules` in the docs for more information.

    // Layouts

    let master_stack = Box::<MasterStackLayout>::default();
    let dwindle = Box::<DwindleLayout>::default();
    let spiral = Box::<SpiralLayout>::default();
    let corner = Box::<CornerLayout>::default();
    let fair = Box::<FairLayout>::default();

    let layout_requester = layout.set_manager(layout.new_cycling_manager([
        master_stack as _,
        dwindle as _,
        spiral as _,
        corner as _,
        fair as _,
    ]));

    let mut layout_requester_clone = layout_requester.clone();

    // `mod_key + space` cycles to the next layout
    input.keybind([mod_key], Keysym::space, move || {
        let Some(focused_op) = output.get_focused() else { return };
        let Some(first_active_tag) = focused_op
            .tags()
            .into_iter()
            .find(|tg| tg.active().unwrap_or(false))
        else {
            return;
        };

        layout_requester.cycle_layout_forward(&first_active_tag);
        layout_requester.request_layout_on_output(&focused_op);
    });

    // `mod_key + shift + space` cycles to the previous layout
    input.keybind([mod_key, Mod::Shift], Keysym::space, move || {
        let Some(focused_op) = output.get_focused() else { return };
        let Some(first_active_tag) = focused_op
            .tags()
            .into_iter()
            .find(|tg| tg.active().unwrap_or(false))
        else {
            return;
        };

        layout_requester_clone.cycle_layout_backward(&first_active_tag);
        layout_requester_clone.request_layout_on_output(&focused_op);
    });

    // Tags

    let tag_names = ["1", "2", "3", "4", "5"];

    // Setup all monitors with tags "1" through "5"
    output.connect_for_all(move |op| {
        let tags = tag.add(op, tag_names);

        // Be sure to set a tag to active or windows won't display
        tags.first().unwrap().set_active(true);
    });

    process.spawn_once([terminal]);

    for tag_name in tag_names {
        // `mod_key + 1-5` switches to tag "1" to "5"
        input.keybind([mod_key], tag_name, move || {
            if let Some(tg) = tag.get(tag_name) {
                tg.switch_to();
            }
        });

        // `mod_key + shift + 1-5` toggles tag "1" to "5"
        input.keybind([mod_key, Mod::Shift], tag_name, move || {
            if let Some(tg) = tag.get(tag_name) {
                tg.toggle_active();
            }
        });

        // `mod_key + alt + 1-5` moves the focused window to tag "1" to "5"
        input.keybind([mod_key, Mod::Alt], tag_name, move || {
            if let Some(tg) = tag.get(tag_name) {
                if let Some(win) = window.get_focused() {
                    win.move_to_tag(&tg);
                }
            }
        });

        // `mod_key + shift + alt + 1-5` toggles tag "1" to "5" on the focused window
        input.keybind([mod_key, Mod::Shift, Mod::Alt], tag_name, move || {
            if let Some(tg) = tag.get(tag_name) {
                if let Some(win) = window.get_focused() {
                    win.toggle_tag(&tg);
                }
            }
        });
    }

    // Enable sloppy focus
    window.connect_signal(WindowSignal::PointerEnter(Box::new(|win| {
        win.set_focused(true);
    })));
}
