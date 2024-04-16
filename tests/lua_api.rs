mod common;

use std::{
    io::Write,
    process::{Command, Stdio},
};

use crate::common::{sleep_secs, test_api, with_state};

use pinnacle::state::WithState;
use test_log::test;

fn run_lua(ident: &str, code: &str) {
    #[rustfmt::skip]
    let code = format!(r#"
        require("pinnacle").run(function({ident})
            local run = function({ident})
                {code}
            end

            local success, err = pcall(run, {ident})

            if not success then
                print(err)
                os.exit(1)
            end
        end)
    "#);

    let mut child = Command::new("lua").stdin(Stdio::piped()).spawn().unwrap();

    let mut stdin = child.stdin.take().unwrap();

    stdin.write_all(code.as_bytes()).unwrap();

    drop(stdin);

    let exit_status = child.wait().unwrap();

    if exit_status.code().is_some_and(|code| code != 0) {
        panic!("lua code panicked");
    }
}

struct SetupLuaGuard {
    child: std::process::Child,
}

impl Drop for SetupLuaGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[must_use]
fn setup_lua(ident: &str, code: &str) -> SetupLuaGuard {
    #[rustfmt::skip]
    let code = format!(r#"
        require("pinnacle").setup(function({ident})
            local run = function({ident})
                {code}
            end

            local success, err = pcall(run, {ident})

            if not success then
                print(err)
                os.exit(1)
            end
        end)
    "#);

    let mut child = Command::new("lua").stdin(Stdio::piped()).spawn().unwrap();

    let mut stdin = child.stdin.take().unwrap();

    stdin.write_all(code.as_bytes()).unwrap();

    drop(stdin);

    SetupLuaGuard { child }

    // let exit_status = child.wait().unwrap();
    //
    // if exit_status.code().is_some_and(|code| code != 0) {
    //     panic!("lua code panicked");
    // }
}

macro_rules! run_lua {
    { |$ident:ident| $($body:tt)* } => {
        run_lua(stringify!($ident), stringify!($($body)*));
    };
}

macro_rules! setup_lua {
    { |$ident:ident| $($body:tt)* } => {
        let _guard = setup_lua(stringify!($ident), stringify!($($body)*));
    };
}

mod coverage {
    use pinnacle::{
        tag::TagId,
        window::{
            rules::{WindowRule, WindowRuleCondition},
            window_state::FullscreenOrMaximized,
        },
    };

    use super::*;

    // Process

    mod process {

        use super::*;

        #[tokio::main]
        #[self::test]
        async fn spawn() -> anyhow::Result<()> {
            test_api(|sender| {
                run_lua! { |Pinnacle|
                    Pinnacle.process.spawn("foot")
                }

                sleep_secs(1);

                with_state(&sender, |state| {
                    assert_eq!(state.windows.len(), 1);
                    assert_eq!(state.windows[0].class(), Some("foot".to_string()));
                });
            })
        }

        #[tokio::main]
        #[self::test]
        async fn set_env() -> anyhow::Result<()> {
            test_api(|sender| {
                run_lua! { |Pinnacle|
                    Pinnacle.process.set_env("PROCESS_SET_ENV", "env value")
                }

                sleep_secs(1);

                with_state(&sender, |_state| {
                    assert_eq!(
                        std::env::var("PROCESS_SET_ENV"),
                        Ok("env value".to_string())
                    );
                });
            })
        }
    }

    // Window

    mod window {
        use super::*;

        #[tokio::main]
        #[self::test]
        async fn get_all() -> anyhow::Result<()> {
            test_api(|_sender| {
                run_lua! { |Pinnacle|
                    assert(#Pinnacle.window.get_all() == 0)

                    for i = 1, 5 do
                        Pinnacle.process.spawn("foot")
                    end
                }

                sleep_secs(1);

                run_lua! { |Pinnacle|
                    assert(#Pinnacle.window.get_all() == 5)
                }
            })
        }

        #[tokio::main]
        #[self::test]
        async fn get_focused() -> anyhow::Result<()> {
            test_api(|_sender| {
                run_lua! { |Pinnacle|
                    assert(not Pinnacle.window.get_focused())

                    Pinnacle.tag.add(Pinnacle.output.get_focused(), "1")[1]:set_active(true)
                    Pinnacle.process.spawn("foot")
                }

                sleep_secs(1);

                run_lua! { |Pinnacle|
                    assert(Pinnacle.window.get_focused())
                }
            })
        }

        #[tokio::main]
        #[self::test]
        async fn add_window_rule() -> anyhow::Result<()> {
            test_api(|sender| {
                run_lua! { |Pinnacle|
                    Pinnacle.tag.add(Pinnacle.output.get_focused(), "Tag Name")
                    Pinnacle.window.add_window_rule({
                        cond = { classes = { "firefox" } },
                        rule = { tags = { Pinnacle.tag.get("Tag Name") } },
                    })
                }

                sleep_secs(1);

                with_state(&sender, |state| {
                    assert_eq!(state.config.window_rules.len(), 1);
                    assert_eq!(
                        state.config.window_rules[0],
                        (
                            WindowRuleCondition {
                                class: Some(vec!["firefox".to_string()]),
                                ..Default::default()
                            },
                            WindowRule {
                                tags: Some(vec![TagId(0)]),
                                ..Default::default()
                            }
                        )
                    );
                });

                run_lua! { |Pinnacle|
                    Pinnacle.tag.add(Pinnacle.output.get_focused(), "Tag Name 2")
                    Pinnacle.window.add_window_rule({
                        cond = {
                            all = {
                                {
                                    classes = { "steam" },
                                    tags = {
                                        Pinnacle.tag.get("Tag Name"),
                                        Pinnacle.tag.get("Tag Name 2"),
                                    },
                                }
                            }
                        },
                        rule = { fullscreen_or_maximized = "fullscreen" },
                    })
                }

                sleep_secs(1);

                with_state(&sender, |state| {
                    assert_eq!(state.config.window_rules.len(), 2);
                    assert_eq!(
                        state.config.window_rules[1],
                        (
                            WindowRuleCondition {
                                cond_all: Some(vec![WindowRuleCondition {
                                    class: Some(vec!["steam".to_string()]),
                                    tag: Some(vec![TagId(0), TagId(1)]),
                                    ..Default::default()
                                }]),
                                ..Default::default()
                            },
                            WindowRule {
                                fullscreen_or_maximized: Some(FullscreenOrMaximized::Fullscreen),
                                ..Default::default()
                            }
                        )
                    );
                });
            })
        }

        // TODO: window_begin_move
        // TODO: window_begin_resize

        mod handle {
            use super::*;

            // WindowHandle

            #[tokio::main]
            #[self::test]
            async fn close() -> anyhow::Result<()> {
                test_api(|sender| {
                    run_lua! { |Pinnacle|
                        Pinnacle.process.spawn("foot")
                    }

                    sleep_secs(1);

                    with_state(&sender, |state| {
                        assert_eq!(state.windows.len(), 1);
                    });

                    run_lua! { |Pinnacle|
                        Pinnacle.window.get_all()[1]:close()
                    }

                    sleep_secs(1);

                    with_state(&sender, |state| {
                        assert_eq!(state.windows.len(), 0);
                    });
                })
            }

            #[tokio::main]
            #[self::test]
            async fn move_to_tag() -> anyhow::Result<()> {
                test_api(|sender| {
                    run_lua! { |Pinnacle|
                        local tags = Pinnacle.tag.add(Pinnacle.output.get_focused(), "1", "2", "3")
                        tags[1]:set_active(true)
                        tags[2]:set_active(true)
                        Pinnacle.process.spawn("foot")
                    }

                    sleep_secs(1);

                    with_state(&sender, |state| {
                        assert_eq!(
                            state.windows[0].with_state(|st| st
                                .tags
                                .iter()
                                .map(|tag| tag.name())
                                .collect::<Vec<_>>()),
                            vec!["1", "2"]
                        );
                    });

                    // Correct usage
                    run_lua! { |Pinnacle|
                        Pinnacle.window.get_all()[1]:move_to_tag(Pinnacle.tag.get("3"))
                    }

                    sleep_secs(1);

                    with_state(&sender, |state| {
                        assert_eq!(
                            state.windows[0].with_state(|st| st
                                .tags
                                .iter()
                                .map(|tag| tag.name())
                                .collect::<Vec<_>>()),
                            vec!["3"]
                        );
                    });

                    // Move to the same tag
                    run_lua! { |Pinnacle|
                        Pinnacle.window.get_all()[1]:move_to_tag(Pinnacle.tag.get("3"))
                    }

                    sleep_secs(1);

                    with_state(&sender, |state| {
                        assert_eq!(
                            state.windows[0].with_state(|st| st
                                .tags
                                .iter()
                                .map(|tag| tag.name())
                                .collect::<Vec<_>>()),
                            vec!["3"]
                        );
                    });
                })
            }
        }
    }

    mod tag {
        use super::*;

        mod handle {
            use super::*;

            #[tokio::main]
            #[self::test]
            async fn props() -> anyhow::Result<()> {
                test_api(|_sender| {
                    run_lua! { |Pinnacle|
                        Pinnacle.output.connect_for_all(function(op)
                            local tags = Pinnacle.tag.add(op, "First", "Mungus", "Potato")
                            tags[1]:set_active(true)
                            tags[3]:set_active(true)
                        end)
                    }

                    sleep_secs(1);

                    run_lua! { |Pinnacle|
                        Pinnacle.process.spawn("foot")
                        Pinnacle.process.spawn("foot")
                    }

                    sleep_secs(1);

                    run_lua! { |Pinnacle|
                        local first_props = Pinnacle.tag.get("First"):props()
                        assert(first_props.active == true)
                        assert(first_props.name == "First")
                        assert(first_props.output.name == "Pinnacle Window")
                        assert(#first_props.windows == 2)
                        assert(first_props.windows[1]:class() == "foot")
                        assert(first_props.windows[2]:class() == "foot")

                        local mungus_props = Pinnacle.tag.get("Mungus"):props()
                        assert(mungus_props.active == false)
                        assert(mungus_props.name == "Mungus")
                        assert(mungus_props.output.name == "Pinnacle Window")
                        assert(#mungus_props.windows == 0)

                        local potato_props = Pinnacle.tag.get("Potato"):props()
                        assert(potato_props.active == true)
                        assert(potato_props.name == "Potato")
                        assert(potato_props.output.name == "Pinnacle Window")
                        assert(#potato_props.windows == 2)
                        assert(first_props.windows[1]:class() == "foot")
                        assert(first_props.windows[2]:class() == "foot")
                    }
                })
            }
        }
    }

    mod output {
        use smithay::utils::Rectangle;

        use super::*;

        #[tokio::main]
        #[self::test]
        async fn setup() -> anyhow::Result<()> {
            test_api(|sender| {
                setup_lua! { |Pinnacle|
                    Pinnacle.output.setup({
                        {
                            function(_)
                                return true
                            end,
                            tag_names = { "First", "Third", "Schmurd" },
                        },
                        {
                            "Pinnacle Window",
                            loc = { x = 300, y = 0 },
                        },
                        {
                            "Output 1",
                            loc = { "Pinnacle Window", "bottom_align_left" },
                        }
                    })
                }

                sleep_secs(1);

                with_state(&sender, |state| {
                    state.new_output("Output 1", (960, 540).into());
                });

                sleep_secs(1);

                with_state(&sender, |state| {
                    let original_op = state
                        .space
                        .outputs()
                        .find(|op| op.name() == "Pinnacle Window")
                        .unwrap();
                    let output_1 = state
                        .space
                        .outputs()
                        .find(|op| op.name() == "Output 1")
                        .unwrap();

                    let original_op_geo = state.space.output_geometry(original_op).unwrap();
                    let output_1_geo = state.space.output_geometry(output_1).unwrap();

                    assert_eq!(
                        original_op_geo,
                        Rectangle::from_loc_and_size((300, 0), (1920, 1080))
                    );

                    assert_eq!(
                        output_1_geo,
                        Rectangle::from_loc_and_size((300, 1080), (960, 540))
                    );

                    assert_eq!(
                        output_1.with_state(|state| state
                            .tags
                            .iter()
                            .map(|tag| tag.name())
                            .collect::<Vec<_>>()),
                        vec!["First", "Third", "Schmurd"]
                    );

                    state.remove_output(&original_op.clone());
                });

                sleep_secs(1);

                with_state(&sender, |state| {
                    let output_1 = state
                        .space
                        .outputs()
                        .find(|op| op.name() == "Output 1")
                        .unwrap();

                    let output_1_geo = state.space.output_geometry(output_1).unwrap();

                    assert_eq!(
                        output_1_geo,
                        Rectangle::from_loc_and_size((0, 0), (960, 540))
                    );

                    state.new_output("Output 2", (300, 500).into());
                });

                sleep_secs(1);

                with_state(&sender, |state| {
                    let output_1 = state
                        .space
                        .outputs()
                        .find(|op| op.name() == "Output 1")
                        .unwrap();

                    let output_2 = state
                        .space
                        .outputs()
                        .find(|op| op.name() == "Output 2")
                        .unwrap();

                    let output_1_geo = state.space.output_geometry(output_1).unwrap();
                    let output_2_geo = state.space.output_geometry(output_2).unwrap();

                    assert_eq!(
                        output_2_geo,
                        Rectangle::from_loc_and_size((0, 0), (300, 500))
                    );

                    assert_eq!(
                        output_1_geo,
                        Rectangle::from_loc_and_size((300, 0), (960, 540))
                    );
                });
            })
        }
    }
}

#[tokio::main]
#[test]
async fn window_count_with_tag_is_correct() -> anyhow::Result<()> {
    test_api(|sender| {
        run_lua! { |Pinnacle|
            Pinnacle.tag.add(Pinnacle.output.get_focused(), "1")
            Pinnacle.process.spawn("foot")
        }

        sleep_secs(1);

        with_state(&sender, |state| assert_eq!(state.windows.len(), 1));

        run_lua! { |Pinnacle|
            for i = 1, 20 do
                Pinnacle.process.spawn("foot")
            end
        }

        sleep_secs(1);

        with_state(&sender, |state| assert_eq!(state.windows.len(), 21));
    })
}

#[tokio::main]
#[test]
async fn window_count_without_tag_is_correct() -> anyhow::Result<()> {
    test_api(|sender| {
        run_lua! { |Pinnacle|
            Pinnacle.process.spawn("foot")
        }

        sleep_secs(1);

        with_state(&sender, |state| assert_eq!(state.windows.len(), 1));
    })
}

#[tokio::main]
#[test]
async fn spawned_window_on_active_tag_has_keyboard_focus() -> anyhow::Result<()> {
    test_api(|sender| {
        run_lua! { |Pinnacle|
            Pinnacle.tag.add(Pinnacle.output.get_focused(), "1")[1]:set_active(true)
            Pinnacle.process.spawn("foot")
        }

        sleep_secs(1);

        with_state(&sender, |state| {
            assert_eq!(
                state
                    .focused_window(state.focused_output().unwrap())
                    .unwrap()
                    .class(),
                Some("foot".to_string())
            );
        });
    })
}

#[tokio::main]
#[test]
async fn spawned_window_on_inactive_tag_does_not_have_keyboard_focus() -> anyhow::Result<()> {
    test_api(|sender| {
        run_lua! { |Pinnacle|
            Pinnacle.tag.add(Pinnacle.output.get_focused(), "1")
            Pinnacle.process.spawn("foot")
        }

        sleep_secs(1);

        with_state(&sender, |state| {
            assert_eq!(state.focused_window(state.focused_output().unwrap()), None);
        });
    })
}

#[tokio::main]
#[test]
async fn spawned_window_has_correct_tags() -> anyhow::Result<()> {
    test_api(|sender| {
        run_lua! { |Pinnacle|
            Pinnacle.tag.add(Pinnacle.output.get_focused(), "1", "2", "3")
            Pinnacle.process.spawn("foot")
        }

        sleep_secs(1);

        with_state(&sender, |state| {
            assert_eq!(state.windows.len(), 1);
            assert_eq!(state.windows[0].with_state(|st| st.tags.len()), 1);
        });

        run_lua! { |Pinnacle|
            Pinnacle.tag.get("1"):set_active(true)
            Pinnacle.tag.get("3"):set_active(true)
            Pinnacle.process.spawn("foot")
        }

        sleep_secs(1);

        with_state(&sender, |state| {
            assert_eq!(state.windows.len(), 2);
            assert_eq!(state.windows[1].with_state(|st| st.tags.len()), 2);
            assert_eq!(
                state.windows[1].with_state(|st| st
                    .tags
                    .iter()
                    .map(|tag| tag.name())
                    .collect::<Vec<_>>()),
                vec!["1", "3"]
            );
        });
    })
}
