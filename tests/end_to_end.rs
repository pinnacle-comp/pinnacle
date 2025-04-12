mod common;

use std::{
    fs::File,
    io::{Read, Write},
    sync::{Arc, OnceLock},
    thread::sleep,
    time::Duration,
};

use common::{
    rust::run_rust, test_api, Lang, PINNACLE_1_OUTPUT_MAKE, PINNACLE_1_OUTPUT_MODEL,
    PINNACLE_1_OUTPUT_NAME, PINNACLE_1_OUTPUT_REFRESH, PINNACLE_1_OUTPUT_SIZE,
};
use pinnacle::{output::OutputName, state::WithState, window::window_state::LayoutModeKind};
use pinnacle_api::input::Bind as _;
use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1;
use test_log::test;

const SLEEP_DURATION: Duration = Duration::from_millis(1000);

// PINNACLE //////////////////////////////////////

#[test]
fn pinnacle_set_last_error() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    Pinnacle.set_last_error("wibbly wobbly timey wimey")
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::pinnacle::set_last_error("wibbly wobbly timey wimey");
            }),
        }?;

        sender.with_state(|state| {
            assert_eq!(
                state.pinnacle.config.last_error.as_deref(),
                Some("wibbly wobbly timey wimey")
            )
        });

        Ok(())
    })
}

#[test]
fn pinnacle_take_last_error() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        sender.with_state(|state| {
            state.pinnacle.config.last_error = Some("i've never watched doctor who".into());
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    local error = Pinnacle.take_last_error()
                    assert(error == "i've never watched doctor who")

                    local error = Pinnacle.take_last_error()
                    assert(error == nil)
                }
            }
            Lang::Rust => run_rust(|| {
                let error = pinnacle_api::pinnacle::take_last_error();
                assert_eq!(error.as_deref(), Some("i've never watched doctor who"));

                let error = pinnacle_api::pinnacle::take_last_error();
                assert_eq!(error.as_deref(), None);
            }),
        }?;

        Ok(())
    })
}

// OUTPUTS ///////////////////////////////////////

#[test]
fn output_get_all() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        sender.with_state(|state| {
            state.pinnacle.new_output(
                "pinnacle-2",
                "",
                "",
                (10000, 10000).into(),
                (2560, 1440).into(),
                144000,
                2.0,
                smithay::utils::Transform::Normal,
            );
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    local outputs = Output.get_all()
                    assert(#outputs == 2)
                    assert(outputs[1].name == "pinnacle-1")
                    assert(outputs[2].name == "pinnacle-2")
                }
            }
            Lang::Rust => run_rust(|| {
                let outputs = pinnacle_api::output::get_all().collect::<Vec<_>>();
                assert_eq!(outputs.len(), 2);
                assert_eq!(outputs[0].name(), "pinnacle-1");
                assert_eq!(outputs[1].name(), "pinnacle-2");
            }),
        }?;

        Ok(())
    })
}

#[test]
fn output_get_all_enabled() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        sender.with_state(|state| {
            let new_output = state.pinnacle.new_output(
                "pinnacle-2",
                "",
                "",
                (10000, 10000).into(),
                (2560, 1440).into(),
                144000,
                2.0,
                smithay::utils::Transform::Normal,
            );

            state.pinnacle.set_output_enabled(&new_output, false);
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    local outputs = Output.get_all_enabled()
                    assert(#outputs == 1)
                    assert(outputs[1].name == "pinnacle-1")
                }
            }
            Lang::Rust => run_rust(|| {
                let outputs = pinnacle_api::output::get_all_enabled().collect::<Vec<_>>();
                assert_eq!(outputs.len(), 1);
                assert_eq!(outputs[0].name(), "pinnacle-1");
            }),
        }?;

        Ok(())
    })
}

#[test]
fn output_get_by_name() -> anyhow::Result<()> {
    test_api(|_sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    local output = Output.get_by_name($PINNACLE_1_OUTPUT_NAME)
                    assert(output.name == $PINNACLE_1_OUTPUT_NAME)
                }
            }
            Lang::Rust => run_rust(|| {
                let output = pinnacle_api::output::get_by_name(PINNACLE_1_OUTPUT_NAME).unwrap();
                assert_eq!(output.name(), PINNACLE_1_OUTPUT_NAME);
            }),
        }?;

        Ok(())
    })
}

#[test]
fn output_get_focused() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        sender.with_state(|state| {
            let new_output = state.pinnacle.new_output(
                "pinnacle-2",
                "",
                "",
                (10000, 10000).into(),
                (2560, 1440).into(),
                144000,
                2.0,
                smithay::utils::Transform::Normal,
            );

            state.pinnacle.set_output_enabled(&new_output, false);
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    local output = Output.get_focused()
                    assert(output.name == $PINNACLE_1_OUTPUT_NAME)
                }
            }
            Lang::Rust => run_rust(|| {
                let output = pinnacle_api::output::get_focused().unwrap();
                assert_eq!(output.name(), PINNACLE_1_OUTPUT_NAME);
            }),
        }?;

        Ok(())
    })
}

// TODO: for_each_output
// TODO: connect_signal

#[test]
fn output_handle_set_loc() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    Output.get_focused():set_loc(500, -250)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .set_loc(500, -250);
            }),
        }?;

        sender.with_state(|state| {
            let op = state.pinnacle.focused_output().unwrap();
            let loc = op.current_location();
            assert_eq!(loc, (500, -250).into());
        });

        Ok(())
    })
}

#[test]
fn output_handle_set_loc_adj_to() -> anyhow::Result<()> {
    // TODO: fuzz/proptest/whatever this

    test_api(|sender, lang| {
        sender.with_state(|state| {
            state.pinnacle.new_output(
                "pinnacle-2",
                "",
                "",
                (10000, 10000).into(),
                (2560, 1440).into(),
                144000,
                2.0,
                smithay::utils::Transform::Normal,
            );
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    local pinnacle1 = Output.get_by_name("pinnacle-1")
                    local pinnacle2 = Output.get_focused()
                    pinnacle1:set_loc_adj_to(pinnacle2, "right_align_center")
                }
            }
            Lang::Rust => run_rust(|| {
                let pinnacle1 = pinnacle_api::output::get_by_name("pinnacle-1").unwrap();
                let pinnacle2 = pinnacle_api::output::get_focused().unwrap();
                pinnacle1.set_loc_adj_to(
                    &pinnacle2,
                    pinnacle_api::output::Alignment::RightAlignCenter,
                );
            }),
        }?;

        sender.with_state(|state| {
            let pinnacle1 = OutputName("pinnacle-1".into())
                .output(&state.pinnacle)
                .unwrap();
            let loc = pinnacle1.current_location();
            let space_loc = state
                .pinnacle
                .space
                .output_geometry(&pinnacle1)
                .unwrap()
                .loc;
            assert_eq!(loc, space_loc);
            assert_eq!(
                loc,
                (10000 + 2560 / 2, 10000 - (1080 - 1440 / 2) / 2).into()
            );
        });

        Ok(())
    })
}

#[test]
fn output_handle_set_mode() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    Output.get_focused():set_mode(800, 600, 75000)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .set_mode(800, 600, 75000);
            }),
        }?;

        sender.with_state(|state| {
            let op = state.pinnacle.focused_output().unwrap();
            let mode = op.current_mode().unwrap();
            // unchanged, mode doesn't exist
            assert_eq!(mode.size, (1920, 1080).into());
            assert_eq!(mode.refresh, 60000);

            let new_mode = smithay::output::Mode {
                size: (800, 600).into(),
                refresh: 75000,
            };
            op.add_mode(new_mode);

            // FIXME: this exists because swww was buggy,
            // recheck and dedup
            op.with_state_mut(|state| {
                state.modes.push(new_mode);
            })
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Output.get_focused():set_mode(800, 600, 75000)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .set_mode(800, 600, 75000);
            }),
        }?;

        sender.with_state(|state| {
            let op = state.pinnacle.focused_output().unwrap();
            let mode = op.current_mode().unwrap();
            assert_eq!(mode.size, (800, 600).into());
            assert_eq!(mode.refresh, 75000);
        });

        Ok(())
    })
}

#[test]
fn output_handle_set_custom_mode() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    Output.get_focused():set_custom_mode(800, 600, 75000)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .set_custom_mode(800, 600, 75000);
            }),
        }?;

        sender.with_state(|state| {
            let op = state.pinnacle.focused_output().unwrap();
            let mode = op.current_mode().unwrap();
            assert_eq!(mode.size, (800, 600).into());
            assert_eq!(mode.refresh, 75000);
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Output.get_focused():set_custom_mode(801, 601)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .set_custom_mode(801, 601, None);
            }),
        }?;

        sender.with_state(|state| {
            let op = state.pinnacle.focused_output().unwrap();
            let mode = op.current_mode().unwrap();
            assert_eq!(mode.size, (801, 601).into());
            assert_eq!(mode.refresh, 60000);
        });

        Ok(())
    })
}

