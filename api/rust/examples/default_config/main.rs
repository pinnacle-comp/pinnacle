use std::sync::Arc;
use std::sync::Mutex;

use pinnacle_api::input;
use pinnacle_api::input::Bind;
use pinnacle_api::input::BindLayer;
use pinnacle_api::input::Keysym;
use pinnacle_api::input::{Mod, MouseButton};
use pinnacle_api::layout;
use pinnacle_api::layout::generators::Corner;
use pinnacle_api::layout::generators::CornerLocation;
use pinnacle_api::layout::generators::Cycle;
use pinnacle_api::layout::generators::Dwindle;
use pinnacle_api::layout::generators::Fair;
use pinnacle_api::layout::generators::MasterSide;
use pinnacle_api::layout::generators::MasterStack;
use pinnacle_api::layout::generators::Spiral;
use pinnacle_api::layout::LayoutGenerator;
use pinnacle_api::layout::LayoutNode;
use pinnacle_api::output;
use pinnacle_api::pinnacle;
use pinnacle_api::pinnacle::Backend;
use pinnacle_api::process::Command;
use pinnacle_api::signal::WindowSignal;
use pinnacle_api::tag;
use pinnacle_api::util::{Axis, Batch};
use pinnacle_api::window;

async fn config() {
    // Change the mod key to `Alt` when running as a nested window.
    let mod_key = match pinnacle::backend() {
        Backend::Tty => Mod::SUPER,
        Backend::Window => Mod::ALT,
    };

    let terminal = "alacritty";

    //------------------------
    // Mousebinds            |
    //------------------------

    // `mod_key + left click` starts moving a window
    input::mousebind(mod_key, MouseButton::Left)
        .on_press(|| {
            window::begin_move(MouseButton::Left);
        })
        .group("Mouse")
        .description("Start an interactive window move");

    // `mod_key + right click` starts resizing a window
    input::mousebind(mod_key, MouseButton::Right)
        .on_press(|| {
            window::begin_resize(MouseButton::Right);
        })
        .group("Mouse")
        .description("Start an interactive window resize");

    input::mousebind(mod_key | Mod::SHIFT, MouseButton::Right)
        .on_press(|| {
            pinnacle::reload_config();
            println!("right click press")
        })
        .on_release(|| println!("right click release"));

    //------------------------
    // Keybinds              |
    //------------------------

    // `mod_key + s` shows the bindings overlay
    #[cfg(feature = "snowcap")]
    input::keybind(mod_key, 's')
        .on_press(|| {
            pinnacle_api::snowcap::BindOverlay::new().show();
        })
        .group("Compositor")
        .description("Show the bindings overlay");

    let another_layer = BindLayer::get("another");
    another_layer
        .keybind(mod_key, 'f')
        .on_press(|| println!("pressed f in another"))
        .on_press(|| println!("pressed f in another again"))
        .on_release(|| println!("released f in another"));
    another_layer
        .keybind(Mod::empty(), Keysym::Escape)
        .on_press(|| BindLayer::DEFAULT.enter());

    input::keybind(mod_key, 'a').on_press(move || {
        println!("entering another");
        another_layer.enter()
    });

    input::keybind(mod_key, 'o').on_press(move || {
        println!("mod o");
    });

    #[cfg(not(feature = "snowcap"))]
    input::keybind(mod_key | Mod::SHIFT, 'q')
        .set_as_quit()
        .group("Compositor")
        .description("Quit Pinnacle");

    #[cfg(feature = "snowcap")]
    {
        // `mod_key + shift + q` shows the quit prompt
        input::keybind(mod_key | Mod::SHIFT, 'q')
            .on_press(|| {
                pinnacle_api::snowcap::QuitPrompt::new().show();
            })
            .group("Compositor")
            .description("Show quit prompt");

        // `mod_key + ctrl + shift + q` for the hard shutdown
        input::keybind(mod_key | Mod::CTRL | Mod::SHIFT, 'q')
            .set_as_quit()
            .group("Compositor")
            .description("Quit Pinnacle without prompt");
    }

    // `mod_key + ctrl + r` reloads the config
    input::keybind(mod_key | Mod::CTRL, 'r')
        .set_as_reload_config()
        .group("Compositor")
        .description("Reload the config");

    // `mod_key + shift + c` closes the focused window
    input::keybind(mod_key | Mod::SHIFT, 'c')
        .on_press(|| {
            if let Some(window) = window::get_focused() {
                window.close();
            }
        })
        .group("Window")
        .description("Close the focused window");

    // `mod_key + Return` spawns a terminal
    input::keybind(mod_key, Keysym::Return)
        .on_press(move || {
            Command::new(terminal).spawn();
        })
        .group("Process")
        .description("Spawn a terminal");

    // `mod_key + ctrl + space` toggles floating
    input::keybind(mod_key | Mod::CTRL, Keysym::space)
        .on_press(|| {
            if let Some(window) = window::get_focused() {
                window.toggle_floating();
                window.raise();
            }
        })
        .group("Window")
        .description("Toggle floating on the focused window");

    // `mod_key + f` toggles fullscreen
    input::keybind(mod_key, 'f')
        .on_press(|| {
            if let Some(window) = window::get_focused() {
                window.toggle_fullscreen();
                window.raise();
            }
        })
        .group("Window")
        .description("Toggle fullscreen on the focused window");

    // `mod_key + m` toggles maximized
    input::keybind(mod_key, 'm')
        .on_press(|| {
            if let Some(window) = window::get_focused() {
                window.toggle_maximized();
                window.raise();
            }
        })
        .group("Window")
        .description("Toggle maximized on the focused window");

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

    fn into_box<'a, T: LayoutGenerator + Send + 'a>(
        generator: T,
    ) -> Box<dyn LayoutGenerator + Send + 'a> {
        Box::new(generator) as _
    }

    let cycler = Arc::new(Mutex::new(Cycle::new([
        into_box(MasterStack::default()),
        into_box(MasterStack {
            master_side: MasterSide::Right,
            ..Default::default()
        }),
        into_box(MasterStack {
            master_side: MasterSide::Top,
            ..Default::default()
        }),
        into_box(MasterStack {
            master_side: MasterSide::Bottom,
            ..Default::default()
        }),
        into_box(Dwindle::default()),
        into_box(Spiral::default()),
        into_box(Corner::default()),
        into_box(Corner {
            corner_loc: CornerLocation::TopRight,
            ..Default::default()
        }),
        into_box(Corner {
            corner_loc: CornerLocation::BottomLeft,
            ..Default::default()
        }),
        into_box(Corner {
            corner_loc: CornerLocation::BottomRight,
            ..Default::default()
        }),
        into_box(Fair::default()),
        into_box(Fair {
            axis: Axis::Horizontal,
            ..Default::default()
        }),
    ])));

    let layout_requester = layout::manage({
        let cycler = cycler.clone();
        move |args| {
            let Some(tag) = args.tags.first() else {
                return LayoutNode::new();
            };
            let mut generator = cycler.lock().unwrap();
            generator.set_current_tag(tag.clone());
            generator.layout(args.window_count)
        }
    });

    // `mod_key + space` cycles to the next layout
    input::keybind(mod_key, Keysym::space)
        .on_press({
            let cycler = cycler.clone();
            let requester = layout_requester.clone();
            move || {
                let Some(focused_op) = output::get_focused() else {
                    return;
                };
                let Some(first_active_tag) = focused_op
                    .tags()
                    .batch_find(|tag| Box::pin(tag.active_async()), |active| *active)
                else {
                    return;
                };

                cycler
                    .lock()
                    .unwrap()
                    .cycle_layout_forward(&first_active_tag);
                requester.request_layout_on_output(&focused_op);
            }
        })
        .group("Layout")
        .description("Cycle the layout forward");

    // `mod_key + shift + space` cycles to the previous layout
    input::keybind(mod_key | Mod::SHIFT, Keysym::space)
        .on_press(move || {
            let Some(focused_op) = output::get_focused() else {
                return;
            };
            let Some(first_active_tag) = focused_op
                .tags()
                .batch_find(|tag| Box::pin(tag.active_async()), |active| *active)
            else {
                return;
            };

            cycler
                .lock()
                .unwrap()
                .cycle_layout_backward(&first_active_tag);
            layout_requester.request_layout_on_output(&focused_op);
        })
        .group("Layout")
        .description("Cycle the layout backward");

    //------------------------
    // Tags                  |
    //------------------------

    let tag_names = ["1", "2", "3", "4", "5"];

    // Setup all monitors with tags "1" through "5"
    output::for_each_output(move |output| {
        let mut tags = tag::add(output, tag_names);
        tags.next().unwrap().set_active(true);
    });

    for tag_name in tag_names {
        // `mod_key + 1-5` switches to tag "1" to "5"
        input::keybind(mod_key, tag_name)
            .on_press(move || {
                if let Some(tag) = tag::get(tag_name) {
                    tag.switch_to();
                }
            })
            .group("Tag")
            .description(format!("Switch to tag {tag_name}"));

        // `mod_key + ctrl + 1-5` toggles tag "1" to "5"
        input::keybind(mod_key | Mod::CTRL, tag_name)
            .on_press(move || {
                if let Some(tag) = tag::get(tag_name) {
                    tag.toggle_active();
                }
            })
            .group("Tag")
            .description(format!("Toggle tag {tag_name}"));

        // `mod_key + shift + 1-5` moves the focused window to tag "1" to "5"
        input::keybind(mod_key | Mod::SHIFT, tag_name)
            .on_press(move || {
                if let Some(tag) = tag::get(tag_name) {
                    if let Some(win) = window::get_focused() {
                        win.move_to_tag(&tag);
                    }
                }
            })
            .group("Tag")
            .description(format!("Move the focused window to tag {tag_name}"));

        // `mod_key + ctrl + shift + 1-5` toggles tag "1" to "5" on the focused window
        input::keybind(mod_key | Mod::CTRL | Mod::SHIFT, tag_name)
            .on_press(move || {
                if let Some(tg) = tag::get(tag_name) {
                    if let Some(win) = window::get_focused() {
                        win.toggle_tag(&tg);
                    }
                }
            })
            .group("Tag")
            .description(format!("Toggle tag {tag_name} on the focused window"));
    }

    input::libinput::for_all_devices(|device| {
        if device.device_type().is_touchpad() {
            device.set_natural_scroll(true);
        }
    });

    // Request all windows use client-side decorations.
    window::add_window_rule(|window| {
        window.set_decoration_mode(window::DecorationMode::ClientSide);
    });

    // Enable sloppy focus
    window::connect_signal(WindowSignal::PointerEnter(Box::new(|win| {
        win.set_focused(true);
    })));

    Command::new(terminal).once().spawn();
}

pinnacle_api::main!(config);
