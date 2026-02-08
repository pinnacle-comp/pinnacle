use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, OnceLock},
};

use indexmap::IndexSet;
use mlua::{UserData, UserDataMethods};
use pinnacle::{state::WithState, tag::Tag};
use pinnacle_api::{layout::LayoutNode, output::OutputHandle, signal::TagSignal, tag::TagHandle};
use proptest::prelude::*;
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

/// Arguments for testing `tag.move_to_output`.
#[derive(Debug, Clone)]
struct TagMoveToOutputArgs {
    /// A 2d map of tags that should be active.
    ///
    /// Each nested vec maps to one output.
    /// For each output, the length of the vec is the amount of tags
    /// the output will have. `true` will set that tag active.
    tag_actives: Vec<Vec<bool>>,
    /// The index of the output to focus.
    output_to_focus: usize,
    /// A 2d map of tags that will be moved to the target output.
    ///
    /// Similar to `tag_actives`. `true` means "move this tag".
    tags_to_move: Vec<Vec<bool>>,
    /// The index of the output that tags will be moved to.
    output_to_move_to: usize,
}

fn tag_move_to_output_args() -> impl Strategy<Value = TagMoveToOutputArgs> {
    (2usize..=10)
        .prop_flat_map(|output_count| {
            (
                0..output_count,
                0..output_count,
                vec![2usize..=10; output_count],
            )
        })
        .prop_flat_map(|(output_to_focus, output_to_move_to, tag_counts)| {
            (
                Just(output_to_focus),
                Just(output_to_move_to),
                tag_counts
                    .iter()
                    .map(|&tag_count| vec![proptest::bool::ANY; tag_count])
                    .collect::<Vec<_>>(),
                tag_counts
                    .iter()
                    .map(|&tag_count| vec![proptest::bool::weighted(0.1); tag_count])
                    .collect::<Vec<_>>(),
            )
        })
        .prop_map(
            |(output_to_focus, output_to_move_to, tag_actives, tags_to_move)| TagMoveToOutputArgs {
                tag_actives,
                output_to_focus,
                tags_to_move,
                output_to_move_to,
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 20,
        ..ProptestConfig::default()
    })]
    #[test_log::test]
    fn tag_move_to_output(
        args in tag_move_to_output_args()
    ) {
        for_each_api(|lang| {
            let mut fixture = Fixture::new();

            fixture
                .runtime_handle()
                .block_on(pinnacle_api::connect())
                .unwrap();

            let TagMoveToOutputArgs {
                tag_actives,
                output_to_focus,
                tags_to_move,
                output_to_move_to
            } = args.clone();

            let mut outputs = Vec::new();
            let mut tags = Vec::new();
            let mut tags_to_mv = Vec::new();

            for (i, (tag_mask, tag_move)) in tag_actives.clone().into_iter().zip(tags_to_move.clone()).enumerate() {
                let output = fixture.add_output(Rectangle::new((1920 * i as i32, 0).into(), (1920, 1080).into()));
                output.with_state_mut(|state| {
                    let tgs = tag_mask
                        .into_iter()
                        .zip(tag_move)
                        .map(|(active, r#move)| {
                            let tag = Tag::new("mama mia".to_string());
                            tag.set_active(active);

                            if r#move {
                                tags_to_mv.push(tag.clone());
                            }

                            tag
                        })
                        .collect::<Vec<_>>();
                    state.add_tags(tgs.clone());
                    tags.push(tgs);
                });
                outputs.push(output);
            }

            fixture.pinnacle().focus_output(&outputs[output_to_focus]);

            if outputs[output_to_focus].with_state(|state| state.focused_tags().next().is_none()) {
                // Just ignore this case
                return;
            }

            let id = fixture.add_client();
            fixture.spawn_floating_window_with(id, (500, 500), |_| ());

            let window_has_moved_tags = !outputs[output_to_focus]
                .with_state(|state| {
                    state.focused_tags().cloned().collect::<IndexSet<_>>()
                })
                .is_disjoint(&tags_to_mv.iter().cloned().collect::<IndexSet<_>>());

            let target_output_name = outputs[output_to_move_to].name();
            let focused_output_name = outputs[output_to_focus].name();
            let tag_ids_to_move = tags_to_mv
                .into_iter()
                .map(|tag| tag.id().to_inner())
                .collect::<Vec<_>>();

            match lang {
                Lang::Rust => fixture.spawn_blocking(move || {
                    let target_output = OutputHandle::from_name(target_output_name);
                    let focused_output = OutputHandle::from_name(focused_output_name);
                    let target_tags = tag_ids_to_move
                        .into_iter()
                        .map(TagHandle::from_id)
                        .collect::<Vec<_>>();

                    let ret = pinnacle_api::tag::move_to_output(&target_output, target_tags.clone());

                    match ret {
                        Ok(()) => {
                            let win = pinnacle_api::window::get_all().next().unwrap();

                            if window_has_moved_tags {
                                assert_eq!(win.output().as_ref(), Some(&target_output), "win not on target output");
                            } else {
                                assert_eq!(win.output().as_ref(), Some(&focused_output), "win not on focused output, moved");
                            }

                            for tag in target_tags {
                                assert_eq!(tag.output(), target_output, "tag not on target output");
                            }
                        }
                        Err(err) => match err {
                            pinnacle_api::tag::MoveToOutputError::OutputDoesNotExist => unreachable!(),
                            pinnacle_api::tag::MoveToOutputError::SameWindowOnTwoOutputs(window_handles) => {
                                assert_eq!(window_handles.len(), 1, "win not on two ops");
                                let win = &window_handles[0];
                                assert_eq!(win.output().as_ref(), Some(&focused_output), "win not on focused output, not moved");
                            }
                        }
                    }
                }),
                Lang::Lua => spawn_lua_blocking! {
                    fixture,

                    local target_output = Output.handle.new($target_output_name)
                    local focused_output = Output.handle.new($focused_output_name)
                    local tags_to_move = Tag.handle.new_from_table($tag_ids_to_move)

                    local ok, err = Tag.move_to_output(target_output, tags_to_move)

                    if ok then
                        assert(not err, "err was non-nil")
                        local win = Window.get_all()[1]
                        local win_output = win:output().name
                        if $window_has_moved_tags then
                            assert(win_output == target_output.name, "win not on target output")
                        else
                            assert(win_output == focused_output.name, "win not on focused output, moved")
                        end

                        for _, tag in ipairs(tags_to_move) do
                            assert(tag:output().name == target_output.name, "tag not on target_output")
                        end
                    else
                        assert(not err.output_does_not_exist, "output doesn't exist")
                        assert(err.same_window_on_two_outputs, "window not on two ops")
                        local win = err.same_window_on_two_outputs[1]
                        assert(win:output().name == focused_output.name, "win not on focused output, not moved")
                    end
                }
            }
        });
    }
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