#[test]
fn output_handle_set_modeline() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        // 800x600@75
        let modeline = "48.91 800 840 920 1040 600 601 604 627 -HSync +Vsync";

        match lang {
            Lang::Lua => {
                run_lua! {
                    Output.get_focused():set_modeline($modeline)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .set_modeline(modeline.parse().unwrap());
            }),
        }?;

        sender.with_state(|state| {
            let op = state.pinnacle.focused_output().unwrap();
            let mode = op.current_mode().unwrap();
            assert_eq!(mode.size, (800, 600).into());
            assert_eq!(mode.refresh, 75006);
        });

        Ok(())
    })
}

#[test]
fn output_handle_set_scale() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    Output.get_focused():set_scale(1.5)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::output::get_focused().unwrap().set_scale(1.5);
            }),
        }?;

        sender.with_state(|state| {
            let op = state.pinnacle.focused_output().unwrap();
            assert_eq!(op.current_scale().fractional_scale(), 1.5);
        });

        Ok(())
    })
}

#[test]
fn output_handle_change_scale() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    Output.get_focused():change_scale(0.25)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .change_scale(0.25);
            }),
        }?;

        sender.with_state(|state| {
            let op = state.pinnacle.focused_output().unwrap();
            assert_eq!(op.current_scale().fractional_scale(), 1.25);
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Output.get_focused():change_scale(-0.5)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .change_scale(-0.5);
            }),
        }?;

        sender.with_state(|state| {
            let op = state.pinnacle.focused_output().unwrap();
            assert_eq!(op.current_scale().fractional_scale(), 0.75);
        });

        Ok(())
    })
}

#[test]
fn output_handle_set_transform() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    Output.get_focused():set_transform("flipped_90")
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .set_transform(pinnacle_api::output::Transform::Flipped90);
            }),
        }?;

        sender.with_state(|state| {
            let op = state.pinnacle.focused_output().unwrap();
            assert_eq!(op.current_transform(), smithay::utils::Transform::Flipped90);
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Output.get_focused():set_transform("normal")
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .set_transform(pinnacle_api::output::Transform::Normal);
            }),
        }?;

        sender.with_state(|state| {
            let op = state.pinnacle.focused_output().unwrap();
            assert_eq!(op.current_transform(), smithay::utils::Transform::Normal);
        });

        Ok(())
    })
}

#[test]
fn output_handle_set_powered() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    Output.get_focused():set_powered(false)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .set_powered(false);
            }),
        }?;

        sender.with_state(|state| {
            let op = state.pinnacle.focused_output().unwrap();
            assert!(!op.with_state(|state| state.powered))
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Output.get_focused():set_powered(true)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .set_powered(true);
            }),
        }?;

        sender.with_state(|state| {
            let op = state.pinnacle.focused_output().unwrap();
            assert!(op.with_state(|state| state.powered))
        });

        Ok(())
    })
}

#[test]
fn output_handle_toggle_powered() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    Output.get_focused():toggle_powered()
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .toggle_powered();
            }),
        }?;

        sender.with_state(|state| {
            let op = state.pinnacle.focused_output().unwrap();
            assert!(!op.with_state(|state| state.powered))
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Output.get_focused():toggle_powered()
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .toggle_powered();
            }),
        }?;

        sender.with_state(|state| {
            let op = state.pinnacle.focused_output().unwrap();
            assert!(op.with_state(|state| state.powered))
        });

        Ok(())
    })
}

#[test]
fn output_handle_make() -> anyhow::Result<()> {
    test_api(|_sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    local op = Output.get_focused()
                    assert(op:make() == $PINNACLE_1_OUTPUT_MAKE)
                }
            }
            Lang::Rust => run_rust(|| {
                let op = pinnacle_api::output::get_focused().unwrap();
                assert_eq!(op.make(), PINNACLE_1_OUTPUT_MAKE);
            }),
        }?;

        Ok(())
    })
}

#[test]
fn output_handle_model() -> anyhow::Result<()> {
    test_api(|_sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    local op = Output.get_focused()
                    assert(op:model() == $PINNACLE_1_OUTPUT_MODEL)
                }
            }
            Lang::Rust => run_rust(|| {
                let op = pinnacle_api::output::get_focused().unwrap();
                assert_eq!(op.model(), PINNACLE_1_OUTPUT_MODEL);
            }),
        }?;

        Ok(())
    })
}

#[test]
fn output_handle_serial() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        let serial = "this-is-a-serial-138421";

        sender.with_state(|state| {
            let op = state.pinnacle.outputs.keys().next().unwrap();
            op.with_state_mut(|state| state.serial = serial.into());
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    local op = Output.get_focused()
                    assert(op:serial() == $serial)
                }
            }
            Lang::Rust => run_rust(move || {
                let op = pinnacle_api::output::get_focused().unwrap();
                assert_eq!(op.serial(), serial);
            }),
        }?;

        Ok(())
    })
}

#[test]
fn output_handle_loc() -> anyhow::Result<()> {
    test_api(|_sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    local loc = Output.get_focused():loc()
                    assert(loc.x == 0)
                    assert(loc.y == 0)
                }
            }
            Lang::Rust => run_rust(move || {
                let loc = pinnacle_api::output::get_focused().unwrap().loc().unwrap();
                assert_eq!(loc.x, 0);
                assert_eq!(loc.y, 0);
            }),
        }?;

        Ok(())
    })
}

#[test]
fn output_handle_logical_size() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        let logical_width = PINNACLE_1_OUTPUT_SIZE.w;
        let logical_height = PINNACLE_1_OUTPUT_SIZE.h;

        match lang {
            Lang::Lua => {
                run_lua! {
                    local size = Output.get_focused():logical_size()
                    assert(size.width == $logical_width)
                    assert(size.height == $logical_height)
                }
            }
            Lang::Rust => run_rust(move || {
                let size = pinnacle_api::output::get_focused()
                    .unwrap()
                    .logical_size()
                    .unwrap();
                assert_eq!(size.w, logical_width as u32);
                assert_eq!(size.h, logical_height as u32);
            }),
        }?;

        sender.with_state(|state| {
            let output = state.pinnacle.outputs.keys().next().unwrap().clone();
            state.pinnacle.change_output_state(
                &mut state.backend,
                &output,
                None,
                None,
                Some(smithay::output::Scale::Fractional(2.0)),
                None,
            );
        });

        let logical_width = PINNACLE_1_OUTPUT_SIZE.w / 2;
        let logical_height = PINNACLE_1_OUTPUT_SIZE.h / 2;

        match lang {
            Lang::Lua => {
                run_lua! {
                    local size = Output.get_focused():logical_size()
                    assert(size.width == $logical_width)
                    assert(size.height == $logical_height)
                }
            }
            Lang::Rust => run_rust(move || {
                let size = pinnacle_api::output::get_focused()
                    .unwrap()
                    .logical_size()
                    .unwrap();
                assert_eq!(size.w, logical_width as u32);
                assert_eq!(size.h, logical_height as u32);
            }),
        }?;

        Ok(())
    })
}

#[test]
fn output_handle_physical_size() -> anyhow::Result<()> {
    test_api(|_sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    local size = Output.get_focused():physical_size()
                    assert(size.width == 0)
                    assert(size.height == 0)
                }
            }
            Lang::Rust => run_rust(move || {
                let size = pinnacle_api::output::get_focused().unwrap().physical_size();
                assert_eq!(size.w, 0);
                assert_eq!(size.h, 0);
            }),
        }?;

        Ok(())
    })
}

#[test]
fn output_handle_current_mode() -> anyhow::Result<()> {
    test_api(|_sender, lang| {
        let mode_width = PINNACLE_1_OUTPUT_SIZE.w;
        let mode_height = PINNACLE_1_OUTPUT_SIZE.h;
        let mode_refresh = PINNACLE_1_OUTPUT_REFRESH;

        match lang {
            Lang::Lua => {
                run_lua! {
                    local mode = Output.get_focused():current_mode()
                    assert(mode.width == $mode_width)
                    assert(mode.height == $mode_height)
                    assert(mode.refresh_rate_mhz == $mode_refresh)
                }
            }
            Lang::Rust => run_rust(move || {
                let mode = pinnacle_api::output::get_focused()
                    .unwrap()
                    .current_mode()
                    .unwrap();
                assert_eq!(mode.size.w, mode_width as u32);
                assert_eq!(mode.size.h, mode_height as u32);
                assert_eq!(mode.refresh_rate_mhz, mode_refresh as u32);
            }),
        }?;

        Ok(())
    })
}

