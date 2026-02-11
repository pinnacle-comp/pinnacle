use crate::{
    common::{Lang, fixture::Fixture, for_each_api},
    spawn_lua_blocking,
};
use pinnacle::{focus::keyboard::KeyboardFocusTarget, state::WithState, tag::Tag};
use pinnacle_api::{
    layout::{LayoutGenerator as _, generators::MasterStack},
    output::OutputHandle,
};
use smithay::{
    output::Output,
    reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1,
    utils::Rectangle,
};

fn set_up() -> (Fixture, Output) {
    let mut fixture = Fixture::new();

    let output = fixture.add_output(Rectangle::new((0, 0).into(), (1920, 1080).into()));
    output.with_state_mut(|state| {
        let tag = Tag::new("1".to_string());
        tag.set_active(true);
        state.add_tags([tag]);
    });
    fixture.pinnacle().focus_output(&output);

    fixture
        .runtime_handle()
        .block_on(pinnacle_api::connect())
        .unwrap();

    fixture.spawn_blocking(|| {
        pinnacle_api::layout::manage(|args| pinnacle_api::layout::LayoutResponse {
            root_node: MasterStack::default().layout(args.window_count),
            tree_id: 0,
        });
    });

    (fixture, output)
}

#[test_log::test]
fn window_get_all() {
    let (mut fixture, _) = set_up();

    fixture.spawn_blocking(|| {
        assert_eq!(pinnacle_api::window::get_all().count(), 0);
    });

    spawn_lua_blocking! {
        fixture,
        assert(#Window.get_all() == 0)
    }

    let client_id = fixture.add_client();

    fixture.spawn_windows(5, client_id);

    fixture.spawn_blocking(|| {
        assert_eq!(pinnacle_api::window::get_all().count(), 5);
    });

    spawn_lua_blocking! {
        fixture,
        assert(#Window.get_all() == 5)
    }
}

#[test_log::test]
fn window_get_focused() {
    let (mut fixture, _) = set_up();

    fixture.spawn_blocking(|| {
        assert!(pinnacle_api::window::get_focused().is_none());
    });

    spawn_lua_blocking! {
        fixture,
        assert(not Window.get_focused())
    }

    let client_id = fixture.add_client();

    fixture.spawn_windows(1, client_id);

    fixture.spawn_blocking(|| {
        assert!(pinnacle_api::window::get_focused().is_some());
    });

    spawn_lua_blocking! {
        fixture,
        assert(Window.get_focused())
    }
}

#[test_log::test]
fn window_handle_close() {
    let (mut fixture, _) = set_up();

    let client_id = fixture.add_client();

    for_each_api(|lang| {
        let surface = fixture.spawn_windows(1, client_id).remove(0);

        assert_eq!(fixture.pinnacle().windows.len(), 1);

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused().unwrap().close();
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():close()
            },
        }
        fixture.roundtrip(client_id);

        let window = fixture.client(client_id).window_for_surface(&surface);
        assert!(window.close_requested);
        fixture.client(client_id).close_window(&surface);
        fixture.roundtrip(client_id);

        assert_eq!(fixture.pinnacle().windows.len(), 0);
    });
}

#[test_log::test]
fn window_handle_set_geometry_floating() {
    for_each_api(|lang| {
        let (mut fixture, _) = set_up();

        let client_id = fixture.add_client();

        let _surface = fixture.spawn_floating_window_with(client_id, (500, 500), |_| ());

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_geometry(200, 300, 1000, 1000);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():set_geometry({ x = 200, y = 300, width = 1000, height = 1000 })
            },
        }

        fixture.wait_client_configure(client_id);
        fixture.flush();

        let window = fixture.pinnacle().windows[0].clone();
        let geo = fixture.pinnacle().space.element_geometry(&window).unwrap();

        assert_eq!(geo, Rectangle::new((200, 300).into(), (1000, 1000).into()));
    });
}

#[test_log::test]
fn window_handle_set_geometry_tiled_does_not_change_geometry() {
    for_each_api(|lang| {
        let (mut fixture, _) = set_up();

        let client_id = fixture.add_client();

        let _surface = fixture.spawn_windows(1, client_id).remove(0);

        let window = fixture.pinnacle().windows[0].clone();

        let old_geo = fixture.pinnacle().space.element_geometry(&window).unwrap();

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_geometry(200, 300, 1000, 1000);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():set_geometry({ x = 200, y = 300, width = 1000, height = 1000 })
            },
        }

        fixture.flush();

        let new_geo = fixture.pinnacle().space.element_geometry(&window).unwrap();

        assert_eq!(new_geo, old_geo);
    });
}

#[test_log::test]
fn window_handle_set_fullscreen() {
    for_each_api(|lang| {
        let (mut fixture, _) = set_up();

        let client_id = fixture.add_client();

        let surface = fixture.spawn_windows(1, client_id).remove(0);

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_fullscreen(true);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():set_fullscreen(true)
            },
        }

        fixture.wait_client_configure(client_id);
        fixture.flush();

        assert!(
            fixture
                .client(client_id)
                .window_for_surface(&surface)
                .fullscreen
        );

        let window = fixture.pinnacle().windows[0].clone();
        assert!(window.with_state(|state| state.layout_mode.is_fullscreen()));

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_fullscreen(false);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():set_fullscreen(false)
            },
        }

        fixture.wait_client_configure(client_id);
        fixture.flush();

        assert!(
            !fixture
                .client(client_id)
                .window_for_surface(&surface)
                .fullscreen
        );

        let window = fixture.pinnacle().windows[0].clone();
        assert!(window.with_state(|state| !state.layout_mode.is_fullscreen()));
    });
}

