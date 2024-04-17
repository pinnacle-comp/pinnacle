mod common;

use std::thread::JoinHandle;

use pinnacle_api::ApiModules;
use test_log::test;

use crate::common::output_for_name;
use crate::common::{sleep_secs, test_api, with_state};

#[tokio::main]
async fn setup_rust_inner(run: impl FnOnce(ApiModules) + Send + 'static) {
    let (api, recv) = pinnacle_api::connect().await.unwrap();

    run(api.clone());

    pinnacle_api::listen(api, recv).await;
}

fn setup_rust(run: impl FnOnce(ApiModules) + Send + 'static) -> JoinHandle<()> {
    std::thread::spawn(|| {
        setup_rust_inner(run);
    })
}

mod output {
    use pinnacle::state::WithState;
    use pinnacle_api::output::{Alignment, OutputLoc, OutputSetup, UpdateLocsOn};
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
                    OutputSetup::new("Second").with_scale(2.0).with_mode(
                        pinnacle_api::output::Mode {
                            pixel_width: 6900,
                            pixel_height: 420,
                            refresh_rate_millihertz: 69420,
                        },
                    ),
                ]);
            });

            sleep_secs(1);

            with_state(&sender, |state| {
                state.new_output("First", (300, 200).into());
                state.new_output("Second", (300, 200).into());
                state.new_output("Test Third", (300, 200).into());
            });

            sleep_secs(1);

            with_state(&sender, |state| {
                let original_op = output_for_name(state, "Pinnacle Window");
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
            });
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
                        ("Pinnacle Window", OutputLoc::Point(0, 0)),
                        (
                            "First",
                            OutputLoc::relative_to("Second", Alignment::LeftAlignTop),
                        ),
                        (
                            "Second",
                            OutputLoc::relative_to("First", Alignment::RightAlignTop),
                        ),
                    ],
                );
            });

            sleep_secs(1);

            with_state(&sender, |state| {
                state.new_output("First", (300, 200).into());
            });

            sleep_secs(1);

            with_state(&sender, |state| {
                let original_op = output_for_name(state, "Pinnacle Window");
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
                let original_op = output_for_name(state, "Pinnacle Window");
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
                        ("Pinnacle Window", OutputLoc::Point(0, 0)),
                        (
                            "First",
                            OutputLoc::relative_to("Pinnacle Window", Alignment::BottomAlignLeft),
                        ),
                        (
                            "Second",
                            OutputLoc::relative_to("First", Alignment::BottomAlignLeft),
                        ),
                        (
                            "Third",
                            OutputLoc::relative_to_with_fallbacks(
                                "Second",
                                Alignment::BottomAlignLeft,
                                [("First", Alignment::BottomAlignLeft)],
                            ),
                        ),
                    ],
                );
            });

            sleep_secs(1);

            with_state(&sender, |state| {
                state.new_output("First", (300, 200).into());
                state.new_output("Second", (300, 700).into());
                state.new_output("Third", (300, 400).into());
            });

            sleep_secs(1);

            with_state(&sender, |state| {
                let original_op = output_for_name(state, "Pinnacle Window");
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
                let original_op = output_for_name(state, "Pinnacle Window");
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
        })
    }
}