#[test]
fn output_handle_preferred_mode() -> anyhow::Result<()> {
    test_api(|_sender, lang| {
        let mode_width = PINNACLE_1_OUTPUT_SIZE.w;
        let mode_height = PINNACLE_1_OUTPUT_SIZE.h;
        let mode_refresh = PINNACLE_1_OUTPUT_REFRESH;

        match lang {
            Lang::Lua => {
                run_lua! {
                    local mode = Output.get_focused():preferred_mode()
                    assert(mode.width == $mode_width)
                    assert(mode.height == $mode_height)
                    assert(mode.refresh_rate_mhz == $mode_refresh)
                }
            }
            Lang::Rust => run_rust(move || {
                let mode = pinnacle_api::output::get_focused()
                    .unwrap()
                    .preferred_mode()
                    .unwrap();
                assert_eq!(mode.size.w, mode_width as u32);
                assert_eq!(mode.size.h, mode_height as u32);
                assert_eq!(mode.refresh_rate_mhz, mode_refresh as u32);
            }),
        }?;

        Ok(())
    })
}

#[test]
fn output_handle_modes() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        let first_mode_width = PINNACLE_1_OUTPUT_SIZE.w;
        let first_mode_height = PINNACLE_1_OUTPUT_SIZE.h;
        let first_mode_refresh = PINNACLE_1_OUTPUT_REFRESH;

        sender.with_state(|state| {
            let op = state.pinnacle.outputs.keys().next().unwrap().clone();

            let new_modes = [
                smithay::output::Mode {
                    size: (100, 100).into(),
                    refresh: 30000,
                },
                smithay::output::Mode {
                    size: (200, 200).into(),
                    refresh: 60000,
                },
                smithay::output::Mode {
                    size: (400, 400).into(),
                    refresh: 120000,
                },
            ];

            for mode in new_modes {
                op.add_mode(mode);
                op.with_state_mut(|state| state.modes.push(mode));
            }
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    local modes = Output.get_focused():modes()
                    assert(modes[1].width == $first_mode_width)
                    assert(modes[1].height == $first_mode_height)
                    assert(modes[1].refresh_rate_mhz == $first_mode_refresh)
                    assert(modes[2].width == 100)
                    assert(modes[2].height == 100)
                    assert(modes[2].refresh_rate_mhz == 30000)
                    assert(modes[3].width == 200)
                    assert(modes[3].height == 200)
                    assert(modes[3].refresh_rate_mhz == 60000)
                    assert(modes[4].width == 400)
                    assert(modes[4].height == 400)
                    assert(modes[4].refresh_rate_mhz == 120000)
                }
            }
            Lang::Rust => run_rust(move || {
                let modes = pinnacle_api::output::get_focused()
                    .unwrap()
                    .modes()
                    .collect::<Vec<_>>();
                assert_eq!(modes[0].size.w, first_mode_width as u32);
                assert_eq!(modes[0].size.h, first_mode_height as u32);
                assert_eq!(modes[0].refresh_rate_mhz, first_mode_refresh as u32);
                assert_eq!(modes[1].size.w, 100);
                assert_eq!(modes[1].size.h, 100);
                assert_eq!(modes[1].refresh_rate_mhz, 30000);
                assert_eq!(modes[2].size.w, 200);
                assert_eq!(modes[2].size.h, 200);
                assert_eq!(modes[2].refresh_rate_mhz, 60000);
                assert_eq!(modes[3].size.w, 400);
                assert_eq!(modes[3].size.h, 400);
                assert_eq!(modes[3].refresh_rate_mhz, 120000);
            }),
        }?;

        Ok(())
    })
}

#[test]
fn output_handle_focused() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    local focused = Output.get_by_name($PINNACLE_1_OUTPUT_NAME):focused()
                    assert(focused)
                }
            }
            Lang::Rust => run_rust(move || {
                let focused = pinnacle_api::output::get_by_name(PINNACLE_1_OUTPUT_NAME)
                    .unwrap()
                    .focused();
                assert!(focused);
            }),
        }?;

        sender.with_state(|state| {
            state.pinnacle.new_output(
                "",
                "",
                "",
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
            );
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    local focused = Output.get_by_name($PINNACLE_1_OUTPUT_NAME):focused()
                    assert(not focused)
                }
            }
            Lang::Rust => run_rust(move || {
                let focused = pinnacle_api::output::get_by_name(PINNACLE_1_OUTPUT_NAME)
                    .unwrap()
                    .focused();
                assert!(!focused);
            }),
        }?;

        Ok(())
    })
}

#[test]
fn output_handle_tags() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    local tags = Output.get_by_name($PINNACLE_1_OUTPUT_NAME):tags()
                    assert(#tags == 1)
                    assert(tags[1]:name() == "1")
                }
            }
            Lang::Rust => run_rust(move || {
                let tags = pinnacle_api::output::get_by_name(PINNACLE_1_OUTPUT_NAME)
                    .unwrap()
                    .tags()
                    .collect::<Vec<_>>();
                assert_eq!(tags.len(), 1);
                assert_eq!(tags[0].name(), "1");
            }),
        }?;

        Ok(())
    })
}

#[test]
fn output_handle_scale() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        sender.with_state(|state| {
            let op = state.pinnacle.outputs.keys().next().unwrap().clone();

            state.pinnacle.change_output_state(
                &mut state.backend,
                &op,
                None,
                None,
                Some(smithay::output::Scale::Fractional(1.75)),
                None,
            );
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    local scale = Output.get_by_name($PINNACLE_1_OUTPUT_NAME):scale()
                    assert(scale == 1.75)
                }
            }
            Lang::Rust => run_rust(move || {
                let scale = pinnacle_api::output::get_by_name(PINNACLE_1_OUTPUT_NAME)
                    .unwrap()
                    .scale();
                assert_eq!(scale, 1.75);
            }),
        }?;

        Ok(())
    })
}

#[test]
fn output_handle_transform() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        sender.with_state(|state| {
            let op = state.pinnacle.outputs.keys().next().unwrap().clone();

            state.pinnacle.change_output_state(
                &mut state.backend,
                &op,
                None,
                Some(smithay::utils::Transform::Flipped180),
                None,
                None,
            );
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    local transform = Output.get_by_name($PINNACLE_1_OUTPUT_NAME):transform()
                    assert(transform == "flipped_180")
                }
            }
            Lang::Rust => run_rust(move || {
                let transform = pinnacle_api::output::get_by_name(PINNACLE_1_OUTPUT_NAME)
                    .unwrap()
                    .transform();
                assert_eq!(transform, pinnacle_api::output::Transform::Flipped180);
            }),
        }?;

        Ok(())
    })
}

#[test]
fn output_handle_enabled() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    local enabled = Output.get_by_name($PINNACLE_1_OUTPUT_NAME):enabled()
                    assert(enabled)
                }
            }
            Lang::Rust => run_rust(move || {
                let enabled = pinnacle_api::output::get_by_name(PINNACLE_1_OUTPUT_NAME)
                    .unwrap()
                    .enabled();
                assert!(enabled);
            }),
        }?;

        sender.with_state(|state| {
            let op = state.pinnacle.outputs.keys().next().unwrap().clone();

            state.pinnacle.set_output_enabled(&op, false);
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    local enabled = Output.get_by_name($PINNACLE_1_OUTPUT_NAME):enabled()
                    assert(not enabled)
                }
            }
            Lang::Rust => run_rust(move || {
                let enabled = pinnacle_api::output::get_by_name(PINNACLE_1_OUTPUT_NAME)
                    .unwrap()
                    .enabled();
                assert!(!enabled);
            }),
        }?;

        Ok(())
    })
}

#[test]
fn output_handle_powered() -> anyhow::Result<()> {
    test_api(|_sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    local powered = Output.get_by_name($PINNACLE_1_OUTPUT_NAME):powered()
                    assert(powered)
                }
            }
            Lang::Rust => run_rust(move || {
                let powered = pinnacle_api::output::get_by_name(PINNACLE_1_OUTPUT_NAME)
                    .unwrap()
                    .powered();
                assert!(powered);
            }),
        }?;

        // TODO: set powered to false

        Ok(())
    })
}

// TODO: keyboard_focus_stack
// TODO: keyboard_focus_stack_visible

// TAGS ////////////////////////////////////////////////////

#[test]
fn tag_get_all() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        sender.with_state(|state| {
            let op = state.pinnacle.outputs.keys().next().unwrap().clone();

            pinnacle::api::tag::add(state, ["2".into()], OutputName(op.name()));

            let new_op = state.pinnacle.new_output(
                "pinnacle-2",
                "",
                "",
                (10000, 10000).into(),
                (2560, 1440).into(),
                144000,
                2.0,
                smithay::utils::Transform::Normal,
            );

            pinnacle::api::tag::add(
                state,
                ["buckle".into(), "shoe".into()],
                OutputName(new_op.name()),
            );
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    local tags = Tag.get_all()
                    assert(#tags == 4)
                }
            }
            Lang::Rust => run_rust(move || {
                let tags = pinnacle_api::tag::get_all();
                assert_eq!(tags.count(), 4);
            }),
        }?;

        Ok(())
    })
}