#[test_log::test]
fn window_handle_toggle_fullscreen() {
    for_each_api(|lang| {
        let (mut fixture, _) = set_up();

        let client_id = fixture.add_client();

        let surface = fixture.spawn_windows(1, client_id).remove(0);

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .toggle_fullscreen();
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():toggle_fullscreen()
            },
        }

        fixture.wait_client_configure(client_id);
        fixture.flush();

        assert!(
            fixture
                .client(client_id)
                .window_for_surface(&surface)
                .fullscreen
        );

        let window = fixture.pinnacle().windows[0].clone();
        assert!(window.with_state(|state| state.layout_mode.is_fullscreen()));

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .toggle_fullscreen();
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():toggle_fullscreen()
            },
        }

        fixture.wait_client_configure(client_id);
        fixture.flush();

        assert!(
            !fixture
                .client(client_id)
                .window_for_surface(&surface)
                .fullscreen
        );

        let window = fixture.pinnacle().windows[0].clone();
        assert!(window.with_state(|state| !state.layout_mode.is_fullscreen()));
    });
}

#[test_log::test]
fn window_handle_set_maximized() {
    for_each_api(|lang| {
        let (mut fixture, _) = set_up();

        let client_id = fixture.add_client();

        let surface = fixture.spawn_windows(1, client_id).remove(0);

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_maximized(true);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():set_maximized(true)
            },
        }

        fixture.wait_client_configure(client_id);
        fixture.flush();

        assert!(
            fixture
                .client(client_id)
                .window_for_surface(&surface)
                .maximized
        );

        let window = fixture.pinnacle().windows[0].clone();
        assert!(window.with_state(|state| state.layout_mode.is_maximized()));

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_maximized(false);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():set_maximized(false)
            },
        }

        fixture.wait_client_configure(client_id);
        fixture.flush();

        assert!(
            !fixture
                .client(client_id)
                .window_for_surface(&surface)
                .maximized
        );

        let window = fixture.pinnacle().windows[0].clone();
        assert!(window.with_state(|state| !state.layout_mode.is_maximized()));
    });
}

#[test_log::test]
fn window_handle_toggle_maximized() {
    for_each_api(|lang| {
        let (mut fixture, _) = set_up();

        let client_id = fixture.add_client();

        let surface = fixture.spawn_windows(1, client_id).remove(0);

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .toggle_maximized();
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():toggle_maximized()
            },
        }

        fixture.wait_client_configure(client_id);
        fixture.flush();

        assert!(
            fixture
                .client(client_id)
                .window_for_surface(&surface)
                .maximized
        );

        let window = fixture.pinnacle().windows[0].clone();
        assert!(window.with_state(|state| state.layout_mode.is_maximized()));

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .toggle_maximized();
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():toggle_maximized()
            },
        }

        fixture.wait_client_configure(client_id);
        fixture.flush();

        assert!(
            !fixture
                .client(client_id)
                .window_for_surface(&surface)
                .maximized
        );

        let window = fixture.pinnacle().windows[0].clone();
        assert!(window.with_state(|state| !state.layout_mode.is_maximized()));
    });
}

#[test_log::test]
fn window_handle_set_floating() {
    for_each_api(|lang| {
        let (mut fixture, _) = set_up();

        let client_id = fixture.add_client();

        let _surface = fixture.spawn_windows(1, client_id).remove(0);

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_floating(true);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():set_floating(true)
            },
        }

        fixture.wait_client_configure(client_id);
        fixture.flush();

        let window = fixture.pinnacle().windows[0].clone();
        assert!(window.with_state(|state| state.layout_mode.is_floating()));

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_floating(false);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():set_floating(false)
            },
        }

        fixture.wait_client_configure(client_id);
        fixture.flush();

        let window = fixture.pinnacle().windows[0].clone();
        assert!(window.with_state(|state| !state.layout_mode.is_floating()));
    });
}

#[test_log::test]
fn window_handle_toggle_floating() {
    for_each_api(|lang| {
        let (mut fixture, _) = set_up();

        let client_id = fixture.add_client();

        let _surface = fixture.spawn_windows(1, client_id).remove(0);

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .toggle_floating();
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():toggle_floating()
            },
        }

        fixture.wait_client_configure(client_id);
        fixture.flush();

        let window = fixture.pinnacle().windows[0].clone();
        assert!(window.with_state(|state| state.layout_mode.is_floating()));

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .toggle_floating();
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():toggle_floating()
            },
        }

        fixture.wait_client_configure(client_id);
        fixture.flush();

        let window = fixture.pinnacle().windows[0].clone();
        assert!(window.with_state(|state| !state.layout_mode.is_floating()));
    });
}

#[test_log::test]
fn window_handle_set_focused() {
    for_each_api(|lang| {
        let (mut fixture, _) = set_up();

        let client_id = fixture.add_client();

        let _surfaces = fixture.spawn_windows(2, client_id);

        let keyboard = fixture.pinnacle().seat.get_keyboard().unwrap();

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .try_set_focused(false)
                    .unwrap()
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():try_set_focused(false)
            },
        }

        fixture.wait_client_configure(client_id);
        fixture.flush();

        assert_eq!(keyboard.current_focus(), None);

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_all()
                    .next()
                    .unwrap()
                    .try_set_focused(true)
                    .unwrap()
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_all()[1]:try_set_focused(true)
            },
        }

        fixture.wait_client_configure(client_id);
        fixture.flush();

        let first_window = fixture.pinnacle().windows[0].clone();
        assert_eq!(
            keyboard.current_focus(),
            Some(KeyboardFocusTarget::Window(first_window))
        );

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_all()
                    .nth(1)
                    .unwrap()
                    .try_set_focused(true)
                    .unwrap()
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_all()[2]:try_set_focused(true)
            },
        }

        fixture.wait_client_configure(client_id);
        fixture.flush();

        let second_window = fixture.pinnacle().windows[1].clone();
        assert_eq!(
            keyboard.current_focus(),
            Some(KeyboardFocusTarget::Window(second_window))
        );
    });
}

