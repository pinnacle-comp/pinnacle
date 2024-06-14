use pinnacle_api::input::libinput::LibinputSetting;
use pinnacle_api::layout::{
    CornerLayout, CornerLocation, CyclingLayoutManager, DwindleLayout, FairLayout, MasterSide,
    MasterStackLayout, SpiralLayout,
};
use pinnacle_api::output::OutputSetup;
use pinnacle_api::signal::WindowSignal;
use pinnacle_api::util::{Axis, Batch};
use pinnacle_api::xkbcommon::xkb::Keysym;
use pinnacle_api::{
    input::{Mod, MouseButton, MouseEdge},
    ApiModules,
};
#[cfg(feature = "snowcap")]
use snowcap_api::layer::{ExclusiveZone, KeyboardInteractivity, ZLayer};
#[cfg(feature = "snowcap")]
use snowcap_api::widget::{Alignment, Column, Container, Length, Text};

// Pinnacle needs to perform some setup before and after your config.
// The `#[pinnacle_api::config(modules)]` attribute does so and
// will bind all the config structs to the provided identifier.
#[pinnacle_api::config(modules)]
async fn main() {
    // Deconstruct to get all the APIs.
    #[cfg(feature = "snowcap")]
    #[allow(unused_variables)]
    let ApiModules {
        pinnacle,
        process,
        window,
        input,
        output,
        tag,
        layout,
        render,
        snowcap,
        ..
    } = modules;

    #[cfg(not(feature = "snowcap"))]
    #[allow(unused_variables)]
    let ApiModules {
        pinnacle,
        process,
        window,
        input,
        output,
        tag,
        layout,
        render,
        ..
    } = modules;

    let mod_key = Mod::Ctrl;

    let terminal = "alacritty";

    //------------------------
    // Mousebinds            |
    //------------------------

    // `mod_key + left click` starts moving a window
    input.mousebind([mod_key], MouseButton::Left, MouseEdge::Press, || {
        window.begin_move(MouseButton::Left);
    });

    // `mod_key + right click` starts resizing a window
    input.mousebind([mod_key], MouseButton::Right, MouseEdge::Press, || {
        window.begin_resize(MouseButton::Right);
    });

    //------------------------
    // Keybinds              |
    //------------------------

    // `mod_key + alt + q` quits Pinnacle
    input.keybind([mod_key, Mod::Alt], 'q', || {
        #[cfg(feature = "snowcap")]
        {
            let widget = Container::new(Column::new_with_children([
                Text::new("Quit Pinnacle?").into(),
                Text::new("Press ENTER to confirm.").into(),
            ]))
            .with_vertical_alignment(Alignment::Center)
            .with_horizontal_alignment(Alignment::Center)
            .with_width(Length::Fill)
            .with_height(Length::Fill)
            .with_border_thickness(4.0)
            .with_border_radius(16.0);

            snowcap
                .new_widget(
                    widget.into(),
                    200,
                    100,
                    None,
                    KeyboardInteractivity::Exclusive,
                    ExclusiveZone::Respect,
                    ZLayer::Overlay,
                )
                .on_key_press(|widget, key, _mods| {
                    if key == Keysym::Return {
                        pinnacle.quit();
                    } else {
                        widget.close();
                    }
                });
        }

        #[cfg(not(feature = "snowcap"))]
        pinnacle.quit();
    });

    // `mod_key + alt + r` reloads the config
    input.keybind([mod_key, Mod::Alt], 'r', || {
        pinnacle.reload_config();
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
            window.raise();
        }
    });

    // `mod_key + f` toggles fullscreen
    input.keybind([mod_key], 'f', || {
        if let Some(window) = window.get_focused() {
            window.toggle_fullscreen();
            window.raise();
        }
    });

    // `mod_key + m` toggles maximized
    input.keybind([mod_key], 'm', || {
        if let Some(window) = window.get_focused() {
            window.toggle_maximized();
            window.raise();
        }
    });

    //------------------------
    // Window rules          |
    //------------------------
    // You can define window rules to get windows to open with desired properties.
    // See `pinnacle_api::window::rules` in the docs for more information.

    //------------------------
    // Layouts               |
    //------------------------

    // Pinnacle does not manage layouts compositor-side.
    // Instead, it delegates computation of layouts to your config,
    // which provides an interface to calculate the size and location of
    // windows that the compositor will use to position windows.
    //
    // If you're familiar with River's layout generators, you'll understand the system here
    // a bit better.
    //
    // The Rust API provides two layout system abstractions:
    //     1. Layout managers, and
    //     2. Layout generators.
    //
    // ### Layout Managers ###
    // A layout manager is a struct that implements the `LayoutManager` trait.
    // A manager is meant to keep track of and choose various layout generators
    // across your usage of the compositor.
    //
    // ### Layout generators ###
    // A layout generator is a struct that implements the `LayoutGenerator` trait.
    // It takes in layout arguments and computes a vector of geometries that will
    // determine the size and position of windows being laid out.
    //
    // There is one built-in layout manager and five built-in layout generators,
    // as shown below.
    //
    // Additionally, this system is designed to be user-extensible;
    // you are free to create your own layout managers and generators for
    // maximum customizability! Docs for doing so are in the works, so sit tight.

    // Create a `CyclingLayoutManager` that can cycle between layouts on different tags.
    //
    // It takes in some layout generators that need to be boxed and dyn-coerced.
    let layout_requester = layout.set_manager(CyclingLayoutManager::new([
        Box::<MasterStackLayout>::default() as _,
        Box::new(MasterStackLayout {
            master_side: MasterSide::Right,
            ..Default::default()
        }) as _,
        Box::new(MasterStackLayout {
            master_side: MasterSide::Top,
            ..Default::default()
        }) as _,
        Box::new(MasterStackLayout {
            master_side: MasterSide::Bottom,
            ..Default::default()
        }) as _,
        Box::<DwindleLayout>::default() as _,
        Box::<SpiralLayout>::default() as _,
        Box::<CornerLayout>::default() as _,
        Box::new(CornerLayout {
            corner_loc: CornerLocation::TopRight,
            ..Default::default()
        }) as _,
        Box::new(CornerLayout {
            corner_loc: CornerLocation::BottomLeft,
            ..Default::default()
        }) as _,
        Box::new(CornerLayout {
            corner_loc: CornerLocation::BottomRight,
            ..Default::default()
        }) as _,
        Box::<FairLayout>::default() as _,
        Box::new(FairLayout {
            axis: Axis::Horizontal,
            ..Default::default()
        }) as _,
    ]));

    let mut layout_requester_clone = layout_requester.clone();

    // `mod_key + space` cycles to the next layout
    input.keybind([mod_key], Keysym::space, move || {
        let Some(focused_op) = output.get_focused() else { return };
        let Some(first_active_tag) = focused_op.tags().batch_find(
            |tg| Box::pin(tg.active_async()),
            |active| active == &Some(true),
        ) else {
            return;
        };

        layout_requester.cycle_layout_forward(&first_active_tag);
        layout_requester.request_layout_on_output(&focused_op);
    });

    // `mod_key + shift + space` cycles to the previous layout
    input.keybind([mod_key, Mod::Shift], Keysym::space, move || {
        let Some(focused_op) = output.get_focused() else { return };
        let Some(first_active_tag) = focused_op.tags().batch_find(
            |tg| Box::pin(tg.active_async()),
            |active| active == &Some(true),
        ) else {
            return;
        };

        layout_requester_clone.cycle_layout_backward(&first_active_tag);
        layout_requester_clone.request_layout_on_output(&focused_op);
    });

    //------------------------
    // Tags                  |
    //------------------------

    let tag_names = ["1", "2", "3", "4", "5"];

    // Setup all monitors with tags "1" through "5"
    output.setup([OutputSetup::new_with_matcher(|_| true).with_tags(tag_names)]);

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

    input.set_libinput_setting(LibinputSetting::Tap(true));

    // Enable sloppy focus
    window.connect_signal(WindowSignal::PointerEnter(Box::new(|win| {
        win.set_focused(true);
    })));

    process.spawn_once([terminal]);
}