// TODO: Implement a less shady/more generic way to test signals,
// ideally something allowing to describe an expected sequence/list of signals
#[derive(Clone)]
struct TagSignalTester {
    active: Arc<Mutex<HashMap<TagHandle, bool>>>,
    created: Arc<Mutex<HashSet<TagHandle>>>,
    removed: Arc<Mutex<HashSet<TagHandle>>>,
    done: Arc<dyn Fn(&Self) -> bool + Send + Sync + 'static>,
}

impl TagSignalTester {
    fn new<F>(done: F) -> Self
    where
        F: Fn(&Self) -> bool + Send + Sync + 'static,
    {
        Self {
            active: Default::default(),
            created: Default::default(),
            removed: Default::default(),
            done: Arc::new(done),
        }
    }

    fn log_active(&self, tag: TagHandle, active: bool) {
        let mut storage = self.active.lock().unwrap();
        storage.insert(tag, active);
    }

    fn active(&self) -> &Arc<Mutex<HashMap<TagHandle, bool>>> {
        &self.active
    }

    fn log_created(&self, tag: TagHandle) {
        let mut storage = self.created.lock().unwrap();
        storage.insert(tag);
    }

    fn created(&self) -> &Arc<Mutex<HashSet<TagHandle>>> {
        &self.created
    }

    fn log_removed(&self, tag: TagHandle) {
        let mut storage = self.removed.lock().unwrap();
        storage.insert(tag);
    }

    fn removed(&self) -> &Arc<Mutex<HashSet<TagHandle>>> {
        &self.removed
    }

    fn done(&self) -> bool {
        (self.done)(&self.clone())
    }
}

impl UserData for TagSignalTester {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("log_active", |_, this, (id, active): (u32, bool)| {
            let handle = TagHandle::from_id(id);

            this.log_active(handle, active);

            Ok(())
        });

        methods.add_method("log_created", |_, this, id: u32| {
            let handle = TagHandle::from_id(id);

            this.log_created(handle);

            Ok(())
        });

        methods.add_method("log_removed", |_, this, id: u32| {
            let handle = TagHandle::from_id(id);

            this.log_removed(handle);

            Ok(())
        });

        methods.add_method("done", |_, this, ()| Ok(this.done()));
    }
}

