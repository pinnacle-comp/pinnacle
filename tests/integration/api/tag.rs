use pinnacle::{state::WithState, tag::Tag};
use pinnacle_api::layout::LayoutNode;
use smithay::{output::Output, utils::Rectangle};

use crate::{
    common::{Lang, fixture::Fixture, for_each_api},
    spawn_lua_blocking,
};

fn set_up() -> (Fixture, Output, Output, Vec<Tag>, Vec<Tag>) {
    let mut fixture = Fixture::new();

    let output1 = fixture.add_output(Rectangle::new((0, 0).into(), (1920, 1080).into()));
    output1.with_state_mut(|state| {
        let tag = Tag::new("1".to_string());
        tag.set_active(true);
        let tag2 = Tag::new("2".to_string());
        let tag3 = Tag::new("3".to_string());
        state.add_tags([tag, tag2, tag3]);
    });

    let output2 = fixture.add_output(Rectangle::new((1920, 0).into(), (1920, 1080).into()));
    output2.with_state_mut(|state| {
        let tag = Tag::new("4".to_string());
        tag.set_active(true);
        let tag2 = Tag::new("5".to_string());
        let tag3 = Tag::new("6".to_string());
        state.add_tags([tag, tag2, tag3]);
    });

    fixture.pinnacle().focus_output(&output1);

    fixture
        .runtime_handle()
        .block_on(pinnacle_api::connect())
        .unwrap();

    let tags1 = output1.with_state(|state| state.tags.clone());
    let tags2 = output2.with_state(|state| state.tags.clone());

    (
        fixture,
        output1,
        output2,
        tags1.into_iter().collect(),
        tags2.into_iter().collect(),
    )
}

#[test_log::test]
fn tag_get_all() {
    let (mut fixture, ..) = set_up();

    fixture.spawn_blocking(|| {
        assert_eq!(pinnacle_api::tag::get_all().count(), 6);
    });

    spawn_lua_blocking! {
        fixture,
        assert(#Tag.get_all() == 6)
    }
}

#[test_log::test]
fn tag_get() {
    let (mut fixture, _, output2, ..) = set_up();

    fixture.spawn_blocking({
        let output2_name = output2.name();
        move || {
            let tag = pinnacle_api::tag::get("1");
            assert!(tag.is_some());

            let tag = pinnacle_api::tag::get("4");
            assert!(tag.is_none());

            let tag = pinnacle_api::tag::get_on_output(
                "4",
                &pinnacle_api::output::get_by_name(&output2_name).unwrap(),
            );
            assert!(tag.is_some());
        }
    });

    let output2_name = output2.name();
    spawn_lua_blocking! {
        fixture,
        local tag = Tag.get("1")
        assert(tag)

        local tag = Tag.get("4")
        assert(not tag)

        local tag = Tag.get("4", Output.get_by_name($output2_name))
        assert(tag)
    }
}

#[test_log::test]
fn tag_add() {
    for_each_api(|lang| {
        let (mut fixture, output, ..) = set_up();

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                let tags = pinnacle_api::tag::add(
                    &pinnacle_api::output::get_focused().unwrap(),
                    ["nubby's", "number", "factory"],
                );
                assert_eq!(tags.count(), 3);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                local tags = Tag.add(Output.get_focused(), "nubby's", "number", "factory")
                assert(#tags == 3)
            },
        }

        let tag_count = output.with_state(|state| state.tags.len());
        assert_eq!(tag_count, 6);
    });
}

#[test_log::test]
fn tag_remove() {
    for_each_api(|lang| {
        let (mut fixture, output, ..) = set_up();

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                let mut tags = pinnacle_api::output::get_focused().unwrap().tags();

                pinnacle_api::tag::remove([tags.next().unwrap()]);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                local tags = Output.get_focused():tags()
                Tag.remove({ tags[1] })
            },
        }

        let tag_count = output.with_state(|state| state.tags.len());
        assert_eq!(tag_count, 2);
    });
}

#[test_log::test]
fn tag_handle_remove() {
    for_each_api(|lang| {
        let (mut fixture, output, ..) = set_up();

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                let mut tags = pinnacle_api::output::get_focused().unwrap().tags();

                tags.next().unwrap().remove();
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                local tags = Output.get_focused():tags()
                tags[1]:remove()
            },
        }

        let tag_count = output.with_state(|state| state.tags.len());
        assert_eq!(tag_count, 2);
    });
}

#[test_log::test]
fn tag_handle_switch_to() {
    for_each_api(|lang| {
        let (mut fixture, output, ..) = set_up();

        output.with_state(|state| {
            state.tags[0].set_active(true);
            state.tags[1].set_active(true);
            state.tags[2].set_active(true);
        });

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                pinnacle_api::tag::get("2").unwrap().switch_to();
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Tag.get("2"):switch_to()
            },
        }

        output.with_state(|state| {
            assert!(!state.tags[0].active());
            assert!(state.tags[1].active());
            assert!(!state.tags[2].active());
        });
    });
}