#[test]
fn tag_get() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        sender.with_state(|state| {
            let op = state.pinnacle.outputs.keys().next().unwrap().clone();

            pinnacle::api::tag::add(state, ["2".into()], OutputName(op.name()));

            let new_op = state.pinnacle.new_output(
                "pinnacle-2",
                "",
                "",
                (10000, 10000).into(),
                (2560, 1440).into(),
                144000,
                2.0,
                smithay::utils::Transform::Normal,
            );

            pinnacle::api::tag::add(
                state,
                ["buckle".into(), "shoe".into()],
                OutputName(new_op.name()),
            );
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    local tag = Tag.get("buckle")
                    assert(tag:output().name == "pinnacle-2")

                    local tag = Tag.get("1", Output.get_by_name($PINNACLE_1_OUTPUT_NAME))
                    assert(tag:output().name == $PINNACLE_1_OUTPUT_NAME)

                    local tag = Tag.get("shoe", Output.get_by_name($PINNACLE_1_OUTPUT_NAME))
                    assert(tag == nil)
                }
            }
            Lang::Rust => run_rust(move || {
                let tag = pinnacle_api::tag::get("buckle").unwrap();
                assert_eq!(tag.output().name(), "pinnacle-2");

                let tag = pinnacle_api::tag::get_on_output(
                    "1",
                    &pinnacle_api::output::get_by_name(PINNACLE_1_OUTPUT_NAME).unwrap(),
                )
                .unwrap();
                assert_eq!(tag.output().name(), PINNACLE_1_OUTPUT_NAME);

                let tag = pinnacle_api::tag::get_on_output(
                    "shoe",
                    &pinnacle_api::output::get_by_name(PINNACLE_1_OUTPUT_NAME).unwrap(),
                );
                assert!(tag.is_none());
            }),
        }?;

        Ok(())
    })
}

#[test]
fn tag_add() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    Tag.add(Output.get_focused(), "2", "3")
                }
            }
            Lang::Rust => run_rust(move || {
                let _ = pinnacle_api::tag::add(
                    &pinnacle_api::output::get_focused().unwrap(),
                    ["2", "3"],
                );
            }),
        }?;

        sender.with_state(|state| {
            let op = state.pinnacle.outputs.keys().next().unwrap();
            let tag_count = op.with_state(|state| state.tags.len());
            assert_eq!(tag_count, 3);
        });

        Ok(())
    })
}

#[test]
fn tag_remove() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    local tags = Tag.add(Output.get_focused(), "2", "3")
                    Tag.remove({ tags[1] })
                }
            }
            Lang::Rust => run_rust(move || {
                let mut tags = pinnacle_api::tag::add(
                    &pinnacle_api::output::get_focused().unwrap(),
                    ["2", "3"],
                );
                pinnacle_api::tag::remove([tags.next().unwrap()]);
            }),
        }?;

        sender.with_state(|state| {
            let op = state.pinnacle.outputs.keys().next().unwrap();
            let tags = op.with_state(|state| state.tags.clone());
            assert_eq!(tags.len(), 2);
            assert_eq!(tags[0].name(), "1");
            assert_eq!(tags[1].name(), "3");
        });

        Ok(())
    })
}

// TODO: tag connect_signal

#[test]
fn tag_handle_remove() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    local tags = Tag.add(Output.get_focused(), "2", "3")
                    tags[1]:remove()
                }
            }
            Lang::Rust => run_rust(move || {
                let mut tags = pinnacle_api::tag::add(
                    &pinnacle_api::output::get_focused().unwrap(),
                    ["2", "3"],
                );
                tags.next().unwrap().remove();
            }),
        }?;

        sender.with_state(|state| {
            let op = state.pinnacle.outputs.keys().next().unwrap();
            let tags = op.with_state(|state| state.tags.clone());
            assert_eq!(tags.len(), 2);
            assert_eq!(tags[0].name(), "1");
            assert_eq!(tags[1].name(), "3");
        });

        Ok(())
    })
}

#[test]
fn tag_handle_switch_to() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    local tags = Tag.add(Output.get_focused(), "2", "3")
                    tags[1]:set_active(true)

                    tags[2]:switch_to()
                }
            }
            Lang::Rust => run_rust(move || {
                let mut tags = pinnacle_api::tag::add(
                    &pinnacle_api::output::get_focused().unwrap(),
                    ["2", "3"],
                );
                tags.next().unwrap().set_active(true);

                tags.next().unwrap().switch_to();
            }),
        }?;

        sender.with_state(|state| {
            let op = state.pinnacle.outputs.keys().next().unwrap();
            let tags = op.with_state(|state| state.tags.clone());
            assert!(!tags[0].active());
            assert!(!tags[1].active());
            assert!(tags[2].active());
        });

        Ok(())
    })
}

#[test]
fn tag_handle_set_active() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    local tags = Tag.add(Output.get_focused(), "2", "3")
                    tags[1]:set_active(true)
                    tags[2]:set_active(true)
                    tags[2]:set_active(false)
                }
            }
            Lang::Rust => run_rust(move || {
                let mut tags = pinnacle_api::tag::add(
                    &pinnacle_api::output::get_focused().unwrap(),
                    ["2", "3"],
                );
                tags.next().unwrap().set_active(true);
                let second = tags.next().unwrap();
                second.set_active(true);
                second.set_active(false);
            }),
        }?;

        sender.with_state(|state| {
            let op = state.pinnacle.outputs.keys().next().unwrap();
            let tags = op.with_state(|state| state.tags.clone());
            assert!(!tags[0].active());
            assert!(tags[1].active());
            assert!(!tags[2].active());
        });

        Ok(())
    })
}

#[test]
fn tag_handle_toggle_active() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    local tags = Tag.add(Output.get_focused(), "2", "3")
                    tags[1]:toggle_active()
                    tags[2]:toggle_active()
                    tags[2]:toggle_active()
                }
            }
            Lang::Rust => run_rust(move || {
                let mut tags = pinnacle_api::tag::add(
                    &pinnacle_api::output::get_focused().unwrap(),
                    ["2", "3"],
                );
                tags.next().unwrap().toggle_active();
                let second = tags.next().unwrap();
                second.toggle_active();
                second.toggle_active();
            }),
        }?;

        sender.with_state(|state| {
            let op = state.pinnacle.outputs.keys().next().unwrap();
            let tags = op.with_state(|state| state.tags.clone());
            assert!(!tags[0].active());
            assert!(tags[1].active());
            assert!(!tags[2].active());
        });

        Ok(())
    })
}

#[test]
fn tag_handle_active() -> anyhow::Result<()> {
    test_api(|_sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    local tags = Tag.add(Output.get_focused(), "2", "3")
                    tags[1]:toggle_active()
                    assert(tags[1]:active())
                    tags[1]:toggle_active()
                    assert(not tags[1]:active())
                }
            }
            Lang::Rust => run_rust(move || {
                let mut tags = pinnacle_api::tag::add(
                    &pinnacle_api::output::get_focused().unwrap(),
                    ["2", "3"],
                );
                let first = tags.next().unwrap();
                first.toggle_active();
                assert!(first.active());
                first.toggle_active();
                assert!(!first.active());
            }),
        }?;

        Ok(())
    })
}

#[test]
fn tag_handle_name() -> anyhow::Result<()> {
    test_api(|_sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    local tags = Tag.add(Output.get_focused(), "2", "3")
                    assert(tags[1]:name() == "2")
                    assert(tags[2]:name() == "3")
                }
            }
            Lang::Rust => run_rust(move || {
                let mut tags = pinnacle_api::tag::add(
                    &pinnacle_api::output::get_focused().unwrap(),
                    ["2", "3"],
                );
                assert_eq!(tags.next().unwrap().name(), "2");
                assert_eq!(tags.next().unwrap().name(), "3");
            }),
        }?;

        Ok(())
    })
}

#[test]
fn tag_handle_output() -> anyhow::Result<()> {
    test_api(|_sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    local tags = Tag.add(Output.get_focused(), "2", "3")
                    assert(tags[1]:output().name == $PINNACLE_1_OUTPUT_NAME)
                }
            }
            Lang::Rust => run_rust(move || {
                let mut tags = pinnacle_api::tag::add(
                    &pinnacle_api::output::get_focused().unwrap(),
                    ["2", "3"],
                );
                assert_eq!(tags.next().unwrap().output().name(), PINNACLE_1_OUTPUT_NAME);
            }),
        }?;

        Ok(())
    })
}

// TODO: tag_handle_windows

#[test]
fn tag_get_all_does_not_return_tags_cleared_after_config_reload() -> anyhow::Result<()> {
    test_api(|_sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    Tag.add(Output.get_focused(), "2", "3")
                    assert(#Tag.get_all() == 3)
                    Pinnacle.reload_config()
                    assert(#Tag.get_all() == 0)
                }
            }
            Lang::Rust => run_rust(move || {
                let _ = pinnacle_api::tag::add(
                    &pinnacle_api::output::get_focused().unwrap(),
                    ["2", "3"],
                );
                assert_eq!(pinnacle_api::tag::get_all().count(), 3);
                pinnacle_api::pinnacle::reload_config();
                assert_eq!(pinnacle_api::tag::get_all().count(), 0);
            }),
        }?;

        Ok(())
    })
}