#[test_log::test]
fn window_handle_toggle_focused() {
    for_each_api(|lang| {
        let (mut fixture, _) = set_up();

        let client_id = fixture.add_client();

        let _surfaces = fixture.spawn_windows(2, client_id);

        let keyboard = fixture.pinnacle().seat.get_keyboard().unwrap();

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .try_toggle_focused()
                    .unwrap()
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():toggle_focused()
            },
        }

        fixture.wait_client_configure(client_id);
        fixture.flush();

        assert_eq!(keyboard.current_focus(), None);

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_all()
                    .next()
                    .unwrap()
                    .try_toggle_focused()
                    .unwrap()
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_all()[1]:try_toggle_focused()
            },
        }

        fixture.wait_client_configure(client_id);
        fixture.flush();

        let first_window = fixture.pinnacle().windows[0].clone();
        assert_eq!(
            keyboard.current_focus(),
            Some(KeyboardFocusTarget::Window(first_window))
        );

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_all()
                    .nth(1)
                    .unwrap()
                    .try_toggle_focused()
                    .unwrap()
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_all()[2]:try_toggle_focused()
            },
        }

        fixture.wait_client_configure(client_id);
        fixture.flush();

        let second_window = fixture.pinnacle().windows[1].clone();
        assert_eq!(
            keyboard.current_focus(),
            Some(KeyboardFocusTarget::Window(second_window))
        );
    });
}

#[test_log::test]
fn window_handle_set_decoration_mode() {
    for_each_api(|lang| {
        let (mut fixture, _) = set_up();

        let client_id = fixture.add_client();

        let _surface = fixture.spawn_windows(1, client_id).remove(0);
        let window = fixture.pinnacle().windows[0].clone();

        assert_eq!(window.with_state(|state| state.decoration_mode), None);

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_decoration_mode(pinnacle_api::window::DecorationMode::ClientSide);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():set_decoration_mode("client_side")
            },
        }

        fixture.wait_client_configure(client_id);
        fixture.flush();

        assert_eq!(
            window.with_state(|state| state.decoration_mode),
            Some(zxdg_toplevel_decoration_v1::Mode::ClientSide)
        );

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_decoration_mode(pinnacle_api::window::DecorationMode::ServerSide);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():set_decoration_mode("server_side")
            },
        }

        fixture.wait_client_configure(client_id);
        fixture.flush();

        assert_eq!(
            window.with_state(|state| state.decoration_mode),
            Some(zxdg_toplevel_decoration_v1::Mode::ServerSide)
        );
    });
}

#[test_log::test]
fn window_handle_set_vrr_demand() {
    for_each_api(|lang| {
        let (mut fixture, _) = set_up();

        let client_id = fixture.add_client();

        let _surface = fixture.spawn_windows(1, client_id).remove(0);
        let window = fixture.pinnacle().windows[0].clone();

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_vrr_demand(pinnacle_api::window::VrrDemand::when_visible());
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():set_vrr_demand("visible")
            },
        }

        assert_eq!(
            window.with_state(|state| state.vrr_demand),
            Some(pinnacle::window::window_state::VrrDemand { fullscreen: false })
        );

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_vrr_demand(pinnacle_api::window::VrrDemand::when_fullscreen());
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():set_vrr_demand("fullscreen")
            },
        }

        assert_eq!(
            window.with_state(|state| state.vrr_demand),
            Some(pinnacle::window::window_state::VrrDemand { fullscreen: true })
        );

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_vrr_demand(None);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():set_vrr_demand(nil)
            },
        }

        assert_eq!(window.with_state(|state| state.vrr_demand), None);
    });
}

#[test_log::test]
fn window_handle_move_to_tag() {
    for_each_api(|lang| {
        let (mut fixture, output) = set_up();
        output.with_state_mut(|state| {
            let tag2 = Tag::new("2".to_string());
            let tag3 = Tag::new("3".to_string());
            state.add_tags([tag2, tag3]);
        });

        let client_id = fixture.add_client();

        fixture.spawn_windows(1, client_id);

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .move_to_tag(&pinnacle_api::tag::get("2").unwrap());
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():move_to_tag(Tag.get('2'))
            },
        }

        let tags = fixture.pinnacle().windows[0].with_state(|state| state.tags.clone());

        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name(), "2");
    });
}

#[test_log::test]
fn window_handle_set_tag() {
    for_each_api(|lang| {
        let (mut fixture, output) = set_up();
        output.with_state_mut(|state| {
            let tag2 = Tag::new("2".to_string());
            let tag3 = Tag::new("3".to_string());
            state.add_tags([tag2, tag3]);
        });

        let client_id = fixture.add_client();

        fixture.spawn_windows(1, client_id);

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_tag(&pinnacle_api::tag::get("2").unwrap(), true);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():set_tag(Tag.get('2'), true)
            },
        }

        let tags = fixture.pinnacle().windows[0].with_state(|state| state.tags.clone());

        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].name(), "1");
        assert_eq!(tags[1].name(), "2");

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_tag(&pinnacle_api::tag::get("1").unwrap(), false);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():set_tag(Tag.get('1'), false)
            },
        }

        let tags = fixture.pinnacle().windows[0].with_state(|state| state.tags.clone());

        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name(), "2");
    });
}

