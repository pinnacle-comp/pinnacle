mod common;

use std::{
    io::Write,
    process::{Command, Stdio},
};

use crate::common::{sleep_secs, test_api, with_state};

use anyhow::anyhow;
use pinnacle::state::WithState;
use test_log::test;

fn run_lua(code: &str) -> anyhow::Result<()> {
    #[rustfmt::skip]
    let code = format!(r#"
        local Pinnacle = require("pinnacle")
        local Input = require("pinnacle.input")
        local Process = require("pinnacle.process")
        local Output = require("pinnacle.output")
        local Tag = require("pinnacle.tag")
        local Window = require("pinnacle.window")
        local Render = require("pinnacle.render")
        local Layout = require("pinnacle.layout")

        require("pinnacle").run(function()
            local run = function()
                {code}
            end

            local success, err = pcall(run)

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

    if exit_status.code().is_some_and(|code| code != 0) {
        return Err(anyhow!("lua code panicked"));
    }

    Ok(())
}

#[allow(dead_code)] // TODO:
struct SetupLuaGuard {
    child: std::process::Child,
}

impl Drop for SetupLuaGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[allow(dead_code)] // TODO:
fn setup_lua(code: &str) -> anyhow::Result<SetupLuaGuard> {
    #[rustfmt::skip]
    let code = format!(r#"
        require("pinnacle").setup(function()
            local run = function()
                {code}
            end

            local success, err = pcall(run)

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
    { $($body:tt)* } => {
        run_lua(stringify!($($body)*))?;
    };
}

#[allow(unused_macros)] // TODO:
macro_rules! setup_lua {
    { $($body:tt)* } => {
        let _guard = setup_lua(stringify!($($body)*))?;
    };
}

// Process

mod process {

    use super::*;

    #[self::test]
    fn spawn() -> anyhow::Result<()> {
        test_api(|sender| {
            run_lua! {
                Process.spawn("foot")
            }

            sleep_secs(3);

            with_state(&sender, |state| {
                assert_eq!(state.pinnacle.windows.len(), 1);
                assert_eq!(state.pinnacle.windows[0].class(), Some("foot".to_string()));
            });

            Ok(())
        })
    }
}

// Window

mod window {
    use super::*;

    #[self::test]
    fn get_all() -> anyhow::Result<()> {
        test_api(|_sender| {
            run_lua! {
                assert(#Window.get_all() == 0)

                for i = 1, 5 do
                    Process.spawn("foot")
                end
            }

            sleep_secs(1);

            run_lua! {
                assert(#Window.get_all() == 5)
            }

            Ok(())
        })
    }

    #[self::test]
    fn get_focused() -> anyhow::Result<()> {
        test_api(|_sender| {
            run_lua! {
                assert(not Window.get_focused())

                Tag.add(Output.get_focused(), "1")[1]:set_active(true)
                Process.spawn("foot")
            }

            sleep_secs(1);

            run_lua! {
                assert(Window.get_focused())
            }

            Ok(())
        })
    }

    mod handle {
        use super::*;

        // WindowHandle

        #[self::test]
        fn close() -> anyhow::Result<()> {
            test_api(|sender| {
                run_lua! {
                    Process.spawn("foot")
                }

                sleep_secs(1);

                with_state(&sender, |state| {
                    assert_eq!(state.pinnacle.windows.len(), 1);
                });

                run_lua! {
                    Window.get_all()[1]:close()
                }

                sleep_secs(1);

                with_state(&sender, |state| {
                    assert_eq!(state.pinnacle.windows.len(), 0);
                });

                Ok(())
            })
        }

        #[self::test]
        fn move_to_tag() -> anyhow::Result<()> {
            test_api(|sender| {
                run_lua! {
                    local tags = Tag.add(Output.get_focused(), "1", "2", "3")
                    tags[1]:set_active(true)
                    tags[2]:set_active(true)
                    Process.spawn("foot")
                }

                sleep_secs(1);

                with_state(&sender, |state| {
                    assert_eq!(
                        state.pinnacle.windows[0].with_state(|st| st
                            .tags
                            .iter()
                            .map(|tag| tag.name())
                            .collect::<Vec<_>>()),
                        vec!["1", "2"]
                    );
                });

                // Correct usage
                run_lua! {
                    Window.get_all()[1]:move_to_tag(Tag.get("3"))
                }

                sleep_secs(1);

                with_state(&sender, |state| {
                    assert_eq!(
                        state.pinnacle.windows[0].with_state(|st| st
                            .tags
                            .iter()
                            .map(|tag| tag.name())
                            .collect::<Vec<_>>()),
                        vec!["3"]
                    );
                });

                // Move to the same tag
                run_lua! {
                    Window.get_all()[1]:move_to_tag(Tag.get("3"))
                }

                sleep_secs(1);

                with_state(&sender, |state| {
                    assert_eq!(
                        state.pinnacle.windows[0].with_state(|st| st
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

        #[self::test]
        fn props() -> anyhow::Result<()> {
            test_api(|_sender| {
                run_lua! {
                    Output.for_each_output(function(op)
                        local tags = Tag.add(op, "First", "Mungus", "Potato")
                        tags[1]:set_active(true)
                        tags[3]:set_active(true)
                    end)
                }

                sleep_secs(1);

                run_lua! {
                    Process.spawn("foot")
                    Process.spawn("foot")
                }

                sleep_secs(1);

                run_lua! {
                    local first = Tag.get("First")
                    assert(first:active() == true)
                    assert(first:name() == "First")
                    assert(first:output().name == "Dummy Window")
                    assert(#first:windows() == 2)
                    assert(first:windows()[1]:app_id() == "foot")
                    assert(first:windows()[2]:app_id() == "foot")

                    local mungus = Tag.get("Mungus")
                    assert(mungus:active() == false)
                    assert(mungus:name() == "Mungus")
                    assert(mungus:output().name == "Dummy Window")
                    assert(#mungus:windows() == 0)

                    local potato = Tag.get("Potato")
                    assert(potato:active() == true)
                    assert(potato:name() == "Potato")
                    assert(potato:output().name == "Dummy Window")
                    assert(#potato:windows() == 2)
                    assert(potato:windows()[1]:app_id() == "foot")
                    assert(potato:windows()[2]:app_id() == "foot")
                }

                Ok(())
            })
        }
    }
}

mod output {
    use super::*;

    mod handle {
        use super::*;

        #[self::test]
        fn set_transform() -> anyhow::Result<()> {
            test_api(|sender| {
                run_lua! {
                    Output.get_focused():set_transform("flipped_90")
                }

                sleep_secs(1);

                with_state(&sender, |state| {
                    let op = state.pinnacle.focused_output().unwrap();
                    assert_eq!(op.current_transform(), smithay::utils::Transform::Flipped90);
                });

                run_lua! {
                    Output.get_focused():set_transform("normal")
                }

                sleep_secs(1);

                with_state(&sender, |state| {
                    let op = state.pinnacle.focused_output().unwrap();
                    assert_eq!(op.current_transform(), smithay::utils::Transform::Normal);
                });

                Ok(())
            })
        }

        #[self::test]
        fn set_powered() -> anyhow::Result<()> {
            test_api(|sender| {
                run_lua! {
                    Output.get_focused():set_powered(false)
                }

                sleep_secs(1);

                with_state(&sender, |state| {
                    let op = state.pinnacle.focused_output().unwrap();
                    assert!(!op.with_state(|state| state.powered))
                });

                run_lua! {
                    Output.get_focused():set_powered(true)
                }

                sleep_secs(1);

                with_state(&sender, |state| {
                    let op = state.pinnacle.focused_output().unwrap();
                    assert!(op.with_state(|state| state.powered))
                });

                Ok(())
            })
        }

        #[self::test]
        fn props() -> anyhow::Result<()> {
            test_api(|_sender| {
                run_lua! {
                    local op = Output.get_focused()

                    assert(op:make() == "Pinnacle")
                    assert(op:model() == "Dummy Window")
                    assert(op:loc().x == 0)
                    assert(op:loc().y == 0)
                    assert(op:logical_size().width == 1920)
                    assert(op:logical_size().height == 1080)
                    assert(op:current_mode().width == 1920)
                    assert(op:current_mode().height == 1080)
                    assert(op:current_mode().refresh_rate_mhz == 60000)
                    assert(op:preferred_mode().width == 1920)
                    assert(op:preferred_mode().height == 1080)
                    assert(op:preferred_mode().refresh_rate_mhz == 60000)
                    assert(op:physical_size().width == 0)
                    assert(op:physical_size().height == 0)
                    assert(op:focused() == true)
                    assert(op:scale() == 1.0)
                    assert(op:transform() == "flipped_180")
                }

                Ok(())
            })
        }

        // FIXME: nondeterministic on github CI
        // #[tokio::main]
        // #[self::test]
        // async fn keyboard_focus_stack() -> anyhow::Result<()> {
        //     test_api(|_sender| {
        //         run_lua! { |Pinnacle|
        //             Pinnacle.output.setup({
        //                 ["*"] = { tags = { "1", "2", "3" } },
        //             })
        //         }
        //
        //         sleep_secs(1);
        //
        //         run_lua! { |Pinnacle|
        //             Pinnacle.process.spawn("foot")
        //         }
        //         sleep_millis(250);
        //         run_lua! { |Pinnacle|
        //             Pinnacle.process.spawn("foot")
        //         }
        //         sleep_millis(250);
        //         run_lua! { |Pinnacle|
        //             Pinnacle.process.spawn("foot")
        //         }
        //
        //         sleep_millis(250);
        //
        //         run_lua! { |Pinnacle|
        //             Pinnacle.tag.get("2"):switch_to()
        //             Pinnacle.process.spawn("foot")
        //         }
        //         sleep_millis(250);
        //         run_lua! { |Pinnacle|
        //             Pinnacle.process.spawn("foot")
        //         }
        //
        //         sleep_secs(1);
        //
        //         run_lua! { |Pinnacle|
        //             Pinnacle.tag.get("1"):switch_to()
        //
        //             local focus_stack = Pinnacle.output.get_focused():keyboard_focus_stack()
        //             assert(#focus_stack == 5, "focus stack len != 5")
        //
        //             assert(focus_stack[1].id == 0, "focus stack at 1 id != 0")
        //             assert(focus_stack[2].id == 1, "focus stack at 2 id != 1")
        //             assert(focus_stack[3].id == 2, "focus stack at 3 id != 2")
        //             assert(focus_stack[4].id == 3, "focus stack at 4 id != 3")
        //             assert(focus_stack[5].id == 4, "focus stack at 5 id != 4")
        //
        //             local focus_stack = Pinnacle.output.get_focused():keyboard_focus_stack_visible()
        //             assert(#focus_stack == 3, "focus stack visible len != 3")
        //             assert(focus_stack[1].id == 0)
        //             assert(focus_stack[2].id == 1)
        //             assert(focus_stack[3].id == 2)
        //
        //             Pinnacle.tag.get("2"):switch_to()
        //
        //             local focus_stack = Pinnacle.output.get_focused():keyboard_focus_stack_visible()
        //             assert(#focus_stack == 2)
        //             assert(focus_stack[1].id == 3)
        //             assert(focus_stack[2].id == 4)
        //         }
        //
        //         Ok(())
        //     })
        // }
    }
}

#[test]
fn window_count_with_tag_is_correct() -> anyhow::Result<()> {
    test_api(|sender| {
        run_lua! {
            Tag.add(Output.get_focused(), "1")
            Process.spawn("foot")
        }

        sleep_secs(1);

        with_state(&sender, |state| assert_eq!(state.pinnacle.windows.len(), 1));

        run_lua! {
            for i = 1, 5 do
                Process.spawn("foot")
            end
        }

        sleep_secs(1);

        with_state(&sender, |state| assert_eq!(state.pinnacle.windows.len(), 6));

        Ok(())
    })
}

#[test]
fn window_count_without_tag_is_correct() -> anyhow::Result<()> {
    test_api(|sender| {
        run_lua! {
            Process.spawn("foot")
        }

        sleep_secs(1);

        with_state(&sender, |state| assert_eq!(state.pinnacle.windows.len(), 1));

        Ok(())
    })
}

#[test]
fn spawned_window_on_active_tag_has_keyboard_focus() -> anyhow::Result<()> {
    test_api(|sender| {
        run_lua! {
            Tag.add(Output.get_focused(), "1")[1]:set_active(true)
            Process.spawn("foot")
        }

        sleep_secs(1);

        with_state(&sender, |state| {
            assert_eq!(
                state
                    .pinnacle
                    .focused_window(state.pinnacle.focused_output().unwrap())
                    .unwrap()
                    .class(),
                Some("foot".to_string())
            );
        });

        Ok(())
    })
}

#[test]
fn spawned_window_on_inactive_tag_does_not_have_keyboard_focus() -> anyhow::Result<()> {
    test_api(|sender| {
        run_lua! {
            Tag.add(Output.get_focused(), "1")
            Process.spawn("foot")
        }

        sleep_secs(1);

        with_state(&sender, |state| {
            assert_eq!(
                state
                    .pinnacle
                    .focused_window(state.pinnacle.focused_output().unwrap()),
                None
            );
        });

        Ok(())
    })
}

#[test]
fn spawned_window_has_correct_tags() -> anyhow::Result<()> {
    test_api(|sender| {
        run_lua! {
            Tag.add(Output.get_focused(), "1", "2", "3")
            Process.spawn("foot")
        }

        sleep_secs(1);

        with_state(&sender, |state| {
            assert_eq!(state.pinnacle.windows.len(), 1);
            assert_eq!(state.pinnacle.windows[0].with_state(|st| st.tags.len()), 1);
        });

        run_lua! {
            Tag.get("1"):set_active(true)
            Tag.get("3"):set_active(true)
            Process.spawn("foot")
        }

        sleep_secs(1);

        with_state(&sender, |state| {
            assert_eq!(state.pinnacle.windows.len(), 2);
            assert_eq!(state.pinnacle.windows[1].with_state(|st| st.tags.len()), 2);
            assert_eq!(
                state.pinnacle.windows[1].with_state(|st| st
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