#[test]
fn tag_get_does_not_return_tags_cleared_after_config_reload() -> anyhow::Result<()> {
    test_api(|_sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    Tag.add(Output.get_focused(), "2", "3")
                    assert(Tag.get("1"))
                    Pinnacle.reload_config()
                    assert(not Tag.get("1"))
                }
            }
            Lang::Rust => run_rust(move || {
                let _ = pinnacle_api::tag::add(
                    &pinnacle_api::output::get_focused().unwrap(),
                    ["2", "3"],
                );
                assert!(pinnacle_api::tag::get("1").is_some());
                pinnacle_api::pinnacle::reload_config();
                assert!(pinnacle_api::tag::get("1").is_none());
            }),
        }?;

        Ok(())
    })
}

// WINDOW ///////////////////////////////////////////

fn window_set_up_test(lang: Lang) -> anyhow::Result<()> {
    match lang {
        Lang::Lua => {
            run_lua! {
                Tag.get("1"):set_active(true)
                Process.spawn("alacritty", "-o", "general.ipc_socket=false")
            }
        }
        Lang::Rust => run_rust(|| {
            pinnacle_api::tag::get("1").unwrap().set_active(true);
            pinnacle_api::process::Command::new("alacritty")
                .args(["-o", "general.ipc_socket=false"])
                .spawn()
                .unwrap();
        }),
    }
}

#[test]
fn window_get_all() -> anyhow::Result<()> {
    test_api(|_sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    assert(#Window.get_all() == 0)

                    for i = 1, 5 do
                        Process.spawn(
                            "alacritty",
                            "-o",
                            "general.ipc_socket=false"
                        )
                    end
                }
            }
            Lang::Rust => run_rust(|| {
                assert_eq!(pinnacle_api::window::get_all().count(), 0);

                for _ in 0..5 {
                    pinnacle_api::process::Command::new("alacritty")
                        .args(["-o", "general.ipc_socket=false"])
                        .spawn()
                        .unwrap();
                }
            }),
        }?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    assert(#Window.get_all() == 5)
                }
            }
            Lang::Rust => run_rust(|| {
                assert_eq!(pinnacle_api::window::get_all().count(), 5);
            }),
        }?;

        Ok(())
    })
}

#[test]
fn window_get_focused() -> anyhow::Result<()> {
    test_api(|_sender, lang| {
        window_set_up_test(lang)?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    assert(Window.get_focused())
                }
            }
            Lang::Rust => run_rust(|| {
                assert!(pinnacle_api::window::get_focused().is_some());
            }),
        }?;

        Ok(())
    })
}

// TODO: window_begin_move
// TODO: window_begin_resize
// TODO: window_connect_signal
// TODO: window_add_window_rule

#[test]
fn window_handle_close() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        window_set_up_test(lang)?;

        sleep(SLEEP_DURATION);

        sender.with_state(|state| {
            assert_eq!(state.pinnacle.windows.len(), 1);
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():close()
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused().unwrap().close();
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(|state| {
            assert_eq!(state.pinnacle.windows.len(), 0);
        });

        Ok(())
    })
}

#[test]
fn window_handle_set_geometry_floating() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        setup_lua! {
            Layout.manage(function(args)
                local node = Layout.builtin.master_stack():layout(args.window_count)
                return node
            end)
        };

        sleep(SLEEP_DURATION);

        window_set_up_test(lang)?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():set_floating(true)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_floating(true);
            }),
        }?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():set_geometry({
                        x = 222,
                        y = 333,
                        width = 444,
                        height = 555,
                    })
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_geometry(222, 333, 444, 555);
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(|state| {
            let win = &state.pinnacle.windows[0];
            let geo = state.pinnacle.space.element_geometry(win).unwrap();
            assert_eq!(geo.loc.x, 222);
            assert_eq!(geo.loc.y, 333);
            assert_eq!(geo.size.w, 444);
            assert_eq!(geo.size.h, 555);
        });

        Ok(())
    })
}

#[test]
fn window_handle_set_geometry_tiled_does_not_change_geometry() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        setup_lua! {
            Layout.manage(function(args)
                local node = Layout.builtin.master_stack():layout(args.window_count)
                return node
            end)
        };

        sleep(SLEEP_DURATION);

        window_set_up_test(lang)?;

        sleep(SLEEP_DURATION);

        let win_geo = Arc::new(OnceLock::new());
        sender.with_state({
            let win_geo = win_geo.clone();
            move |state| {
                let win = &state.pinnacle.windows[0];
                let geo = state.pinnacle.space.element_geometry(win).unwrap();
                win_geo.set(geo).unwrap();
            }
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():set_geometry({
                        x = 222,
                        y = 333,
                        width = 444,
                        height = 555,
                    })
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_geometry(222, 333, 444, 555);
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let win = &state.pinnacle.windows[0];
            let geo = state.pinnacle.space.element_geometry(win).unwrap();
            assert_eq!(geo, *win_geo.get().unwrap());
        });

        Ok(())
    })
}

#[test]
fn window_handle_set_fullscreen() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        window_set_up_test(lang)?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():set_fullscreen(true)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_fullscreen(true);
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let win = &state.pinnacle.windows[0];
            let layout_mode = win.with_state(|state| state.layout_mode.current());
            assert_eq!(layout_mode, LayoutModeKind::Fullscreen);
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():set_fullscreen(false)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_fullscreen(false);
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let win = &state.pinnacle.windows[0];
            let layout_mode = win.with_state(|state| state.layout_mode.current());
            assert_ne!(layout_mode, LayoutModeKind::Fullscreen);
        });

        Ok(())
    })
}

#[test]
fn window_handle_toggle_fullscreen() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        window_set_up_test(lang)?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():toggle_fullscreen()
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .toggle_fullscreen();
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let win = &state.pinnacle.windows[0];
            let layout_mode = win.with_state(|state| state.layout_mode.current());
            assert_eq!(layout_mode, LayoutModeKind::Fullscreen);
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():toggle_fullscreen()
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .toggle_fullscreen();
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let win = &state.pinnacle.windows[0];
            let layout_mode = win.with_state(|state| state.layout_mode.current());
            assert_ne!(layout_mode, LayoutModeKind::Fullscreen);
        });

        Ok(())
    })
}

#[test]
fn window_handle_set_maximized() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        window_set_up_test(lang)?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():set_maximized(true)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_maximized(true);
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let win = &state.pinnacle.windows[0];
            let layout_mode = win.with_state(|state| state.layout_mode.current());
            assert_eq!(layout_mode, LayoutModeKind::Maximized);
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():set_maximized(false)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_maximized(false);
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let win = &state.pinnacle.windows[0];
            let layout_mode = win.with_state(|state| state.layout_mode.current());
            assert_ne!(layout_mode, LayoutModeKind::Maximized);
        });

        Ok(())
    })
}

#[test]
fn window_handle_toggle_maximized() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        window_set_up_test(lang)?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():toggle_maximized()
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .toggle_maximized();
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let win = &state.pinnacle.windows[0];
            let layout_mode = win.with_state(|state| state.layout_mode.current());
            assert_eq!(layout_mode, LayoutModeKind::Maximized);
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():toggle_maximized()
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .toggle_maximized();
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let win = &state.pinnacle.windows[0];
            let layout_mode = win.with_state(|state| state.layout_mode.current());
            assert_ne!(layout_mode, LayoutModeKind::Maximized);
        });

        Ok(())
    })
}

#[test]
fn window_handle_set_floating() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        window_set_up_test(lang)?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():set_floating(true)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_floating(true);
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let win = &state.pinnacle.windows[0];
            let layout_mode = win.with_state(|state| state.layout_mode.current());
            assert_eq!(layout_mode, LayoutModeKind::Floating);
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():set_floating(false)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_floating(false);
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let win = &state.pinnacle.windows[0];
            let layout_mode = win.with_state(|state| state.layout_mode.current());
            assert_ne!(layout_mode, LayoutModeKind::Floating);
        });

        Ok(())
    })
}

#[test]
fn window_handle_toggle_floating() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        window_set_up_test(lang)?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():toggle_floating()
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .toggle_floating();
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let win = &state.pinnacle.windows[0];
            let layout_mode = win.with_state(|state| state.layout_mode.current());
            assert_eq!(layout_mode, LayoutModeKind::Floating);
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():toggle_floating()
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .toggle_floating();
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let win = &state.pinnacle.windows[0];
            let layout_mode = win.with_state(|state| state.layout_mode.current());
            assert_ne!(layout_mode, LayoutModeKind::Floating);
        });

        Ok(())
    })
}