#[test_log::test]
fn window_handle_toggle_tag() {
    for_each_api(|lang| {
        let (mut fixture, output) = set_up();
        output.with_state_mut(|state| {
            let tag2 = Tag::new("2".to_string());
            let tag3 = Tag::new("3".to_string());
            state.add_tags([tag2, tag3]);
        });

        let client_id = fixture.add_client();

        fixture.spawn_windows(1, client_id);

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .toggle_tag(&pinnacle_api::tag::get("2").unwrap());
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():toggle_tag(Tag.get('2'))
            },
        }

        let tags = fixture.pinnacle().windows[0].with_state(|state| state.tags.clone());

        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].name(), "1");
        assert_eq!(tags[1].name(), "2");

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .toggle_tag(&pinnacle_api::tag::get("1").unwrap());
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():toggle_tag(Tag.get('1'))
            },
        }

        let tags = fixture.pinnacle().windows[0].with_state(|state| state.tags.clone());

        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name(), "2");
    });
}

#[test_log::test]
fn window_handle_set_tags() {
    for_each_api(|lang| {
        let (mut fixture, output) = set_up();
        output.with_state_mut(|state| {
            let tag2 = Tag::new("2".to_string());
            let tag3 = Tag::new("3".to_string());
            state.add_tags([tag2, tag3]);
        });

        let client_id = fixture.add_client();

        fixture.spawn_windows(1, client_id);

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused().unwrap().set_tags([
                    pinnacle_api::tag::get("2").unwrap(),
                    pinnacle_api::tag::get("3").unwrap(),
                ]);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():set_tags({ Tag.get("2"), Tag.get("3") })
            },
        }

        let tags = fixture.pinnacle().windows[0].with_state(|state| state.tags.clone());

        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].name(), "2");
        assert_eq!(tags[1].name(), "3");

        // Setting tags to empty shouldn't do anything

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_tags::<pinnacle_api::tag::TagHandle>([]);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():set_tags({})
            },
        }

        let tags = fixture.pinnacle().windows[0].with_state(|state| state.tags.clone());

        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].name(), "2");
        assert_eq!(tags[1].name(), "3");
    });
}

#[test_log::test]
fn window_handle_raise() {
    for_each_api(|lang| {
        let (mut fixture, _) = set_up();

        let client_id = fixture.add_client();

        fixture.spawn_windows(2, client_id);

        let top = fixture
            .pinnacle()
            .z_index_stack
            .last()
            .unwrap()
            .window()
            .unwrap()
            .clone();
        let second = fixture.pinnacle().windows[1].clone();
        assert_eq!(top, second);

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_all().next().unwrap().raise();
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_all()[1]:raise()
            },
        }

        let top = fixture
            .pinnacle()
            .z_index_stack
            .last()
            .unwrap()
            .window()
            .unwrap()
            .clone();
        let first = fixture.pinnacle().windows[0].clone();
        assert_eq!(top, first);
    });
}

#[test_log::test]
fn window_handle_lower() {
    for_each_api(|lang| {
        let (mut fixture, _) = set_up();

        let client_id = fixture.add_client();

        fixture.spawn_windows(2, client_id);

        let top = fixture
            .pinnacle()
            .z_index_stack
            .last()
            .unwrap()
            .window()
            .unwrap()
            .clone();
        let second = fixture.pinnacle().windows[1].clone();
        assert_eq!(top, second);

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_all().nth(1).unwrap().lower();
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_all()[2]:lower()
            },
        }

        let top = fixture
            .pinnacle()
            .z_index_stack
            .last()
            .unwrap()
            .window()
            .unwrap()
            .clone();
        let first = fixture.pinnacle().windows[0].clone();
        assert_eq!(top, first);
    });
}

#[test_log::test]
fn window_handle_is_on_active_tag() {
    for_each_api(|lang| {
        let (mut fixture, output) = set_up();

        let client_id = fixture.add_client();

        fixture.spawn_windows(1, client_id);

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                assert!(
                    pinnacle_api::window::get_all()
                        .next()
                        .unwrap()
                        .is_on_active_tag()
                );
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                assert(Window.get_all()[1]:is_on_active_tag())
            },
        }

        output.with_state(|state| {
            state.tags[0].set_active(false);
        });

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                assert!(
                    !pinnacle_api::window::get_all()
                        .next()
                        .unwrap()
                        .is_on_active_tag()
                );
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                assert(not Window.get_all()[1]:is_on_active_tag())
            },
        }
    });
}

#[test_log::test]
fn window_handle_loc() {
    for_each_api(|lang| {
        let (mut fixture, _) = set_up();

        let client_id = fixture.add_client();

        fixture.spawn_windows(1, client_id);

        let window = fixture.pinnacle().windows[0].clone();
        let loc = fixture.pinnacle().space.element_location(&window).unwrap();

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                let point = pinnacle_api::window::get_focused().unwrap().loc().unwrap();
                assert_eq!(point.x, loc.x);
                assert_eq!(point.y, loc.y);
            }),
            Lang::Lua => {
                let x = loc.x;
                let y = loc.y;
                spawn_lua_blocking! {
                    fixture,
                    local point = Window.get_focused():loc()
                    assert(point.x == $x)
                    assert(point.y == $y)
                };
            }
        }
    });
}

