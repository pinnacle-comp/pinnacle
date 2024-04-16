mod common;

use std::thread::JoinHandle;

use pinnacle_api::ApiModules;
use test_log::test;

use crate::common::{sleep_secs, test_api, with_state};

#[tokio::main]
async fn setup_rust_inner(run: impl FnOnce(ApiModules) + Send + 'static) {
    let (api, recv) = pinnacle_api::connect().await.unwrap();

    run(api);

    pinnacle_api::listen(recv).await;
}

fn setup_rust(run: impl FnOnce(ApiModules) + Send + 'static) -> JoinHandle<()> {
    std::thread::spawn(|| {
        setup_rust_inner(run);
    })
}

mod output {
    use pinnacle_api::output::{Alignment, OutputMatcher, OutputSetup};
    use smithay::utils::Rectangle;

    use super::*;

    #[tokio::main]
    #[self::test]
    async fn setup() -> anyhow::Result<()> {
        test_api(|sender| {
            setup_rust(|api| {
                api.output
                    .setup([OutputSetup::new_with_matcher(|_| true).with_tags(["1", "2", "3"])]);
            });

            sleep_secs(1);

            with_state(&sender, |state| {
                state.new_output("First", (300, 200).into());
            });
        })
    }

    #[tokio::main]
    #[self::test]
    async fn setup_with_cyclic_relative_tos_work() -> anyhow::Result<()> {
        test_api(|sender| {
            setup_rust(|api| {
                api.output.setup([
                    OutputSetup::new("Pinnacle Window"),
                    OutputSetup::new("First").with_relative_loc(
                        OutputMatcher::Name("Second".into()),
                        Alignment::RightAlignTop,
                    ),
                    OutputSetup::new("Second").with_relative_loc(
                        OutputMatcher::Name("First".into()),
                        Alignment::LeftAlignTop,
                    ),
                ]);
            });

            sleep_secs(1);

            with_state(&sender, |state| {
                state.new_output("First", (300, 200).into());
            });

            sleep_secs(1);

            with_state(&sender, |state| {
                let original_op = state
                    .space
                    .outputs()
                    .find(|op| op.name() == "Pinnacle Window")
                    .unwrap();
                let first_op = state
                    .space
                    .outputs()
                    .find(|op| op.name() == "First")
                    .unwrap();

                let original_geo = state.space.output_geometry(original_op).unwrap();
                let first_geo = state.space.output_geometry(first_op).unwrap();

                assert_eq!(
                    original_geo,
                    Rectangle::from_loc_and_size((0, 0), (1920, 1080))
                );
                assert_eq!(
                    first_geo,
                    Rectangle::from_loc_and_size((1920, 0), (300, 200))
                );
            });
        })
    }
}