#[test]
fn window_handle_set_focused() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        window_set_up_test(lang)?;

        match lang {
            Lang::Lua => {
                run_lua! {
                    Process.spawn("alacritty", "-o", "general.ipc_socket=false")
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::process::Command::new("alacritty")
                    .args(["-o", "general.ipc_socket=false"])
                    .spawn()
                    .unwrap();
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(|state| {
            let op = state.pinnacle.focused_output().unwrap();
            assert_eq!(
                state.pinnacle.focused_window(op).as_ref(),
                Some(&state.pinnacle.windows[1])
            );
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():set_focused(false)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_focused(false);
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let op = state.pinnacle.focused_output().unwrap();
            assert!(state.pinnacle.focused_window(op).is_none());
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_all()[1]:set_focused(true)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_all()
                    .next()
                    .unwrap()
                    .set_focused(true);
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let op = state.pinnacle.focused_output().unwrap();
            assert_eq!(
                state.pinnacle.focused_window(op).as_ref(),
                Some(&state.pinnacle.windows[0])
            );
        });

        Ok(())
    })
}

#[test]
fn window_handle_toggle_focused() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        window_set_up_test(lang)?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():toggle_focused()
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .toggle_focused();
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let op = state.pinnacle.focused_output().unwrap();
            assert!(state.pinnacle.focused_window(op).is_none());
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_all()[1]:toggle_focused(true)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_all()
                    .next()
                    .unwrap()
                    .toggle_focused();
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let op = state.pinnacle.focused_output().unwrap();
            assert_eq!(
                state.pinnacle.focused_window(op).as_ref(),
                Some(&state.pinnacle.windows[0])
            );
        });

        Ok(())
    })
}

#[test]
fn window_handle_set_decoration_mode() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        setup_lua! {
            // Alacritty causes a shm protocol error without this,
            // idk why
            Layout.manage(function(args)
                local node = Layout.builtin.master_stack():layout(args.window_count)
                return node
            end)
        };

        window_set_up_test(lang)?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():set_decoration_mode("client_side")
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_decoration_mode(pinnacle_api::window::DecorationMode::ClientSide);
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let win = &state.pinnacle.windows[0];
            assert_eq!(
                win.with_state(|state| state.decoration_mode),
                Some(zxdg_toplevel_decoration_v1::Mode::ClientSide)
            );
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():set_decoration_mode("server_side")
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_decoration_mode(pinnacle_api::window::DecorationMode::ServerSide);
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let win = &state.pinnacle.windows[0];
            assert_eq!(
                win.with_state(|state| state.decoration_mode),
                Some(zxdg_toplevel_decoration_v1::Mode::ServerSide)
            );
        });

        Ok(())
    })
}

#[test]
fn window_handle_move_to_tag() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        window_set_up_test(lang)?;

        match lang {
            Lang::Lua => {
                run_lua! {
                    Tag.add(Output.get_focused(), "2", "3")
                }
            }
            Lang::Rust => run_rust(|| {
                let _ = pinnacle_api::tag::add(
                    &pinnacle_api::output::get_focused().unwrap(),
                    ["2", "3"],
                );
            }),
        }?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():move_to_tag(Tag.get("3"))
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .move_to_tag(&pinnacle_api::tag::get("3").unwrap());
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let tags = state.pinnacle.windows[0].with_state(|state| state.tags.clone());
            assert_eq!(tags.len(), 1);
            assert_eq!(tags[0].name(), "3");
        });

        Ok(())
    })
}

#[test]
fn window_handle_set_tag() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        window_set_up_test(lang)?;

        match lang {
            Lang::Lua => {
                run_lua! {
                    Tag.add(Output.get_focused(), "2", "3")
                }
            }
            Lang::Rust => run_rust(|| {
                let _ = pinnacle_api::tag::add(
                    &pinnacle_api::output::get_focused().unwrap(),
                    ["2", "3"],
                );
            }),
        }?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():set_tag(Tag.get("3"), true)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_tag(&pinnacle_api::tag::get("3").unwrap(), true);
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let tags = state.pinnacle.windows[0].with_state(|state| state.tags.clone());
            assert_eq!(tags.len(), 2);
            assert_eq!(tags[0].name(), "1");
            assert_eq!(tags[1].name(), "3");
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():set_tag(Tag.get("1"), false)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_tag(&pinnacle_api::tag::get("1").unwrap(), false);
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let tags = state.pinnacle.windows[0].with_state(|state| state.tags.clone());
            assert_eq!(tags.len(), 1);
            assert_eq!(tags[0].name(), "3");
        });

        Ok(())
    })
}

#[test]
fn window_handle_toggle_tag() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        window_set_up_test(lang)?;

        match lang {
            Lang::Lua => {
                run_lua! {
                    Tag.add(Output.get_focused(), "2", "3")
                }
            }
            Lang::Rust => run_rust(|| {
                let _ = pinnacle_api::tag::add(
                    &pinnacle_api::output::get_focused().unwrap(),
                    ["2", "3"],
                );
            }),
        }?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():toggle_tag(Tag.get("3"))
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .toggle_tag(&pinnacle_api::tag::get("3").unwrap());
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let tags = state.pinnacle.windows[0].with_state(|state| state.tags.clone());
            assert_eq!(tags.len(), 2);
            assert_eq!(tags[0].name(), "1");
            assert_eq!(tags[1].name(), "3");
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():toggle_tag(Tag.get("3"))
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .toggle_tag(&pinnacle_api::tag::get("3").unwrap());
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let tags = state.pinnacle.windows[0].with_state(|state| state.tags.clone());
            assert_eq!(tags.len(), 1);
            assert_eq!(tags[0].name(), "1");
        });

        Ok(())
    })
}

#[test]
fn window_handle_raise() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        window_set_up_test(lang)?;

        match lang {
            Lang::Lua => {
                run_lua! {
                    Process.spawn("alacritty", "-o", "general.ipc_socket=false")
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::process::Command::new("alacritty")
                    .args(["-o", "general.ipc_socket=false"])
                    .spawn()
                    .unwrap();
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let top = state.pinnacle.z_index_stack.last().unwrap();
            let second = &state.pinnacle.windows[1];
            assert_eq!(top, second);
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_all()[1]:raise()
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_all().next().unwrap().raise();
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let top = state.pinnacle.z_index_stack.last().unwrap();
            let first = &state.pinnacle.windows[0];
            assert_eq!(top, first);
        });

        Ok(())
    })
}

#[test]
fn window_handle_is_on_active_tag() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        window_set_up_test(lang)?;

        match lang {
            Lang::Lua => {
                run_lua! {
                    Process.spawn("alacritty", "-o", "general.ipc_socket=false")
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::process::Command::new("alacritty")
                    .args(["-o", "general.ipc_socket=false"])
                    .spawn()
                    .unwrap();
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(move |state| {
            let win = &state.pinnacle.windows[0];
            assert!(win.is_on_active_tag());
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Tag.get("1"):set_active(false)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::tag::get("1").unwrap().set_active(false);
            }),
        }?;

        sender.with_state(move |state| {
            let win = &state.pinnacle.windows[0];
            assert!(!win.is_on_active_tag());
        });

        Ok(())
    })
}

#[test]
fn window_handle_loc() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        setup_lua! {
            Layout.manage(function(args)
                local node = Layout.builtin.master_stack():layout(args.window_count)
                return node
            end)
        };

        window_set_up_test(lang)?;

        sleep(SLEEP_DURATION);

        let loc = Arc::new(OnceLock::new());
        sender.with_state({
            let loc = loc.clone();
            move |state| {
                let win = &state.pinnacle.windows[0];
                let point = state.pinnacle.space.element_location(win).unwrap();
                loc.set(point).unwrap();
            }
        });

        sleep(SLEEP_DURATION);

        let x = loc.get().unwrap().x;
        let y = loc.get().unwrap().y;

        match lang {
            Lang::Lua => {
                run_lua! {
                    local loc = Window.get_focused():loc()
                    assert(loc.x == $x)
                    assert(loc.y == $y)
                }
            }
            Lang::Rust => run_rust(move || {
                let loc = pinnacle_api::window::get_focused().unwrap().loc().unwrap();
                assert_eq!(loc.x, x);
                assert_eq!(loc.y, y);
            }),
        }?;

        Ok(())
    })
}

#[test]
fn window_handle_size() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        setup_lua! {
            Layout.manage(function(args)
                local node = Layout.builtin.master_stack():layout(args.window_count)
                return node
            end)
        };

        window_set_up_test(lang)?;

        sleep(SLEEP_DURATION);

        let size = Arc::new(OnceLock::new());
        sender.with_state({
            let size = size.clone();
            move |state| {
                let win = &state.pinnacle.windows[0];
                let rect = state.pinnacle.space.element_geometry(win).unwrap();
                size.set(rect.size).unwrap();
            }
        });

        sleep(SLEEP_DURATION);

        let width = size.get().unwrap().w;
        let height = size.get().unwrap().h;

        match lang {
            Lang::Lua => {
                run_lua! {
                    local size = Window.get_focused():size()
                    assert(size.width == $width)
                    assert(size.height == $height)
                }
            }
            Lang::Rust => run_rust(move || {
                let size = pinnacle_api::window::get_focused().unwrap().size().unwrap();
                assert_eq!(size.w, width as u32);
                assert_eq!(size.h, height as u32);
            }),
        }?;

        Ok(())
    })
}