#[test_log::test]
fn window_handle_size() {
    for_each_api(|lang| {
        let (mut fixture, _) = set_up();

        let client_id = fixture.add_client();

        fixture.spawn_windows(1, client_id);

        let window = fixture.pinnacle().windows[0].clone();
        let actual_size = fixture
            .pinnacle()
            .space
            .element_geometry(&window)
            .unwrap()
            .size;

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                let api_size = pinnacle_api::window::get_focused().unwrap().size().unwrap();
                assert_eq!(api_size.w, actual_size.w as u32);
                assert_eq!(api_size.h, actual_size.h as u32);
            }),
            Lang::Lua => {
                let w = actual_size.w;
                let h = actual_size.h;
                spawn_lua_blocking! {
                    fixture,
                    local size = Window.get_focused():size()
                    assert(size.width == $w)
                    assert(size.height == $h)
                };
            }
        }
    });
}

#[test_log::test]
fn window_handle_app_id() {
    let (mut fixture, _) = set_up();

    let client_id = fixture.add_client();

    let app_id = "mango";

    fixture.spawn_window_with(client_id, |win| {
        win.set_app_id(app_id);
    });

    fixture.spawn_blocking(move || {
        assert_eq!(
            pinnacle_api::window::get_focused().unwrap().app_id(),
            app_id
        );
    });
    spawn_lua_blocking! {
        fixture,
        assert(Window.get_focused():app_id() == $app_id)
    };
}

#[test_log::test]
fn window_handle_title() {
    let (mut fixture, _) = set_up();

    let client_id = fixture.add_client();

    let title = "a title";

    fixture.spawn_window_with(client_id, |win| {
        win.set_title(title);
    });

    fixture.spawn_blocking(move || {
        assert_eq!(pinnacle_api::window::get_focused().unwrap().title(), title);
    });
    spawn_lua_blocking! {
        fixture,
        assert(Window.get_focused():title() == $title)
    };
}

#[test_log::test]
fn window_handle_focused() {
    let (mut fixture, _) = set_up();

    let client_id = fixture.add_client();

    fixture.spawn_windows(2, client_id);

    fixture.spawn_blocking(move || {
        let mut windows = pinnacle_api::window::get_all();
        assert!(!windows.next().unwrap().focused());
        assert!(windows.next().unwrap().focused());
    });
    spawn_lua_blocking! {
        fixture,
        local windows = Window.get_all()
        assert(not windows[1]:focused())
        assert(windows[2]:focused())
    };
}

#[test_log::test]
fn window_handle_output() {
    let (mut fixture, output) = set_up();

    let client_id = fixture.add_client();

    fixture.spawn_windows(1, client_id);

    fixture.spawn_blocking({
        let output_name = output.name();
        move || {
            assert_eq!(
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .output()
                    .unwrap()
                    .name(),
                output_name
            );
        }
    });

    let output_name = output.name();
    spawn_lua_blocking! {
        fixture,
        assert(Window.get_focused():output().name == $output_name)
    };
}

#[test_log::test]
fn window_handle_tiled() {
    let (mut fixture, _) = set_up();

    let client_id = fixture.add_client();

    fixture.spawn_windows(1, client_id);

    fixture.spawn_blocking(move || {
        assert_eq!(
            pinnacle_api::window::get_focused().unwrap().layout_mode(),
            pinnacle_api::window::LayoutMode::Tiled
        );
    });
    spawn_lua_blocking! {
        fixture,
        local win = Window.get_focused()
        assert(win:tiled())
        assert(not win:floating())
        assert(not win:fullscreen())
        assert(not win:maximized())
    };
}

#[test_log::test]
fn window_handle_floating() {
    let (mut fixture, _) = set_up();

    let client_id = fixture.add_client();

    fixture.spawn_floating_window_with(client_id, (500, 500), |_| ());

    fixture.spawn_blocking(move || {
        assert_eq!(
            pinnacle_api::window::get_focused().unwrap().layout_mode(),
            pinnacle_api::window::LayoutMode::Floating
        );
    });
    spawn_lua_blocking! {
        fixture,
        local win = Window.get_focused()
        assert(not win:tiled())
        assert(win:floating())
        assert(not win:fullscreen())
        assert(not win:maximized())
    };
}

#[test_log::test]
fn window_handle_fullscreen() {
    let (mut fixture, _) = set_up();

    let client_id = fixture.add_client();

    fixture.spawn_windows(1, client_id);

    fixture.spawn_blocking(|| {
        pinnacle_api::window::get_focused()
            .unwrap()
            .set_fullscreen(true);
    });

    fixture.spawn_blocking(move || {
        assert_eq!(
            pinnacle_api::window::get_focused().unwrap().layout_mode(),
            pinnacle_api::window::LayoutMode::Fullscreen
        );
    });
    spawn_lua_blocking! {
        fixture,
        local win = Window.get_focused()
        assert(not win:tiled())
        assert(not win:floating())
        assert(win:fullscreen())
        assert(not win:maximized())
    };
}

#[test_log::test]
fn window_handle_maximized() {
    let (mut fixture, _) = set_up();

    let client_id = fixture.add_client();

    fixture.spawn_windows(1, client_id);

    fixture.spawn_blocking(|| {
        pinnacle_api::window::get_focused()
            .unwrap()
            .set_maximized(true);
    });

    fixture.spawn_blocking(move || {
        assert_eq!(
            pinnacle_api::window::get_focused().unwrap().layout_mode(),
            pinnacle_api::window::LayoutMode::Maximized
        );
    });
    spawn_lua_blocking! {
        fixture,
        local win = Window.get_focused()
        assert(not win:tiled())
        assert(not win:floating())
        assert(not win:fullscreen())
        assert(win:maximized())
    };
}