#[test_log::test]
fn tag_handle_set_active() {
    for_each_api(|lang| {
        let (mut fixture, output, ..) = set_up();

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                pinnacle_api::tag::get("1").unwrap().set_active(false);
                pinnacle_api::tag::get("2").unwrap().set_active(true);
                pinnacle_api::tag::get("3").unwrap().set_active(true);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Tag.get("1"):set_active(false)
                Tag.get("2"):set_active(true)
                Tag.get("3"):set_active(true)
            },
        }

        output.with_state(|state| {
            assert!(!state.tags[0].active());
            assert!(state.tags[1].active());
            assert!(state.tags[2].active());
        });
    });
}

#[test_log::test]
fn tag_handle_toggle_active() {
    for_each_api(|lang| {
        let (mut fixture, output, ..) = set_up();

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                pinnacle_api::tag::get("1").unwrap().toggle_active();
                pinnacle_api::tag::get("2").unwrap().toggle_active();
                pinnacle_api::tag::get("3").unwrap().toggle_active();
                pinnacle_api::tag::get("3").unwrap().toggle_active();
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Tag.get("1"):toggle_active()
                Tag.get("2"):toggle_active()
                Tag.get("3"):toggle_active()
                Tag.get("3"):toggle_active()
            },
        }

        output.with_state(|state| {
            assert!(!state.tags[0].active());
            assert!(state.tags[1].active());
            assert!(!state.tags[2].active());
        });
    });
}

#[test_log::test]
fn tag_handle_active() {
    let (mut fixture, ..) = set_up();

    fixture.spawn_blocking(move || {
        assert!(pinnacle_api::tag::get("1").unwrap().active());
        assert!(!pinnacle_api::tag::get("2").unwrap().active());
        assert!(!pinnacle_api::tag::get("3").unwrap().active());
    });

    spawn_lua_blocking! {
        fixture,
        assert(Tag.get("1"):active())
        assert(not Tag.get("2"):active())
        assert(not Tag.get("3"):active())
    }
}

#[test_log::test]
fn tag_handle_name() {
    let (mut fixture, ..) = set_up();

    fixture.spawn_blocking(move || {
        assert_eq!(pinnacle_api::tag::get("1").unwrap().name(), "1");
        assert_eq!(pinnacle_api::tag::get("2").unwrap().name(), "2");
        assert_eq!(pinnacle_api::tag::get("3").unwrap().name(), "3");
    });

    spawn_lua_blocking! {
        fixture,
        assert(Tag.get("1"):name() == "1")
        assert(Tag.get("2"):name() == "2")
        assert(Tag.get("3"):name() == "3")
    }
}

#[test_log::test]
fn tag_handle_output() {
    let (mut fixture, output1, output2, ..) = set_up();

    fixture.spawn_blocking({
        let output1_name = output1.name();
        let output2_name = output2.name();
        move || {
            let tag = pinnacle_api::tag::get("1").unwrap();
            assert_eq!(tag.output().name(), output1_name);

            let tag = pinnacle_api::tag::get_on_output(
                "4",
                &pinnacle_api::output::get_by_name(&output2_name).unwrap(),
            )
            .unwrap();
            assert_eq!(tag.output().name(), output2_name);
        }
    });

    let output1_name = output1.name();
    let output2_name = output2.name();
    spawn_lua_blocking! {
        fixture,
        local tag = Tag.get("1")
        assert(tag:output().name == $output1_name)

        local tag = Tag.get("4", Output.get_by_name($output2_name))
        assert(tag:output().name == $output2_name)
    }
}

#[test_log::test]
fn tag_handle_windows() {
    let (mut fixture, ..) = set_up();

    fixture.spawn_blocking(|| {
        pinnacle_api::layout::manage(|_| pinnacle_api::layout::LayoutResponse {
            root_node: LayoutNode::new(),
            tree_id: 0,
        })
    });

    let id = fixture.add_client();
    fixture.spawn_windows(1, id);

    fixture.spawn_blocking(move || {
        let tag = pinnacle_api::tag::get("1").unwrap();
        assert_eq!(tag.windows().count(), 1);

        let tag = pinnacle_api::tag::get("2").unwrap();
        assert_eq!(tag.windows().count(), 0);
    });

    spawn_lua_blocking! {
        fixture,
        local tag = Tag.get("1")
        assert(#tag:windows() == 1)

        local tag = Tag.get("2")
        assert(#tag:windows() == 0)
    }
}

#[test_log::test]
fn tag_get_all_does_not_return_tags_cleared_after_config_reload() {
    for_each_api(|lang| {
        let (mut fixture, ..) = set_up();

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                assert_eq!(pinnacle_api::tag::get_all().count(), 6);
                pinnacle_api::pinnacle::reload_config();
                assert_eq!(pinnacle_api::tag::get_all().count(), 0);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                assert(#Tag.get_all() == 6)
                Pinnacle.reload_config()
                assert(#Tag.get_all() == 0)
            },
        }
    });
}

#[test_log::test]
fn tag_get_does_not_return_tags_cleared_after_config_reload() {
    for_each_api(|lang| {
        let (mut fixture, ..) = set_up();

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                assert!(pinnacle_api::tag::get("1").is_some());
                pinnacle_api::pinnacle::reload_config();
                assert!(pinnacle_api::tag::get("1").is_none());
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                assert(Tag.get("1"))
                Pinnacle.reload_config()
                assert(not Tag.get("1"))
            },
        }
    });
}

// TODO: tag connect_signal
