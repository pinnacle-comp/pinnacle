mod common;

use std::thread::JoinHandle;

use anyhow::anyhow;
use test_log::test;

use crate::common::{sleep_secs, test_api, with_state};

#[tokio::main]
async fn run_rust_inner(run: impl FnOnce() + Send + 'static) {
    pinnacle_api::connect().await.unwrap();

    run();
}

fn run_rust(run: impl FnOnce() + Send + 'static) -> anyhow::Result<()> {
    std::thread::spawn(|| {
        run_rust_inner(run);
    })
    .join()
    .map_err(|_| anyhow!("rust oneshot api calls failed"))
}

#[tokio::main]
async fn setup_rust_inner(run: impl FnOnce() + Send + 'static) {
    pinnacle_api::connect().await.unwrap();

    run();

    pinnacle_api::block().await;
}

fn setup_rust(run: impl FnOnce() + Send + 'static) -> JoinHandle<()> {
    std::thread::spawn(|| {
        setup_rust_inner(run);
    })
}

mod output {
    use pinnacle::state::WithState;

    use super::*;

    mod handle {
        use common::sleep_millis;
        use pinnacle_api::output::Transform;

        use super::*;

        #[self::test]
        fn set_transform() -> anyhow::Result<()> {
            test_api(|sender| {
                run_rust(|| {
                    pinnacle_api::output::get_focused()
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

                run_rust(|| {
                    pinnacle_api::output::get_focused()
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

        #[self::test]
        fn set_powered() -> anyhow::Result<()> {
            test_api(|sender| {
                run_rust(|| {
                    pinnacle_api::output::get_focused()
                        .unwrap()
                        .set_powered(false);
                })?;

                sleep_secs(1);

                with_state(&sender, |state| {
                    let op = state.pinnacle.focused_output().unwrap();
                    assert!(op.with_state(|state| !state.powered));
                });

                run_rust(|| {
                    pinnacle_api::output::get_focused()
                        .unwrap()
                        .set_powered(true);
                })?;

                sleep_secs(1);

                with_state(&sender, |state| {
                    let op = state.pinnacle.focused_output().unwrap();
                    assert!(op.with_state(|state| state.powered));
                });

                Ok(())
            })
        }

        #[self::test]
        fn keyboard_focus_stack() -> anyhow::Result<()> {
            test_api(|_sender| {
                setup_rust(|| {
                    let output = pinnacle_api::output::get_focused().unwrap();
                    pinnacle_api::tag::add(&output, ["1", "2", "3"])
                        .next()
                        .unwrap()
                        .set_active(true);
                });

                sleep_secs(1);

                // FIXME: make a custom test client instead of using foot

                run_rust(|| {
                    pinnacle_api::process::Command::new("foot").spawn();
                })?;
                sleep_millis(250);
                run_rust(|| {
                    pinnacle_api::process::Command::new("foot").spawn();
                })?;
                sleep_millis(250);
                run_rust(|| {
                    pinnacle_api::process::Command::new("foot").spawn();
                })?;

                sleep_millis(250);

                run_rust(|| {
                    pinnacle_api::tag::get("2").unwrap().switch_to();
                    pinnacle_api::process::Command::new("foot").spawn();
                })?;
                sleep_millis(250);
                run_rust(|| {
                    pinnacle_api::process::Command::new("foot").spawn();
                })?;

                sleep_secs(1);

                run_rust(|| {
                    pinnacle_api::tag::get("1").unwrap().switch_to();

                    let focus_stack = pinnacle_api::output::get_focused()
                        .unwrap()
                        .keyboard_focus_stack();
                    assert_eq!(focus_stack.count(), 5);
                })?;

                Ok(())
            })
        }

        #[self::test]
        fn keyboard_focus_stack_visible() -> anyhow::Result<()> {
            test_api(|_sender| {
                setup_rust(|| {
                    let output = pinnacle_api::output::get_focused().unwrap();
                    pinnacle_api::tag::add(&output, ["1", "2", "3"])
                        .next()
                        .unwrap()
                        .set_active(true);
                });

                sleep_secs(1);

                // FIXME: make a custom test client instead of using foot

                run_rust(|| {
                    pinnacle_api::process::Command::new("foot").spawn();
                })?;
                sleep_millis(250);
                run_rust(|| {
                    pinnacle_api::process::Command::new("foot").spawn();
                })?;
                sleep_millis(250);
                run_rust(|| {
                    pinnacle_api::process::Command::new("foot").spawn();
                })?;

                sleep_millis(250);

                run_rust(|| {
                    pinnacle_api::tag::get("2").unwrap().switch_to();
                    pinnacle_api::process::Command::new("foot").spawn();
                })?;
                sleep_millis(250);
                run_rust(|| {
                    pinnacle_api::process::Command::new("foot").spawn();
                })?;

                sleep_secs(1);

                run_rust(|| {
                    pinnacle_api::tag::get("1").unwrap().switch_to();

                    let focus_stack = pinnacle_api::output::get_focused()
                        .unwrap()
                        .keyboard_focus_stack_visible();
                    assert_eq!(focus_stack.count(), 3);

                    pinnacle_api::tag::get("2").unwrap().switch_to();

                    let focus_stack = pinnacle_api::output::get_focused()
                        .unwrap()
                        .keyboard_focus_stack_visible();
                    assert_eq!(focus_stack.count(), 2);
                })?;

                Ok(())
            })
        }
    }
}