#[test]
fn window_handle_app_id() -> anyhow::Result<()> {
    test_api(|_sender, lang| {
        window_set_up_test(lang)?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    assert(Window.get_focused():app_id(), "Alacritty")
                }
            }
            Lang::Rust => run_rust(move || {
                assert_eq!(
                    pinnacle_api::window::get_focused().unwrap().app_id(),
                    "Alacritty"
                );
            }),
        }?;

        Ok(())
    })
}

#[test]
fn window_handle_title() -> anyhow::Result<()> {
    test_api(|_sender, lang| {
        window_set_up_test(lang)?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    Process.spawn(
                        "alacritty",
                        "-o",
                        "general.ipc_socket=false",
                        "-o",
                        "window.dynamic_title=false"
                    )
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::process::Command::new("alacritty")
                    .args([
                        "-o",
                        "general.ipc_socket=false",
                        "-o",
                        "window.dynamic_title=false",
                    ])
                    .spawn()
                    .unwrap();
            }),
        }?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    assert(Window.get_focused():title(), "Alacritty")
                }
            }
            Lang::Rust => run_rust(move || {
                assert_eq!(
                    pinnacle_api::window::get_focused().unwrap().title(),
                    "Alacritty"
                );
            }),
        }?;

        Ok(())
    })
}

#[test]
fn window_handle_focused() -> anyhow::Result<()> {
    test_api(|_sender, lang| {
        window_set_up_test(lang)?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    Process.spawn("alacritty", "-o", "general.ipc_socket=false")
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::process::Command::new("alacritty")
                    .args(["-o", "general.ipc_socket=false"])
                    .spawn()
                    .unwrap();
            }),
        }?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    assert(Window.get_focused():focused())
                    assert(not Window.get_all()[1]:focused())
                }
            }
            Lang::Rust => run_rust(move || {
                assert!(pinnacle_api::window::get_focused().unwrap().focused());
                assert!(!pinnacle_api::window::get_all().next().unwrap().focused());
            }),
        }?;

        Ok(())
    })
}

#[test]
fn window_handle_layout_mode() -> anyhow::Result<()> {
    test_api(|_sender, lang| {
        window_set_up_test(lang)?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():set_floating(true)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_floating(true);
            }),
        }?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    local win = Window.get_focused()
                    assert(win:floating())
                    assert(not win:tiled())
                    assert(not win:fullscreen())
                    assert(not win:maximized())
                }
            }
            Lang::Rust => run_rust(move || {
                let win = pinnacle_api::window::get_focused().unwrap();
                assert_eq!(
                    win.layout_mode(),
                    pinnacle_api::window::LayoutMode::Floating
                );
                // TODO: win.tiled()
                assert!(win.floating());
                assert!(!win.fullscreen());
                assert!(!win.maximized());
            }),
        }?;

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():set_floating(false)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_floating(false);
            }),
        }?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    local win = Window.get_focused()
                    assert(win:tiled())
                    assert(not win:floating())
                    assert(not win:fullscreen())
                    assert(not win:maximized())
                }
            }
            Lang::Rust => run_rust(move || {
                let win = pinnacle_api::window::get_focused().unwrap();
                assert_eq!(win.layout_mode(), pinnacle_api::window::LayoutMode::Tiled);
                // TODO: win.tiled()
                assert!(!win.floating());
                assert!(!win.fullscreen());
                assert!(!win.maximized());
            }),
        }?;

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():set_fullscreen(true)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_fullscreen(true);
            }),
        }?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    local win = Window.get_focused()
                    assert(win:fullscreen())
                    assert(not win:tiled())
                    assert(not win:floating())
                    assert(not win:maximized())
                }
            }
            Lang::Rust => run_rust(move || {
                let win = pinnacle_api::window::get_focused().unwrap();
                assert_eq!(
                    win.layout_mode(),
                    pinnacle_api::window::LayoutMode::Fullscreen
                );
                // TODO: win.tiled()
                assert!(!win.floating());
                assert!(win.fullscreen());
                assert!(!win.maximized());
            }),
        }?;

        match lang {
            Lang::Lua => {
                run_lua! {
                    Window.get_focused():set_maximized(true)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::window::get_focused()
                    .unwrap()
                    .set_maximized(true);
            }),
        }?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    local win = Window.get_focused()
                    assert(not win:tiled())
                    assert(not win:floating())
                    assert(not win:fullscreen())
                    assert(win:maximized())
                }
            }
            Lang::Rust => run_rust(move || {
                let win = pinnacle_api::window::get_focused().unwrap();
                assert_eq!(
                    win.layout_mode(),
                    pinnacle_api::window::LayoutMode::Maximized
                );
                // TODO: win.tiled()
                assert!(!win.floating());
                assert!(!win.fullscreen());
                assert!(win.maximized());
            }),
        }?;

        Ok(())
    })
}

#[test]
fn window_handle_tags() -> anyhow::Result<()> {
    test_api(|_sender, lang| {
        window_set_up_test(lang)?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    local tags = Window.get_focused():tags()
                    assert(#tags == 1)
                    assert(tags[1]:name() == "1")
                }
            }
            Lang::Rust => run_rust(|| {
                let tags = pinnacle_api::window::get_focused()
                    .unwrap()
                    .tags()
                    .collect::<Vec<_>>();
                assert_eq!(tags.len(), 1);
                assert_eq!(tags[0].name(), "1");
            }),
        }?;

        Ok(())
    })
}

#[test]
fn window_spawned_without_tags_gets_tags_after_add() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        sender.with_state(|state| {
            for output in state.pinnacle.outputs.keys() {
                output.with_state_mut(|state| state.tags.clear());
            }
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Process.spawn("alacritty", "-o", "general.ipc_socket=false")
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::process::Command::new("alacritty")
                    .args(["-o", "general.ipc_socket=false"])
                    .spawn()
                    .unwrap();
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(|state| {
            assert_eq!(state.pinnacle.windows.len(), 0);
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Tag.add(Output.get_focused(), "new_tag");
                }
            }
            Lang::Rust => run_rust(|| {
                let _ = pinnacle_api::tag::add(
                    &pinnacle_api::output::get_focused().unwrap(),
                    ["new_tag"],
                );
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(|state| {
            assert_eq!(state.pinnacle.windows.len(), 1);
            let tag_count = state.pinnacle.windows[0].with_state(|state| state.tags.len());
            assert_eq!(tag_count, 1);
        });

        Ok(())
    })
}

// PROCESS //////////////////////////////////////////

#[test]
fn process_spawn() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    Process.spawn("alacritty", "-o", "general.ipc_socket=false")
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::process::Command::new("alacritty")
                    .args(["-o", "general.ipc_socket=false"])
                    .spawn()
                    .unwrap();
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(|state| {
            assert_eq!(state.pinnacle.windows.len(), 1);
            assert_eq!(
                state.pinnacle.windows[0].class(),
                Some("Alacritty".to_string())
            );
        });

        Ok(())
    })
}

