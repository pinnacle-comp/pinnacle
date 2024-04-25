mod common;

use std::{
    io::Write,
    process::{Command, Stdio},
};

use crate::common::{output_for_name, sleep_secs, test_api, with_state};

use anyhow::anyhow;
use pinnacle::backend::dummy::DUMMY_OUTPUT_NAME;
use pinnacle::state::WithState;
use test_log::test;

fn run_lua(ident: &str, code: &str) -> anyhow::Result<()> {
    #[rustfmt::skip]
    let code = format!(r#"
        require("pinnacle").run(function({ident})
            local run = function({ident})
                {code}
            end

            local success, err = pcall(run, {ident})

            if not success then
                print(err)
                print("exiting")
                os.exit(1)
            end
        end)
    "#);

    let mut child = Command::new("lua").stdin(Stdio::piped()).spawn()?;

    let mut stdin = child.stdin.take().ok_or(anyhow!("child had no stdin"))?;

    stdin.write_all(code.as_bytes())?;

    drop(stdin);

    let exit_status = child.wait()?;
    println!("exit status is {exit_status:?}");

    if exit_status.code().is_some_and(|code| code != 0) {
        return Err(anyhow!("lua code panicked"));
    }

    Ok(())
}

struct SetupLuaGuard {
    child: std::process::Child,
}

impl Drop for SetupLuaGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn setup_lua(ident: &str, code: &str) -> anyhow::Result<SetupLuaGuard> {
    #[rustfmt::skip]
    let code = format!(r#"
        require("pinnacle").setup(function({ident})
            local run = function({ident})
                {code}
            end

            local success, err = pcall(run, {ident})

            if not success then
                print(err)
                print("exiting")
                os.exit(1)
            end
        end)
    "#);

    let mut child = Command::new("lua").stdin(Stdio::piped()).spawn()?;

    let mut stdin = child.stdin.take().ok_or(anyhow!("child had no stdin"))?;

    stdin.write_all(code.as_bytes())?;

    drop(stdin);

    Ok(SetupLuaGuard { child })
}

macro_rules! run_lua {
    { |$ident:ident| $($body:tt)* } => {
        run_lua(stringify!($ident), stringify!($($body)*))?;
    };
}

macro_rules! setup_lua {
    { |$ident:ident| $($body:tt)* } => {
        let _guard = setup_lua(stringify!($ident), stringify!($($body)*))?;
    };
}

use pinnacle::{
    tag::TagId,
    window::{
        rules::{WindowRule, WindowRuleCondition},
        window_state::FullscreenOrMaximized,
    },
};

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

            Ok(())
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

            Ok(())
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

            Ok(())
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

            Ok(())
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

            Ok(())
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

                Ok(())
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

                Ok(())
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
                    assert(first_props.output.name == "Dummy Window")
                    assert(#first_props.windows == 2)
                    assert(first_props.windows[1]:class() == "foot")
                    assert(first_props.windows[2]:class() == "foot")

                    local mungus_props = Pinnacle.tag.get("Mungus"):props()
                    assert(mungus_props.active == false)
                    assert(mungus_props.name == "Mungus")
                    assert(mungus_props.output.name == "Dummy Window")
                    assert(#mungus_props.windows == 0)

                    local potato_props = Pinnacle.tag.get("Potato"):props()
                    assert(potato_props.active == true)
                    assert(potato_props.name == "Potato")
                    assert(potato_props.output.name == "Dummy Window")
                    assert(#potato_props.windows == 2)
                    assert(potato_props.windows[1]:class() == "foot")
                    assert(potato_props.windows[2]:class() == "foot")
                }

                Ok(())
            })
        }
    }
}

mod output {
    use smithay::{output::Output, utils::Rectangle};

    use super::*;

    mod handle {
        use super::*;

        #[tokio::main]
        #[self::test]
        async fn set_transform() -> anyhow::Result<()> {
            test_api(|sender| {
                run_lua! { |Pinnacle|
                    Pinnacle.output.get_focused():set_transform("flipped_90")
                }

                sleep_secs(1);

                with_state(&sender, |state| {
                    let op = state.focused_output().unwrap();
                    assert_eq!(op.current_transform(), smithay::utils::Transform::Flipped90);
                });

                run_lua! { |Pinnacle|
                    Pinnacle.output.get_focused():set_transform("normal")
                }

                sleep_secs(1);

                with_state(&sender, |state| {
                    let op = state.focused_output().unwrap();
                    assert_eq!(op.current_transform(), smithay::utils::Transform::Normal);
                });

                Ok(())
            })
        }