#[test_log::test]
fn window_handle_tags() {
    let (mut fixture, _) = set_up();

    let client_id = fixture.add_client();

    fixture.spawn_windows(1, client_id);

    fixture.spawn_blocking(|| {
        let tags = pinnacle_api::window::get_focused()
            .unwrap()
            .tags()
            .collect::<Vec<_>>();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name(), "1");
    });

    spawn_lua_blocking! {
        fixture,
        local tags = Window.get_focused():tags()
        assert(#tags == 1)
        assert(tags[1]:name() == "1")
    };
}

#[test_log::test]
fn window_handle_in_direction() {
    let (mut fixture, _) = set_up();

    let client_id = fixture.add_client();

    // Master stack = two windows side by side
    fixture.spawn_windows(2, client_id);

    fixture.spawn_blocking(|| {
        let left_win = pinnacle_api::window::get_all().next().unwrap();
        let left = left_win
            .in_direction(pinnacle_api::util::Direction::Left)
            .count();
        let right = left_win
            .in_direction(pinnacle_api::util::Direction::Right)
            .count();
        let up = left_win
            .in_direction(pinnacle_api::util::Direction::Up)
            .count();
        let down = left_win
            .in_direction(pinnacle_api::util::Direction::Down)
            .count();

        assert_eq!(left, 0);
        assert_eq!(right, 1);
        assert_eq!(up, 0);
        assert_eq!(down, 0);
    });

    spawn_lua_blocking! {
        fixture,
        local left_win = Window.get_all()[1]
        local left = #left_win:in_direction("left")
        local right = #left_win:in_direction("right")
        local up = #left_win:in_direction("up")
        local down = #left_win:in_direction("down")

        assert(left == 0)
        assert(right == 1)
        assert(up == 0)
        assert(down == 0)
    };
}

#[test_log::test]
fn window_handle_move_to_output() {
    for_each_api(|lang| {
        let (mut fixture, output1) = set_up();

        let output_geo = Rectangle::new((1920, 0).into(), (1920, 1080).into());
        let output2 = fixture.add_output(output_geo);
        output2.with_state_mut(|state| {
            let tag = Tag::new("1".to_string());
            tag.set_active(true);
            state.add_tags([tag]);
        });
        fixture.pinnacle().focus_output(&output1);

        let client_id = fixture.add_client();

        let _surface = fixture.spawn_floating_window_with(client_id, (500, 500), |_| ());

        let output_name = output2.name();

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                let handle = OutputHandle::from_name(output_name);
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .move_to_output(&handle);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():move_to_output(Output.get_by_name($output_name))
            },
        }

        fixture.wait_client_configure(client_id);
        fixture.flush();

        let window = fixture.pinnacle().windows[0].clone();
        let geo = fixture.pinnacle().space.element_geometry(&window).unwrap();

        assert!(output_geo.overlaps(geo));
    });
}

#[test_log::test]
fn window_handle_swap_simple() {
    for_each_api(|lang| {
        let (mut fixture, _output1) = set_up();

        let client_id = fixture.add_client();

        let _surfaces = fixture.spawn_windows(4, client_id);

        let window = fixture.pinnacle().windows[0].clone();
        let target = fixture.pinnacle().windows[2].clone();

        let window_id = window.with_state(|s| s.id.0);
        let target_id = target.with_state(|s| s.id.0);

        let window_geo = fixture.pinnacle().space.element_geometry(&window);
        let target_geo = fixture.pinnacle().space.element_geometry(&target);

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                let window_handle = pinnacle_api::window::WindowHandle::from_id(window_id);
                let target_handle = pinnacle_api::window::WindowHandle::from_id(target_id);

                window_handle.swap(&target_handle);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                local window_handle = Window.handle.new($window_id)
                local target_handle = Window.handle.new($target_id)

                window_handle:swap(target_handle)
            },
        };

        fixture.wait_client_configure(client_id);
        fixture.flush();

        assert_eq!(
            window_geo,
            fixture.pinnacle().space.element_geometry(&target)
        );
        assert_eq!(
            target_geo,
            fixture.pinnacle().space.element_geometry(&window)
        );
    })
}

#[test_log::test]
fn window_handle_swap_multi_output() {
    for_each_api(|lang| {
        let (mut fixture, _output1) = set_up();

        let client_id = fixture.add_client();

        let mut surfaces = fixture.spawn_windows(1, client_id);

        let output2 = fixture.add_output(Rectangle::new((1920, 0).into(), (1920, 1080).into()));
        output2.with_state_mut(|state| {
            let tag = Tag::new("2".to_string());
            tag.set_active(true);
            state.add_tags([tag]);
        });

        fixture.pinnacle().focus_output(&output2);

        surfaces.append(&mut fixture.spawn_windows(1, client_id));

        let window = fixture.pinnacle().windows[0].clone();
        let target = fixture.pinnacle().windows[1].clone();

        let window_id = window.with_state(|s| s.id.0);
        let target_id = target.with_state(|s| s.id.0);

        let window_geo = fixture.pinnacle().space.element_geometry(&window);
        let target_geo = fixture.pinnacle().space.element_geometry(&target);

        let window_tags = window.with_state(|state| state.tags.clone());
        let target_tags = target.with_state(|state| state.tags.clone());

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                let window_handle = pinnacle_api::window::WindowHandle::from_id(window_id);
                let target_handle = pinnacle_api::window::WindowHandle::from_id(target_id);

                window_handle.swap(&target_handle);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                local window_handle = Window.handle.new($window_id)
                local target_handle = Window.handle.new($target_id)

                window_handle:swap(target_handle)
            },
        };

        fixture.dispatch_until(|fixture| !fixture.pinnacle().layout_state.pending_swap);

        assert_eq!(
            window_geo,
            fixture.pinnacle().space.element_geometry(&target)
        );
        assert_eq!(
            target_geo,
            fixture.pinnacle().space.element_geometry(&window)
        );

        assert_eq!(window_tags, target.with_state(|state| state.tags.clone()));
        assert_eq!(target_tags, window.with_state(|state| state.tags.clone()));
    })
}

