mod common;

use std::thread::JoinHandle;

use anyhow::anyhow;
use pinnacle::backend::dummy::DUMMY_OUTPUT_NAME;
use pinnacle_api::ApiModules;
use test_log::test;

use crate::common::output_for_name;
use crate::common::{sleep_secs, test_api, with_state};

#[tokio::main]
async fn run_rust_inner(run: impl FnOnce(ApiModules) + Send + 'static) {
    pinnacle_api::connect().await.unwrap();

    run(ApiModules::new());
}

fn run_rust(run: impl FnOnce(ApiModules) + Send + 'static) -> anyhow::Result<()> {
    std::thread::spawn(|| {
        run_rust_inner(run);
    })
    .join()
    .map_err(|_| anyhow!("rust oneshot api calls failed"))
}

#[tokio::main]
async fn setup_rust_inner(run: impl FnOnce(ApiModules) + Send + 'static) {
    pinnacle_api::connect().await.unwrap();

    run(ApiModules::new());

    pinnacle_api::listen().await;
}

fn setup_rust(run: impl FnOnce(ApiModules) + Send + 'static) -> JoinHandle<()> {
    std::thread::spawn(|| {
        setup_rust_inner(run);
    })
}

mod output {
    use pinnacle::state::WithState;
    use pinnacle_api::output::{Alignment, OutputId, OutputLoc, OutputSetup, UpdateLocsOn};
    use smithay::{output::Output, utils::Rectangle};

    use super::*;