        #[tokio::main]
        #[self::test]
        async fn props() -> anyhow::Result<()> {
            test_api(|_sender| {
                run_lua! { |Pinnacle|
                    local props = Pinnacle.output.get_focused():props()

                    assert(props.make == "Pinnacle")
                    assert(props.model == "Dummy Window")
                    assert(props.x == 0)
                    assert(props.y == 0)
                    assert(props.logical_width == 1920)
                    assert(props.logical_height == 1080)
                    assert(props.current_mode.pixel_width == 1920)
                    assert(props.current_mode.pixel_height == 1080)
                    assert(props.current_mode.refresh_rate_millihz == 60000)
                    assert(props.preferred_mode.pixel_width == 1920)
                    assert(props.preferred_mode.pixel_height == 1080)
                    assert(props.preferred_mode.refresh_rate_millihz == 60000)
                    -- modes
                    assert(props.physical_width == 0)
                    assert(props.physical_height == 0)
                    assert(props.focused == true)
                    -- tags
                    assert(props.scale == 1.0)
                    assert(props.transform == "flipped_180")
                }

                Ok(())
            })
        }
    }

    #[tokio::main]
    #[self::test]
    async fn setup() -> anyhow::Result<()> {
        test_api(|sender| {
            setup_lua! { |Pinnacle|
                Pinnacle.output.setup({
                    ["1:*"] = {
                        tags = { "1", "2", "3" },
                    },
                    ["2:*"] = {
                        filter = function(op)
                            return string.match(op.name, "Test") ~= nil
                        end,
                        tags = { "Test 4", "Test 5" },
                    },
                    ["Second"] = {
                        scale = 2.0,
                        mode = {
                            pixel_width = 6900,
                            pixel_height = 420,
                            refresh_rate_millihz = 69420,
                        },
                        transform = "90",
                    },
                })
            }

            sleep_secs(1);

            with_state(&sender, |state| {
                state.new_output("First", (300, 200).into());
                state.new_output("Second", (300, 200).into());
                state.new_output("Test Third", (300, 200).into());
            });

            sleep_secs(1);

            with_state(&sender, |state| {
                let original_op = output_for_name(state, DUMMY_OUTPUT_NAME);
                let first_op = output_for_name(state, "First");
                let second_op = output_for_name(state, "Second");
                let test_third_op = output_for_name(state, "Test Third");

                let tags_for = |output: &Output| {
                    output
                        .with_state(|state| state.tags.iter().map(|t| t.name()).collect::<Vec<_>>())
                };

                let focused_tags_for = |output: &Output| {
                    output.with_state(|state| {
                        state.focused_tags().map(|t| t.name()).collect::<Vec<_>>()
                    })
                };

                assert_eq!(tags_for(&original_op), vec!["1", "2", "3"]);
                assert_eq!(tags_for(&first_op), vec!["1", "2", "3"]);
                assert_eq!(tags_for(&second_op), vec!["1", "2", "3"]);
                assert_eq!(
                    tags_for(&test_third_op),
                    vec!["1", "2", "3", "Test 4", "Test 5"]
                );

                assert_eq!(focused_tags_for(&original_op), vec!["1"]);
                assert_eq!(focused_tags_for(&test_third_op), vec!["1"]);

                assert_eq!(second_op.current_scale().fractional_scale(), 2.0);

                let second_mode = second_op.current_mode().unwrap();
                assert_eq!(second_mode.size.w, 6900);
                assert_eq!(second_mode.size.h, 420);
                assert_eq!(second_mode.refresh, 69420);

                assert_eq!(
                    second_op.current_transform(),
                    smithay::utils::Transform::_90
                );
            });

            Ok(())
        })
    }

    #[tokio::main]
    #[self::test]
    async fn setup_has_wildcard_first() -> anyhow::Result<()> {
        test_api(|sender| {
            setup_lua! { |Pinnacle|
                Pinnacle.output.setup({
                    ["*"] = {
                        tags = { "1", "2", "3" },
                    },
                    ["First"] = {
                        tags = { "A", "B" },
                    },
                })
            }

            sleep_secs(1);

            with_state(&sender, |state| {
                state.new_output("First", (300, 200).into());
            });

            sleep_secs(1);

            with_state(&sender, |state| {
                let first_op = output_for_name(state, "First");

                let tags_for = |output: &Output| {
                    output
                        .with_state(|state| state.tags.iter().map(|t| t.name()).collect::<Vec<_>>())
                };

                assert_eq!(tags_for(&first_op), vec!["1", "2", "3", "A", "B"]);
            });

            Ok(())
        })
    }