#[test_log::test]
fn window_handle_swap_non_tiled() {
    for_each_api(|lang| {
        let (mut fixture, _output1) = set_up();

        let client_id = fixture.add_client();
        let mut surfaces = fixture.spawn_windows(1, client_id);

        surfaces.push(fixture.spawn_floating_window_with(client_id, (500, 500), |_| ()));

        let window = fixture.pinnacle().windows[0].clone();
        let target = fixture.pinnacle().windows[1].clone();

        let window_id = window.with_state(|s| s.id.0);
        let target_id = target.with_state(|s| s.id.0);

        let window_geo = fixture.pinnacle().space.element_geometry(&window);
        let target_geo = fixture.pinnacle().space.element_geometry(&target);

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                let window_handle = pinnacle_api::window::WindowHandle::from_id(window_id);
                let target_handle = pinnacle_api::window::WindowHandle::from_id(target_id);

                window_handle.swap(&target_handle);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                local window_handle = Window.handle.new($window_id)
                local target_handle = Window.handle.new($target_id)

                window_handle:swap(target_handle)
            },
        };

        fixture.wait_client_configure(client_id);
        fixture.flush();

        fixture.dispatch_until(|fixture| !fixture.pinnacle().layout_state.pending_swap);

        assert_eq!(
            window_geo,
            fixture.pinnacle().space.element_geometry(&target)
        );
        assert_eq!(
            target_geo,
            fixture.pinnacle().space.element_geometry(&window)
        );

        assert!(window.with_state(|state| state.layout_mode.is_floating()));
        assert!(target.with_state(|state| state.layout_mode.is_tiled()));
    })
}

#[test_log::test]
fn window_handle_swap_non_tiled_multi_output() {
    for_each_api(|lang| {
        let (mut fixture, _output1) = set_up();

        let client_id = fixture.add_client();
        let mut surfaces = fixture.spawn_windows(1, client_id);

        let output2 = fixture.add_output(Rectangle::new((1920, 0).into(), (1920, 1080).into()));
        output2.with_state_mut(|state| {
            let tag = Tag::new("2".to_string());
            tag.set_active(true);
            state.add_tags([tag]);
        });
        fixture.pinnacle().focus_output(&output2);

        surfaces.push(fixture.spawn_floating_window_with(client_id, (500, 500), |_| ()));

        let window = fixture.pinnacle().windows[0].clone();
        let target = fixture.pinnacle().windows[1].clone();

        let window_id = window.with_state(|s| s.id.0);
        let target_id = target.with_state(|s| s.id.0);

        let window_geo = fixture.pinnacle().space.element_geometry(&window);
        let target_geo = fixture.pinnacle().space.element_geometry(&target);

        let window_tags = window.with_state(|state| state.tags.clone());
        let target_tags = target.with_state(|state| state.tags.clone());

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                let window_handle = pinnacle_api::window::WindowHandle::from_id(window_id);
                let target_handle = pinnacle_api::window::WindowHandle::from_id(target_id);

                window_handle.swap(&target_handle);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                local window_handle = Window.handle.new($window_id)
                local target_handle = Window.handle.new($target_id)

                window_handle:swap(target_handle)
            },
        };

        fixture.wait_client_configure(client_id);
        fixture.flush();

        fixture.dispatch_until(|fixture| !fixture.pinnacle().layout_state.pending_swap);

        assert_eq!(
            window_geo,
            fixture.pinnacle().space.element_geometry(&target)
        );
        assert_eq!(
            target_geo,
            fixture.pinnacle().space.element_geometry(&window)
        );

        assert_eq!(window_tags, target.with_state(|state| state.tags.clone()));
        assert_eq!(target_tags, window.with_state(|state| state.tags.clone()));

        assert!(window.with_state(|state| state.layout_mode.is_floating()));
        assert!(target.with_state(|state| state.layout_mode.is_tiled()));
    })
}

#[test_log::test]
fn window_handle_swap_swap_tag() {
    for_each_api(|lang| {
        let (mut fixture, output) = set_up();

        let tag = Tag::new("2".to_string());
        tag.set_active(true);
        output.with_state_mut(|state| {
            state.add_tags([tag.clone()]);
        });

        let id = fixture.add_client();

        let _surfaces = fixture.spawn_windows(4, id);

        let window = fixture.pinnacle().windows[0].clone();
        let target = fixture.pinnacle().windows[2].clone();
        target.with_state_mut(|state| state.tags = std::iter::once(tag.clone()).collect());

        let window_tags = window.with_state(|state| state.tags.clone());
        let target_tags = target.with_state(|state| state.tags.clone());

        let window_id = window.with_state(|s| s.id.0);
        let target_id = target.with_state(|s| s.id.0);

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                let window_handle = pinnacle_api::window::WindowHandle::from_id(window_id);
                let target_handle = pinnacle_api::window::WindowHandle::from_id(target_id);

                window_handle.swap(&target_handle);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                local window_handle = Window.handle.new($window_id)
                local target_handle = Window.handle.new($target_id)

                window_handle:swap(target_handle);
            },
        };

        fixture.wait_client_configure(id);
        fixture.flush();

        assert_eq!(window_tags, target.with_state(|state| state.tags.clone()));

        assert_eq!(target_tags, window.with_state(|state| state.tags.clone()));
    })
}