    #[tokio::main]
    #[self::test]
    async fn setup() -> anyhow::Result<()> {
        test_api(|sender| {
            setup_rust(|api| {
                api.output.setup([
                    OutputSetup::new_with_matcher(|_| true).with_tags(["1", "2", "3"]),
                    OutputSetup::new_with_matcher(|op| op.name().contains("Test"))
                        .with_tags(["Test 4", "Test 5"]),
                    OutputSetup::new(OutputId::name("Second"))
                        .with_scale(2.0)
                        .with_mode(pinnacle_api::output::Mode {
                            pixel_width: 6900,
                            pixel_height: 420,
                            refresh_rate_mhz: 69420,
                        })
                        .with_transform(pinnacle_api::output::Transform::_90),
                ]);
            });

            sleep_secs(1);

            with_state(&sender, |state| {
                state.pinnacle.new_output("First", (300, 200).into());
                state.pinnacle.new_output("Second", (300, 200).into());
                state.pinnacle.new_output("Test Third", (300, 200).into());
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
    async fn setup_loc_with_cyclic_relative_locs_works() -> anyhow::Result<()> {
        test_api(|sender| {
            setup_rust(|api| {
                api.output.setup_locs(
                    UpdateLocsOn::all(),
                    [
                        (OutputId::name(DUMMY_OUTPUT_NAME), OutputLoc::Point(0, 0)),
                        (
                            OutputId::name("First"),
                            OutputLoc::RelativeTo(
                                OutputId::name("Second"),
                                Alignment::LeftAlignTop,
                            ),
                        ),
                        (
                            OutputId::name("Second"),
                            OutputLoc::RelativeTo(
                                OutputId::name("First"),
                                Alignment::RightAlignTop,
                            ),
                        ),
                    ],
                );
            });

            sleep_secs(1);

            with_state(&sender, |state| {
                state.pinnacle.new_output("First", (300, 200).into());
            });

            sleep_secs(1);

            with_state(&sender, |state| {
                let original_op = output_for_name(state, DUMMY_OUTPUT_NAME);
                let first_op = output_for_name(state, "First");

                let original_geo = state.pinnacle.space.output_geometry(&original_op).unwrap();
                let first_geo = state.pinnacle.space.output_geometry(&first_op).unwrap();

                assert_eq!(
                    original_geo,
                    Rectangle::from_loc_and_size((0, 0), (1920, 1080))
                );
                assert_eq!(
                    first_geo,
                    Rectangle::from_loc_and_size((1920, 0), (300, 200))
                );

                state.pinnacle.new_output("Second", (500, 500).into());
            });

            sleep_secs(1);

            with_state(&sender, |state| {
                let original_op = output_for_name(state, DUMMY_OUTPUT_NAME);
                let first_op = output_for_name(state, "First");
                let second_op = output_for_name(state, "Second");

                let original_geo = state.pinnacle.space.output_geometry(&original_op).unwrap();
                let first_geo = state.pinnacle.space.output_geometry(&first_op).unwrap();
                let second_geo = state.pinnacle.space.output_geometry(&second_op).unwrap();

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
            setup_rust(|api| {
                api.output.setup_locs(
                    UpdateLocsOn::all(),
                    [
                        (OutputId::name(DUMMY_OUTPUT_NAME), OutputLoc::Point(0, 0)),
                        (
                            OutputId::name("First"),
                            OutputLoc::RelativeTo(
                                OutputId::name(DUMMY_OUTPUT_NAME),
                                Alignment::BottomAlignLeft,
                            ),
                        ),
                        (
                            OutputId::name("Second"),
                            OutputLoc::RelativeTo(
                                OutputId::name("First"),
                                Alignment::BottomAlignLeft,
                            ),
                        ),
                        (
                            OutputId::name("Third"),
                            OutputLoc::RelativeTo(
                                OutputId::name("Second"),
                                Alignment::BottomAlignLeft,
                            ),
                        ),
                        (
                            OutputId::name("Third"),
                            OutputLoc::RelativeTo(
                                OutputId::name("First"),
                                Alignment::BottomAlignLeft,
                            ),
                        ),
                    ],
                );
            });

            sleep_secs(1);

            with_state(&sender, |state| {
                state.pinnacle.new_output("First", (300, 200).into());
                state.pinnacle.new_output("Second", (300, 700).into());
                state.pinnacle.new_output("Third", (300, 400).into());
            });

            sleep_secs(1);

            with_state(&sender, |state| {
                let original_op = output_for_name(state, DUMMY_OUTPUT_NAME);
                let first_op = output_for_name(state, "First");
                let second_op = output_for_name(state, "Second");
                let third_op = output_for_name(state, "Third");

                let original_geo = state.pinnacle.space.output_geometry(&original_op).unwrap();
                let first_geo = state.pinnacle.space.output_geometry(&first_op).unwrap();
                let second_geo = state.pinnacle.space.output_geometry(&second_op).unwrap();
                let third_geo = state.pinnacle.space.output_geometry(&third_op).unwrap();

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

                state.pinnacle.remove_output(&second_op);
            });

            sleep_secs(1);

            with_state(&sender, |state| {
                let original_op = output_for_name(state, DUMMY_OUTPUT_NAME);
                let first_op = output_for_name(state, "First");
                let third_op = output_for_name(state, "Third");

                let original_geo = state.pinnacle.space.output_geometry(&original_op).unwrap();
                let first_geo = state.pinnacle.space.output_geometry(&first_op).unwrap();
                let third_geo = state.pinnacle.space.output_geometry(&third_op).unwrap();

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

    mod handle {
        use common::sleep_millis;
        use pinnacle::window::window_state::WindowId;
        use pinnacle_api::output::Transform;

        use super::*;

        #[tokio::main]
        #[self::test]
        async fn set_transform() -> anyhow::Result<()> {
            test_api(|sender| {
                run_rust(|api| {
                    api.output
                        .get_focused()
                        .unwrap()
                        .set_transform(Transform::Flipped270);
                })?;

                sleep_secs(1);

                with_state(&sender, |state| {
                    let op = state.pinnacle.focused_output().unwrap();
                    assert_eq!(
                        op.current_transform(),
                        smithay::utils::Transform::Flipped270
                    );
                });

                run_rust(|api| {
                    api.output
                        .get_focused()
                        .unwrap()
                        .set_transform(Transform::_180);
                })?;

                sleep_secs(1);

                with_state(&sender, |state| {
                    let op = state.pinnacle.focused_output().unwrap();
                    assert_eq!(op.current_transform(), smithay::utils::Transform::_180);
                });

                Ok(())
            })
        }

        #[tokio::main]
        #[self::test]
        async fn set_powered() -> anyhow::Result<()> {
            test_api(|sender| {
                run_rust(|api| {
                    api.output.get_focused().unwrap().set_powered(false);
                })?;

                sleep_secs(1);

                with_state(&sender, |state| {
                    let op = state.pinnacle.focused_output().unwrap();
                    assert!(!op.with_state(|state| state.powered));
                });

                run_rust(|api| {
                    api.output.get_focused().unwrap().set_powered(true);
                })?;

                sleep_secs(1);

                with_state(&sender, |state| {
                    let op = state.pinnacle.focused_output().unwrap();
                    assert!(op.with_state(|state| state.powered));
                });

                Ok(())
            })
        }

        #[tokio::main]
        #[self::test]
        async fn keyboard_focus_stack() -> anyhow::Result<()> {
            test_api(|_sender| {
                setup_rust(|api| {
                    api.output.setup([
                        OutputSetup::new_with_matcher(|_| true).with_tags(["1", "2", "3"])
                    ]);
                });

                sleep_secs(1);

                run_rust(|api| {
                    api.process.spawn(["foot"]);
                })?;
                sleep_millis(250);
                run_rust(|api| {
                    api.process.spawn(["foot"]);
                })?;
                sleep_millis(250);
                run_rust(|api| {
                    api.process.spawn(["foot"]);
                })?;

                sleep_millis(250);

                run_rust(|api| {
                    api.tag.get("2").unwrap().switch_to();
                    api.process.spawn(["foot"]);
                })?;
                sleep_millis(250);
                run_rust(|api| {
                    api.process.spawn(["foot"]);
                })?;

                sleep_secs(1);

                run_rust(|api| {
                    api.tag.get("1").unwrap().switch_to();

                    let focus_stack = api.output.get_focused().unwrap().keyboard_focus_stack();
                    assert_eq!(focus_stack.len(), 5);
                    assert_eq!(focus_stack[0].id(), 0);
                    assert_eq!(focus_stack[1].id(), 1);
                    assert_eq!(focus_stack[2].id(), 2);
                    assert_eq!(focus_stack[3].id(), 3);
                    assert_eq!(focus_stack[4].id(), 4);
                })?;

                // Terminate all windows related to this test
                run_rust(|api| {
                    api.tag.get("1").unwrap().switch_to();

                    let focus_stack = api.output.get_focused().unwrap().keyboard_focus_stack();
                    focus_stack[0].close();
                    focus_stack[1].close();
                    focus_stack[2].close();
                    focus_stack[3].close();
                    focus_stack[4].close();

                    api.tag.remove(api.tag.get_all());

                    WindowId::reset();
                })?;

                Ok(())
            })
        }

        #[tokio::main]
        #[self::test]
        async fn keyboard_focus_stack_visible() -> anyhow::Result<()> {
            test_api(|_sender| {
                setup_rust(|api| {
                    api.output.setup([
                        OutputSetup::new_with_matcher(|_| true).with_tags(["1", "2", "3"])
                    ]);
                });

                sleep_secs(1);

                run_rust(|api| {
                    api.process.spawn(["foot"]);
                })?;
                sleep_millis(250);
                run_rust(|api| {
                    api.process.spawn(["foot"]);
                })?;
                sleep_millis(250);
                run_rust(|api| {
                    api.process.spawn(["foot"]);
                })?;

                sleep_millis(250);

                run_rust(|api| {
                    api.tag.get("2").unwrap().switch_to();
                    api.process.spawn(["foot"]);
                })?;
                sleep_millis(250);
                run_rust(|api| {
                    api.process.spawn(["foot"]);
                })?;

                sleep_secs(1);

                run_rust(|api| {
                    api.tag.get("1").unwrap().switch_to();

                    let focus_stack = api
                        .output
                        .get_focused()
                        .unwrap()
                        .keyboard_focus_stack_visible();
                    assert_eq!(focus_stack.len(), 3);
                    assert_eq!(focus_stack[0].id(), 0);
                    assert_eq!(focus_stack[1].id(), 1);
                    assert_eq!(focus_stack[2].id(), 2);

                    api.tag.get("2").unwrap().switch_to();

                    let focus_stack = api
                        .output
                        .get_focused()
                        .unwrap()
                        .keyboard_focus_stack_visible();
                    assert_eq!(focus_stack.len(), 2);
                    assert_eq!(focus_stack[0].id(), 3);
                    assert_eq!(focus_stack[1].id(), 4);
                })?;

                // Terminate all windows related to this test
                run_rust(|api| {
                    api.tag.get("1").unwrap().switch_to();

                    let focus_stack = api.output.get_focused().unwrap().keyboard_focus_stack();
                    focus_stack[0].close();
                    focus_stack[1].close();
                    focus_stack[2].close();
                    focus_stack[3].close();
                    focus_stack[4].close();

                    api.tag.remove(api.tag.get_all());

                    WindowId::reset();
                })?;

                Ok(())
            })
        }
    }
}