    #[tokio::main]
    #[self::test]
    async fn setup_loc_with_cyclic_relative_locs_works() -> anyhow::Result<()> {
        test_api(|sender| {
            setup_lua! { |Pinnacle|
                Pinnacle.output.setup_locs("all", {
                    ["Dummy Window"] = { x = 0, y = 0 },
                    ["First"] = { "Second", "left_align_top" },
                    ["Second"] = { "First", "right_align_top" },
                })
            }

            sleep_secs(1);

            with_state(&sender, |state| {
                state.new_output("First", (300, 200).into());
            });

            sleep_secs(1);

            with_state(&sender, |state| {
                let original_op = output_for_name(state, DUMMY_OUTPUT_NAME);
                let first_op = output_for_name(state, "First");

                let original_geo = state.space.output_geometry(&original_op).unwrap();
                let first_geo = state.space.output_geometry(&first_op).unwrap();

                assert_eq!(
                    original_geo,
                    Rectangle::from_loc_and_size((0, 0), (1920, 1080))
                );
                assert_eq!(
                    first_geo,
                    Rectangle::from_loc_and_size((1920, 0), (300, 200))
                );

                state.new_output("Second", (500, 500).into());
            });

            sleep_secs(1);

            with_state(&sender, |state| {
                let original_op = output_for_name(state, DUMMY_OUTPUT_NAME);
                let first_op = output_for_name(state, "First");
                let second_op = output_for_name(state, "Second");

                let original_geo = state.space.output_geometry(&original_op).unwrap();
                let first_geo = state.space.output_geometry(&first_op).unwrap();
                let second_geo = state.space.output_geometry(&second_op).unwrap();

                assert_eq!(
                    original_geo,
                    Rectangle::from_loc_and_size((0, 0), (1920, 1080))
                );
                assert_eq!(
                    first_geo,
                    Rectangle::from_loc_and_size((1920, 0), (300, 200))
                );
                assert_eq!(
                    second_geo,
                    Rectangle::from_loc_and_size((1920 + 300, 0), (500, 500))
                );
            });

            Ok(())
        })
    }

    #[tokio::main]
    #[self::test]
    async fn setup_loc_with_relative_locs_with_more_than_one_relative_works() -> anyhow::Result<()>
    {
        test_api(|sender| {
            setup_lua! { |Pinnacle|
                Pinnacle.output.setup_locs("all", {
                    ["Dummy Window"] = { 0, 0 },
                    ["First"] = { "Dummy Window", "bottom_align_left" },
                    ["Second"] = { "First", "bottom_align_left" },
                    ["4:Third"] = { "Second", "bottom_align_left" },
                    ["5:Third"] = { "First", "bottom_align_left" },
                })
            }

            sleep_secs(1);

            with_state(&sender, |state| {
                state.new_output("First", (300, 200).into());
                state.new_output("Second", (300, 700).into());
                state.new_output("Third", (300, 400).into());
            });

            sleep_secs(1);

            with_state(&sender, |state| {
                let original_op = output_for_name(state, DUMMY_OUTPUT_NAME);
                let first_op = output_for_name(state, "First");
                let second_op = output_for_name(state, "Second");
                let third_op = output_for_name(state, "Third");

                let original_geo = state.space.output_geometry(&original_op).unwrap();
                let first_geo = state.space.output_geometry(&first_op).unwrap();
                let second_geo = state.space.output_geometry(&second_op).unwrap();
                let third_geo = state.space.output_geometry(&third_op).unwrap();

                assert_eq!(
                    original_geo,
                    Rectangle::from_loc_and_size((0, 0), (1920, 1080))
                );
                assert_eq!(
                    first_geo,
                    Rectangle::from_loc_and_size((0, 1080), (300, 200))
                );
                assert_eq!(
                    second_geo,
                    Rectangle::from_loc_and_size((0, 1080 + 200), (300, 700))
                );
                assert_eq!(
                    third_geo,
                    Rectangle::from_loc_and_size((0, 1080 + 200 + 700), (300, 400))
                );

                state.remove_output(&second_op);
            });

            sleep_secs(1);

            with_state(&sender, |state| {
                let original_op = output_for_name(state, DUMMY_OUTPUT_NAME);
                let first_op = output_for_name(state, "First");
                let third_op = output_for_name(state, "Third");

                let original_geo = state.space.output_geometry(&original_op).unwrap();
                let first_geo = state.space.output_geometry(&first_op).unwrap();
                let third_geo = state.space.output_geometry(&third_op).unwrap();

                assert_eq!(
                    original_geo,
                    Rectangle::from_loc_and_size((0, 0), (1920, 1080))
                );
                assert_eq!(
                    first_geo,
                    Rectangle::from_loc_and_size((0, 1080), (300, 200))
                );
                assert_eq!(
                    third_geo,
                    Rectangle::from_loc_and_size((0, 1080 + 200), (300, 400))
                );
            });

            Ok(())
        })
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

        Ok(())
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

        Ok(())
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

        Ok(())
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

        Ok(())
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

        Ok(())
    })
}