#[test_log::test]
fn window_handle_swap_swap_tag_multi_output() {
    for_each_api(|lang| {
        let (mut fixture, _output1) = set_up();

        let client_id = fixture.add_client();

        let mut surfaces = fixture.spawn_windows(1, client_id);

        let output2 = fixture.add_output(Rectangle::new((1920, 0).into(), (1920, 1080).into()));
        output2.with_state_mut(|state| {
            let tag = Tag::new("2".to_string());
            tag.set_active(true);
            state.add_tags([tag, Tag::new("3".to_string())]);
        });
        let tags = output2.with_state(|state| state.tags.clone());

        fixture.pinnacle().focus_output(&output2);

        surfaces.append(&mut fixture.spawn_windows(1, client_id));

        let window = fixture.pinnacle().windows[0].clone();
        let target = fixture.pinnacle().windows[1].clone();
        target.with_state_mut(|state| state.tags = tags);

        let window_id = window.with_state(|s| s.id.0);
        let target_id = target.with_state(|s| s.id.0);

        let window_geo = fixture.pinnacle().space.element_geometry(&window);
        let target_geo = fixture.pinnacle().space.element_geometry(&target);

        let window_tags = window.with_state(|state| state.tags.clone());
        let target_tags = target.with_state(|state| state.tags.clone());

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                let window_handle = pinnacle_api::window::WindowHandle::from_id(window_id);
                let target_handle = pinnacle_api::window::WindowHandle::from_id(target_id);

                window_handle.swap(&target_handle);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                local window_handle = Window.handle.new($window_id)
                local target_handle = Window.handle.new($target_id)

                window_handle:swap(target_handle)
            },
        };

        fixture.dispatch_until(|fixture| !fixture.pinnacle().layout_state.pending_swap);

        assert_eq!(
            window_geo,
            fixture.pinnacle().space.element_geometry(&target)
        );
        assert_eq!(
            target_geo,
            fixture.pinnacle().space.element_geometry(&window)
        );

        assert_eq!(window_tags, target.with_state(|state| state.tags.clone()));
        assert_eq!(target_tags, window.with_state(|state| state.tags.clone()));
    })
}

#[test_log::test]
fn window_handle_swap_swap_inactive_tag() {
    for_each_api(|lang| {
        let (mut fixture, output) = set_up();

        let tag = Tag::new("2".to_string());
        tag.set_active(false);
        output.with_state_mut(|state| {
            state.add_tags([tag.clone()]);
        });

        let id = fixture.add_client();

        let _surfaces = fixture.spawn_windows(4, id);

        let window = fixture.pinnacle().windows[0].clone();
        let target = fixture.pinnacle().windows[2].clone();
        target.with_state_mut(|state| state.tags = std::iter::once(tag.clone()).collect());

        let window_tags = window.with_state(|state| state.tags.clone());
        let target_tags = target.with_state(|state| state.tags.clone());

        let window_id = window.with_state(|s| s.id.0);
        let target_id = target.with_state(|s| s.id.0);

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                let window_handle = pinnacle_api::window::WindowHandle::from_id(window_id);
                let target_handle = pinnacle_api::window::WindowHandle::from_id(target_id);

                window_handle.swap(&target_handle);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                local window_handle = Window.handle.new($window_id)
                local target_handle = Window.handle.new($target_id)

                window_handle:swap(target_handle);
            },
        };

        fixture.wait_client_configure(id);
        fixture.flush();

        assert_eq!(window_tags, target.with_state(|state| state.tags.clone()));

        assert_eq!(target_tags, window.with_state(|state| state.tags.clone()));
    })
}

#[test_log::test]
fn window_handle_swap_swap_inactive_tag_multi_output() {
    for_each_api(|lang| {
        let (mut fixture, _output1) = set_up();

        let client_id = fixture.add_client();

        let mut surfaces = fixture.spawn_windows(1, client_id);

        let output2 = fixture.add_output(Rectangle::new((1920, 0).into(), (1920, 1080).into()));
        let inactive_tag = Tag::new("3".to_string());
        output2.with_state_mut(|state| {
            let tag = Tag::new("2".to_string());
            tag.set_active(true);
            state.add_tags([tag, inactive_tag.clone()]);
        });

        fixture.pinnacle().focus_output(&output2);

        surfaces.append(&mut fixture.spawn_windows(1, client_id));

        let window = fixture.pinnacle().windows[0].clone();
        let target = fixture.pinnacle().windows[1].clone();
        target.with_state_mut(|state| state.tags = std::iter::once(inactive_tag.clone()).collect());

        let window_id = window.with_state(|s| s.id.0);
        let target_id = target.with_state(|s| s.id.0);

        let window_tags = window.with_state(|state| state.tags.clone());
        let target_tags = target.with_state(|state| state.tags.clone());

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                let window_handle = pinnacle_api::window::WindowHandle::from_id(window_id);
                let target_handle = pinnacle_api::window::WindowHandle::from_id(target_id);

                window_handle.swap(&target_handle);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                local window_handle = Window.handle.new($window_id)
                local target_handle = Window.handle.new($target_id)

                window_handle:swap(target_handle)
            },
        };

        fixture.dispatch_until(|fixture| !fixture.pinnacle().layout_state.pending_swap);

        assert_eq!(window_tags, target.with_state(|state| state.tags.clone()));
        assert_eq!(target_tags, window.with_state(|state| state.tags.clone()));
    })
}

// TODO: window_begin_move
// TODO: window_begin_resize
// TODO: window_connect_signal
// TODO: window_add_window_rule