#[test_log::test]
fn tag_signal_active() {
    for_each_api(|lang| {
        let (mut fixture, _o1, _o2, tags, ..) = set_up();

        let tag_name = tags[1].name();
        let handle = TagHandle::from_id(tags[1].id().to_inner());

        let tester = TagSignalTester::new(move |t| {
            let Ok(active) = t.active().try_lock() else {
                return false;
            };

            active.contains_key(&handle)
        });

        let tag1_hndl = TagHandle::from_id(tags[0].id().to_inner());
        let tag2_hndl = TagHandle::from_id(tags[1].id().to_inner());
        let tester_cpy = tester.clone();
        let signal_handle = Arc::new(OnceLock::new());
        let signal_handle_clone = signal_handle.clone();

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                let handle = pinnacle_api::tag::connect_signal(
                    pinnacle_api::signal::TagSignal::Active(Box::new(move |tag, active| {
                        tester.log_active(tag.clone(), active);
                    })),
                );
                signal_handle_clone.set(handle).unwrap();

                pinnacle_api::tag::get(tag_name).unwrap().switch_to();
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Tag.connect_signal({
                    active = function(tag, active)
                        $tester:log_active(tag.id, active)
                    end
                })

                Tag.get($tag_name):switch_to()

                local client = require("pinnacle.grpc.client").client
                while not $tester:done() do
                    client.loop:step();
                end
            },
        }

        fixture.dispatch_until(|_| tester_cpy.done());

        let store = tester_cpy.active().lock().unwrap();
        assert_eq!(store.get(&tag1_hndl), Some(&false));
        assert_eq!(store.get(&tag2_hndl), Some(&true));

        if lang == Lang::Rust {
            // Think the Rust client waits for signals forever, was only able
            // to reproduce in release mode though, strange
            signal_handle.get().unwrap().disconnect();
        }
    })
}

#[test_log::test]
fn tag_signal_created() {
    for_each_api(|lang| {
        let (mut fixture, output, ..) = set_up();

        let tag_name = "test_tag";
        let tester = TagSignalTester::new(move |t| {
            let Ok(created) = t.created().try_lock() else {
                return false;
            };

            !created.is_empty()
        });

        let tester_cpy = tester.clone();
        let signal_handle = Arc::new(OnceLock::new());
        let signal_handle_clone = signal_handle.clone();

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                let handle =
                    pinnacle_api::tag::connect_signal(TagSignal::Created(Box::new(move |tag| {
                        tester.log_created(tag.clone());
                    })));
                signal_handle_clone.set(handle).unwrap();

                let output = pinnacle_api::output::get_focused().unwrap();
                let _ = pinnacle_api::tag::add(&output, [tag_name]);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Tag.connect_signal({
                    created = function(tag)
                        $tester:log_created(tag.id)
                    end
                })

                local out = Output.get_focused()
                Tag.add(out, $tag_name)

                local client = require("pinnacle.grpc.client").client
                while not $tester:done() do
                    client.loop:step();
                end
            },
        }

        fixture.dispatch_until(|_| tester_cpy.done());

        let new_tag = output
            .with_state(|s| {
                s.tags.iter().find_map(|t| {
                    if t.name() == tag_name {
                        Some(TagHandle::from_id(t.id().to_inner()))
                    } else {
                        None
                    }
                })
            })
            .unwrap();

        let storage = tester_cpy.created().lock().unwrap();
        assert!(storage.contains(&new_tag));

        if lang == Lang::Rust {
            signal_handle.get().unwrap().disconnect();
        }
    });
}

#[test_log::test]
fn tag_signal_removed() {
    for_each_api(|lang| {
        let (mut fixture, _o1, _o2, tags, ..) = set_up();

        let tag_name = "2";
        let tag_handle = TagHandle::from_id(tags[1].id().to_inner());
        let tester = TagSignalTester::new(move |t| {
            let Ok(created) = t.removed().try_lock() else {
                return false;
            };

            !created.is_empty()
        });

        let tester_cpy = tester.clone();
        let signal_handle = Arc::new(OnceLock::new());
        let signal_handle_clone = signal_handle.clone();

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                let handle =
                    pinnacle_api::tag::connect_signal(TagSignal::Removed(Box::new(move |tag| {
                        tester.log_removed(tag.clone());
                    })));
                signal_handle_clone.set(handle).unwrap();

                let to_remove = pinnacle_api::tag::get(tag_name).unwrap();
                pinnacle_api::tag::remove([to_remove]);
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Tag.connect_signal({
                    removed = function(tag)
                        $tester:log_removed(tag.id)
                    end
                })

                local to_remove = Tag.get($tag_name);
                Tag.remove({to_remove})

                local client = require("pinnacle.grpc.client").client
                while not $tester:done() do
                    client.loop:step();
                end
            },
        }

        fixture.dispatch_until(|_| tester_cpy.done());

        let storage = tester_cpy.removed().lock().unwrap();
        assert!(storage.contains(&tag_handle));

        if lang == Lang::Rust {
            signal_handle.get().unwrap().disconnect();
        }
    });
}