#[test]
fn process_spawn_unique() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        // Sleep so any windows from previous tests close
        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    assert(Process.spawn_unique("alacritty"))
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::process::Command::new("alacritty")
                    .args(["-o", "general.ipc_socket=false"])
                    .unique()
                    .spawn()
                    .unwrap();
            }),
        }?;

        sleep(SLEEP_DURATION);

        match lang {
            Lang::Lua => {
                run_lua! {
                    Process.spawn_unique("alacritty")
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::process::Command::new("alacritty")
                    .args(["-o", "general.ipc_socket=false"])
                    .unique()
                    .spawn();
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(|state| {
            assert_eq!(state.pinnacle.windows.len(), 1);
            assert_eq!(
                state.pinnacle.windows[0].class(),
                Some("Alacritty".to_string())
            );
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Process.command({
                        cmd = "alacritty",
                        shell_cmd = { "bash", "-c" },
                        unique = true,
                    }):spawn()
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::process::Command::with_shell(["bash", "-c"], "alacritty")
                    .unique()
                    .spawn();
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(|state| {
            assert_eq!(state.pinnacle.windows.len(), 1);
            assert_eq!(
                state.pinnacle.windows[0].class(),
                Some("Alacritty".to_string())
            );
        });

        Ok(())
    })
}

#[test]
fn process_spawn_once() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    assert(Process.spawn_once("alacritty"))
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::process::Command::new("alacritty")
                    .args(["-o", "general.ipc_socket=false"])
                    .once()
                    .spawn()
                    .unwrap();
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(|state| {
            assert_eq!(state.pinnacle.windows.len(), 1);
            assert_eq!(
                state.pinnacle.windows[0].class(),
                Some("Alacritty".to_string())
            );
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    for _, window in ipairs(Window.get_all()) do
                        window:close()
                    end
                }
            }
            Lang::Rust => run_rust(|| {
                for window in pinnacle_api::window::get_all() {
                    window.close();
                }
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(|state| {
            assert_eq!(state.pinnacle.windows.len(), 0);
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Process.spawn_once("alacritty")
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::process::Command::new("alacritty")
                    .args(["-o", "general.ipc_socket=false"])
                    .once()
                    .spawn();
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(|state| {
            assert_eq!(state.pinnacle.windows.len(), 0);
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Process.command({
                        cmd = "alacritty",
                        shell_cmd = { "bash", "-c" },
                        once = true,
                    }):spawn()
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::process::Command::with_shell(["bash", "-c"], "alacritty")
                    .once()
                    .spawn();
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(|state| {
            assert_eq!(state.pinnacle.windows.len(), 0);
        });

        Ok(())
    })
}

#[test]
fn process_stdio() -> anyhow::Result<()> {
    test_api(|_sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    local child = Process.command({
                        cmd = "echo 'hello there'",
                        shell_cmd = { "bash", "-c" },
                    }):spawn()
                    local out = child.stdout:read()
                    assert(out == "hello there")

                    local child = Process.command({
                        cmd = "echo 'hello there' 1>&2",
                        shell_cmd = { "bash", "-c" },
                    }):spawn()
                    local err = child.stderr:read()
                    assert(err == "hello there")

                    local child = Process.command({
                        cmd = "cat",
                    }):spawn()
                    child.stdin:write("sussus amogus")
                    child.stdin:flush()
                    child.stdin:close()
                    local out = child.stdout:read("*a")
                    assert(out == "sussus amogus")
                }
            }
            Lang::Rust => run_rust(|| {
                // Turning the tokio stuff into files to sidestep async

                let mut child = pinnacle_api::process::Command::with_shell(
                    ["bash", "-c"],
                    "echo 'hello there'",
                )
                .spawn()
                .unwrap();
                let mut out = String::new();
                let mut stdout: File = child.stdout.take().unwrap().into_owned_fd().unwrap().into();
                stdout.read_to_string(&mut out).unwrap();
                assert_eq!(out, "hello there\n");

                let mut child = pinnacle_api::process::Command::with_shell(
                    ["bash", "-c"],
                    "echo 'hello there' 1>&2",
                )
                .spawn()
                .unwrap();
                let mut err = String::new();
                let mut stderr: File = child.stderr.take().unwrap().into_owned_fd().unwrap().into();
                stderr.read_to_string(&mut err).unwrap();
                assert_eq!(err, "hello there\n");

                let mut child = pinnacle_api::process::Command::new("cat").spawn().unwrap();
                let mut stdin: File = child.stdin.take().unwrap().into_owned_fd().unwrap().into();
                stdin.write_all(b"sussus amogus").unwrap();
                drop(stdin);
                let mut out = String::new();
                let mut stdout: File = child.stdout.take().unwrap().into_owned_fd().unwrap().into();
                stdout.read_to_string(&mut out).unwrap();
                assert_eq!(out, "sussus amogus");
            }),
        }?;

        Ok(())
    })
}

// INPUT //////////////////////////////////////////////////////////////

#[test]
fn input_set_xkb_config() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    Input.set_xkb_config({
                        layout = "us,fr,ge",
                    })
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::input::set_xkb_config(
                    pinnacle_api::input::XkbConfig::new().with_layout("us,fr,ge"),
                );
            }),
        }?;

        sender.with_state(|state| {
            let kb = state.pinnacle.seat.get_keyboard().unwrap();
            let layouts = kb.with_xkb_state(state, |ctx| {
                let xkb = ctx.xkb().lock().unwrap();
                xkb.layouts()
                    .map(|layout| xkb.layout_name(layout).to_string())
                    .collect::<Vec<_>>()
            });
            assert_eq!(
                layouts,
                [
                    "English (US)".to_string(),
                    "French".to_string(),
                    "Georgian".to_string()
                ]
            );
        });

        Ok(())
    })
}

#[test]
fn input_switch_xkb_layout() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    Input.set_xkb_config({
                        layout = "us,fr,ge",
                    })
                    Input.cycle_xkb_layout_backward()
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::input::set_xkb_config(
                    pinnacle_api::input::XkbConfig::new().with_layout("us,fr,ge"),
                );
                pinnacle_api::input::cycle_xkb_layout_backward();
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(|state| {
            let kb = state.pinnacle.seat.get_keyboard().unwrap();
            let layout_idx = kb.with_xkb_state(state, |ctx| {
                let xkb = ctx.xkb().lock().unwrap();
                xkb.active_layout().0
            });
            assert_eq!(layout_idx, 2);
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Input.cycle_xkb_layout_forward()
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::input::cycle_xkb_layout_forward();
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(|state| {
            let kb = state.pinnacle.seat.get_keyboard().unwrap();
            let layout_idx = kb.with_xkb_state(state, |ctx| {
                let xkb = ctx.xkb().lock().unwrap();
                xkb.active_layout().0
            });
            assert_eq!(layout_idx, 0);
        });

        match lang {
            Lang::Lua => {
                run_lua! {
                    Input.switch_xkb_layout(1)
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::input::switch_xkb_layout(1);
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(|state| {
            let kb = state.pinnacle.seat.get_keyboard().unwrap();
            let layout_idx = kb.with_xkb_state(state, |ctx| {
                let xkb = ctx.xkb().lock().unwrap();
                xkb.active_layout().0
            });
            assert_eq!(layout_idx, 1);
        });

        Ok(())
    })
}

#[test]
fn input_keybind() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    Input.keybind({
                        mods = { "super", "shift" },
                        key = "c",
                        bind_layer = "morb_layer",
                        group = "Left",
                        description = "Right",
                        allow_when_locked = true,
                        on_press = function() end,
                    })
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::input::BindLayer::get("morb_layer")
                    .keybind(
                        pinnacle_api::input::Mod::SUPER | pinnacle_api::input::Mod::SHIFT,
                        'c',
                    )
                    .group("Left")
                    .description("Right")
                    .allow_when_locked()
                    .on_press(|| {});
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(|state| {
            let keybind = state
                .pinnacle
                .input_state
                .bind_state
                .keybinds
                .id_map
                .iter()
                .next()
                .unwrap()
                .1
                .clone();
            let keybind = keybind.borrow();

            assert_eq!(keybind.key, pinnacle_api::Keysym::c);
            assert_eq!(keybind.bind_data.layer.as_deref(), Some("morb_layer"));
            assert_eq!(keybind.bind_data.group, "Left");
            assert_eq!(keybind.bind_data.desc, "Right");
            assert!(!keybind.bind_data.is_quit_bind);
            assert!(!keybind.bind_data.is_reload_config_bind);
            assert!(keybind.bind_data.allow_when_locked);
            assert!(keybind.has_on_press)
        });

        Ok(())
    })
}

#[test]
fn input_mousebind() -> anyhow::Result<()> {
    test_api(|sender, lang| {
        match lang {
            Lang::Lua => {
                run_lua! {
                    Input.mousebind({
                        mods = { "super", "shift" },
                        button = "btn_right",
                        bind_layer = "morb_layer",
                        group = "Left",
                        description = "Right",
                        allow_when_locked = true,
                        on_press = function() end,
                    })
                }
            }
            Lang::Rust => run_rust(|| {
                pinnacle_api::input::BindLayer::get("morb_layer")
                    .mousebind(
                        pinnacle_api::input::Mod::SUPER | pinnacle_api::input::Mod::SHIFT,
                        pinnacle_api::input::MouseButton::Right,
                    )
                    .group("Left")
                    .description("Right")
                    .allow_when_locked()
                    .on_press(|| {});
            }),
        }?;

        sleep(SLEEP_DURATION);

        sender.with_state(|state| {
            let mousebind = state
                .pinnacle
                .input_state
                .bind_state
                .mousebinds
                .id_map
                .iter()
                .next()
                .unwrap()
                .1
                .clone();
            let mousebind = mousebind.borrow();

            assert_eq!(
                mousebind.button,
                u32::from(pinnacle_api::input::MouseButton::Right)
            );
            assert_eq!(mousebind.bind_data.layer.as_deref(), Some("morb_layer"));
            assert_eq!(mousebind.bind_data.group, "Left");
            assert_eq!(mousebind.bind_data.desc, "Right");
            assert!(!mousebind.bind_data.is_quit_bind);
            assert!(!mousebind.bind_data.is_reload_config_bind);
            assert!(mousebind.bind_data.allow_when_locked);
            assert!(mousebind.has_on_press)
        });

        Ok(())
    })
}
