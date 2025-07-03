use pinnacle_api::input::Bind as _;

use crate::{
    common::{Lang, fixture::Fixture, for_each_api},
    spawn_lua_blocking,
};

fn set_up() -> Fixture {
    let fixture = Fixture::new();
    fixture
        .runtime_handle()
        .block_on(pinnacle_api::connect())
        .unwrap();
    fixture
}

#[test_log::test]
fn input_set_xkb_config() {
    for_each_api(|lang| {
        let mut fixture = set_up();

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::input::set_xkb_config(
                    pinnacle_api::input::XkbConfig::new().with_layout("us,fr,ge"),
                );
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Input.set_xkb_config({
                    layout = "us,fr,ge",
                })
            },
        }

        let kb = fixture.pinnacle().seat.get_keyboard().unwrap();
        let layouts = kb.with_xkb_state(fixture.state(), |ctx| {
            let xkb = ctx.xkb().lock().unwrap();
            xkb.layouts()
                .map(|layout| xkb.layout_name(layout).to_string())
                .collect::<Vec<_>>()
        });

        assert_eq!(
            layouts,
            [
                "English (US)".to_string(),
                "French".to_string(),
                "Georgian".to_string()
            ]
        )
    });
}

#[test_log::test]
fn input_switch_xkb_layout() {
    for_each_api(|lang| {
        let mut fixture = set_up();

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::input::set_xkb_config(
                    pinnacle_api::input::XkbConfig::new().with_layout("us,fr,ge"),
                );
                pinnacle_api::input::cycle_xkb_layout_backward();
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Input.set_xkb_config({
                    layout = "us,fr,ge",
                })
                Input.cycle_xkb_layout_backward()
            },
        }

        let kb = fixture.pinnacle().seat.get_keyboard().unwrap();
        let layout_idx = kb.with_xkb_state(fixture.state(), |ctx| {
            let xkb = ctx.xkb().lock().unwrap();
            xkb.active_layout().0
        });
        assert_eq!(layout_idx, 2);

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::input::cycle_xkb_layout_forward();
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Input.cycle_xkb_layout_forward()
            },
        }

        let layout_idx = kb.with_xkb_state(fixture.state(), |ctx| {
            let xkb = ctx.xkb().lock().unwrap();
            xkb.active_layout().0
        });
        assert_eq!(layout_idx, 0);

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::input::switch_xkb_layout(1);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Input.switch_xkb_layout(1)
            },
        }

        let layout_idx = kb.with_xkb_state(fixture.state(), |ctx| {
            let xkb = ctx.xkb().lock().unwrap();
            xkb.active_layout().0
        });
        assert_eq!(layout_idx, 1)
    });
}

#[test_log::test]
fn input_keybind() {
    for_each_api(|lang| {
        let mut fixture = set_up();

        // Need tokio here for the input stuff
        let handle = fixture.runtime_handle();
        let _guard = handle.enter();

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::input::BindLayer::get("morb_layer")
                    .keybind(
                        pinnacle_api::input::Mod::SUPER | pinnacle_api::input::Mod::SHIFT,
                        'c',
                    )
                    .group("Left")
                    .description("Right")
                    .allow_when_locked()
                    .on_press(|| {});
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Input.keybind({
                    mods = { "super", "shift" },
                    key = "c",
                    bind_layer = "morb_layer",
                    group = "Left",
                    description = "Right",
                    allow_when_locked = true,
                    on_press = function() end,
                })
            },
        }

        let keybind = fixture
            .pinnacle()
            .input_state
            .bind_state
            .keybinds
            .id_map
            .iter()
            .next()
            .unwrap()
            .1
            .clone();
        let keybind = keybind.borrow();

        assert_eq!(keybind.key, pinnacle_api::Keysym::c);
        assert_eq!(keybind.bind_data.layer.as_deref(), Some("morb_layer"));
        assert_eq!(keybind.bind_data.group, "Left");
        assert_eq!(keybind.bind_data.desc, "Right");
        assert!(!keybind.bind_data.is_quit_bind);
        assert!(!keybind.bind_data.is_reload_config_bind);
        assert!(keybind.bind_data.allow_when_locked);
        assert!(keybind.has_on_press)
    });
}

#[test_log::test]
fn input_mousebind() {
    for_each_api(|lang| {
        let mut fixture = set_up();

        // Need tokio here for the input stuff
        let handle = fixture.runtime_handle();
        let _guard = handle.enter();

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::input::BindLayer::get("morb_layer")
                    .mousebind(
                        pinnacle_api::input::Mod::SUPER | pinnacle_api::input::Mod::SHIFT,
                        pinnacle_api::input::MouseButton::Right,
                    )
                    .group("Left")
                    .description("Right")
                    .allow_when_locked()
                    .on_press(|| {});
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Input.mousebind({
                    mods = { "super", "shift" },
                    button = "btn_right",
                    bind_layer = "morb_layer",
                    group = "Left",
                    description = "Right",
                    allow_when_locked = true,
                    on_press = function() end,
                })
            },
        }

        let mousebind = fixture
            .pinnacle()
            .input_state
            .bind_state
            .mousebinds
            .id_map
            .iter()
            .next()
            .unwrap()
            .1
            .clone();
        let mousebind = mousebind.borrow();

        assert_eq!(
            mousebind.button,
            u32::from(pinnacle_api::input::MouseButton::Right)
        );
        assert_eq!(mousebind.bind_data.layer.as_deref(), Some("morb_layer"));
        assert_eq!(mousebind.bind_data.group, "Left");
        assert_eq!(mousebind.bind_data.desc, "Right");
        assert!(!mousebind.bind_data.is_quit_bind);
        assert!(!mousebind.bind_data.is_reload_config_bind);
        assert!(mousebind.bind_data.allow_when_locked);
        assert!(mousebind.has_on_press)
    });
}
