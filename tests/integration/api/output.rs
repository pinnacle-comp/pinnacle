use pinnacle::{state::WithState, tag::Tag};
use smithay::{output::Output, utils::Rectangle};

use crate::{
    common::{fixture::Fixture, for_each_api, Lang},
    spawn_lua_blocking,
};

fn set_up() -> (Fixture, Output, Output) {
    let mut fixture = Fixture::new();

    let output1 = fixture.add_output(Rectangle::new((0, 0).into(), (1920, 1080).into()));
    output1.with_state_mut(|state| {
        let tag = Tag::new("1".to_string());
        tag.set_active(true);
        state.add_tags([tag]);
    });
    let output2 = fixture.add_output(Rectangle::new((0, 0).into(), (1920, 1080).into()));
    output2.with_state_mut(|state| {
        let tag = Tag::new("1".to_string());
        tag.set_active(true);
        state.add_tags([tag]);
    });

    fixture.pinnacle().focus_output(&output1);

    fixture
        .runtime_handle()
        .block_on(pinnacle_api::connect())
        .unwrap();

    (fixture, output1, output2)
}

#[test_log::test]
fn output_get_all() {
    let (mut fixture, output1, output2) = set_up();

    fixture.spawn_blocking({
        let output1_name = output1.name();
        let output2_name = output2.name();
        move || {
            let outputs = pinnacle_api::output::get_all().collect::<Vec<_>>();
            assert_eq!(outputs.len(), 2);
            assert_eq!(outputs[0].name(), output1_name);
            assert_eq!(outputs[1].name(), output2_name);
        }
    });

    let output1_name = output1.name();
    let output2_name = output2.name();
    spawn_lua_blocking! {
        fixture,
        local outputs = Output.get_all()
        assert(#outputs == 2)
        assert(outputs[1].name == $output1_name)
        assert(outputs[2].name == $output2_name)
    }
}

#[test_log::test]
fn output_get_all_enabled() {
    let (mut fixture, output1, output2) = set_up();
    fixture.pinnacle().set_output_enabled(&output2, false);

    fixture.spawn_blocking({
        let output1_name = output1.name();
        move || {
            let outputs = pinnacle_api::output::get_all_enabled().collect::<Vec<_>>();
            assert_eq!(outputs.len(), 1);
            assert_eq!(outputs[0].name(), output1_name);
        }
    });

    let output1_name = output1.name();
    spawn_lua_blocking! {
        fixture,
        local outputs = Output.get_all_enabled()
        assert(#outputs == 1)
        assert(outputs[1].name == $output1_name)
    }
}

#[test_log::test]
fn output_get_by_name() {
    let (mut fixture, output1, _) = set_up();

    fixture.spawn_blocking({
        let output1_name = output1.name();
        move || {
            let output = pinnacle_api::output::get_by_name(&output1_name).unwrap();
            assert_eq!(output.name(), output1_name);
        }
    });

    let output1_name = output1.name();
    spawn_lua_blocking! {
        fixture,
        local output = Output.get_by_name($output1_name)
        assert(output.name == $output1_name)
    }
}

#[test_log::test]
fn output_get_focused() {
    let (mut fixture, output1, _) = set_up();

    fixture.spawn_blocking({
        let output1_name = output1.name();
        move || {
            let output = pinnacle_api::output::get_focused().unwrap();
            assert_eq!(output.name(), output1_name);
        }
    });

    let output1_name = output1.name();
    spawn_lua_blocking! {
        fixture,
        local output = Output.get_focused()
        assert(output.name == $output1_name)
    }
}

#[test_log::test]
fn output_handle_set_loc() {
    for_each_api(|lang| {
        let (mut fixture, output, _) = set_up();

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .set_loc(500, -250);
            }),
            Lang::Lua => fixture.spawn_lua_blocking("Output.get_focused():set_loc(500, -250)"),
        }

        assert_eq!(output.current_location(), (500, -250).into());
    });
}

#[test_log::test]
fn output_handle_set_loc_adj_to() {
    for_each_api(|lang| {
        let (mut fixture, output1, output2) = set_up();

        // Originally op1 is left of op2

        let output1_name = output1.name();
        let output2_name = output2.name();

        match lang {
            Lang::Rust => {
                fixture.spawn_blocking(move || {
                    let op1 = pinnacle_api::output::get_by_name(&output1_name).unwrap();
                    let op2 = pinnacle_api::output::get_by_name(&output2_name).unwrap();
                    op2.set_loc_adj_to(&op1, pinnacle_api::output::Alignment::BottomAlignCenter);
                });
            }
            Lang::Lua => {
                spawn_lua_blocking! {
                    fixture,
                    local op1 = Output.get_by_name($output1_name)
                    local op2 = Output.get_by_name($output2_name)
                    op2:set_loc_adj_to(op1, "bottom_align_center")
                }
            }
        }

        let op1_geo = fixture.pinnacle().space.output_geometry(&output1).unwrap();

        let op2_target_loc = (op1_geo.loc.x, op1_geo.loc.y + op1_geo.size.h).into();

        assert_eq!(output2.current_location(), op2_target_loc);
    });
}

#[test_log::test]
fn output_handle_set_mode() {
    for_each_api(|lang| {
        let (mut fixture, output, _) = set_up();

        let old_mode = output.current_mode().unwrap();

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .set_mode(800, 600, 75000);
            }),
            Lang::Lua => {
                fixture.spawn_lua_blocking("Output.get_focused():set_mode(800, 600, 75000)");
            }
        }

        assert_eq!(output.current_mode().unwrap(), old_mode);

        let new_mode = smithay::output::Mode {
            size: (800, 600).into(),
            refresh: 75000,
        };
        output.add_mode(new_mode);
        // FIXME: this exists because swww was buggy,
        // recheck and dedup
        output.with_state_mut(|state| {
            state.modes.push(new_mode);
        });

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .set_mode(800, 600, 75000);
            }),
            Lang::Lua => {
                fixture.spawn_lua_blocking("Output.get_focused():set_mode(800, 600, 75000)");
            }
        }

        assert_eq!(output.current_mode().unwrap(), new_mode);
    });
}

#[test_log::test]
fn output_handle_set_custom_mode() {
    for_each_api(|lang| {
        let (mut fixture, output, _) = set_up();

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .set_custom_mode(800, 600, 75000);
            }),
            Lang::Lua => {
                fixture.spawn_lua_blocking("Output.get_focused():set_custom_mode(800, 600, 75000)");
            }
        }

        let new_mode = smithay::output::Mode {
            size: (800, 600).into(),
            refresh: 75000,
        };

        assert_eq!(output.current_mode().unwrap(), new_mode);

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .set_custom_mode(801, 601, None);
            }),
            Lang::Lua => {
                fixture.spawn_lua_blocking("Output.get_focused():set_custom_mode(801, 601)");
            }
        }

        let new_mode = smithay::output::Mode {
            size: (801, 601).into(),
            refresh: 60000,
        };

        assert_eq!(output.current_mode().unwrap(), new_mode);
    });
}

#[test_log::test]
fn output_handle_set_modeline() {
    for_each_api(|lang| {
        let (mut fixture, output, _) = set_up();

        let modeline = "48.91 800 840 920 1040 600 601 604 627 -HSync +Vsync";

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .set_modeline(modeline.parse().unwrap());
            }),
            Lang::Lua => {
                spawn_lua_blocking! {
                    fixture,
                    Output.get_focused():set_modeline($modeline)
                }
            }
        }

        let new_mode = smithay::output::Mode {
            size: (800, 600).into(),
            refresh: 75006,
        };

        assert_eq!(output.current_mode().unwrap(), new_mode);
    });
}

#[test_log::test]
fn output_handle_set_scale() {
    for_each_api(|lang| {
        let (mut fixture, output, _) = set_up();

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                pinnacle_api::output::get_focused().unwrap().set_scale(1.5);
            }),
            Lang::Lua => {
                spawn_lua_blocking! {
                    fixture,
                    Output.get_focused():set_scale(1.5)
                }
            }
        }

        assert_eq!(output.current_scale().fractional_scale(), 1.5);
    });
}

#[test_log::test]
fn output_handle_change_scale() {
    for_each_api(|lang| {
        let (mut fixture, output, _) = set_up();

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .change_scale(0.25);
            }),
            Lang::Lua => {
                spawn_lua_blocking! {
                    fixture,
                    Output.get_focused():change_scale(0.25)
                }
            }
        }

        assert_eq!(output.current_scale().fractional_scale(), 1.25);

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .change_scale(-0.5);
            }),
            Lang::Lua => {
                spawn_lua_blocking! {
                    fixture,
                    Output.get_focused():change_scale(-0.5)
                }
            }
        }

        assert_eq!(output.current_scale().fractional_scale(), 0.75);
    });
}

#[test_log::test]
fn output_handle_set_transform() {
    for_each_api(|lang| {
        let (mut fixture, output, _) = set_up();

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .set_transform(pinnacle_api::output::Transform::Flipped90);
            }),
            Lang::Lua => {
                spawn_lua_blocking! {
                    fixture,
                    Output.get_focused():set_transform("flipped_90")
                }
            }
        }

        assert_eq!(
            output.current_transform(),
            smithay::utils::Transform::Flipped90
        );
    });
}

#[test_log::test]
fn output_handle_set_powered() {
    for_each_api(|lang| {
        let (mut fixture, output, _) = set_up();

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .set_powered(false);
            }),
            Lang::Lua => {
                spawn_lua_blocking! {
                    fixture,
                    Output.get_focused():set_powered(false)
                }
            }
        }

        assert!(!output.with_state(|state| state.powered));

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .set_powered(true);
            }),
            Lang::Lua => {
                spawn_lua_blocking! {
                    fixture,
                    Output.get_focused():set_powered(true)
                }
            }
        }

        assert!(output.with_state(|state| state.powered));
    });
}

#[test_log::test]
fn output_handle_toggle_powered() {
    for_each_api(|lang| {
        let (mut fixture, output, _) = set_up();

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .toggle_powered();
            }),
            Lang::Lua => {
                spawn_lua_blocking! {
                    fixture,
                    Output.get_focused():toggle_powered()
                }
            }
        }

        assert!(!output.with_state(|state| state.powered));

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                pinnacle_api::output::get_focused()
                    .unwrap()
                    .toggle_powered();
            }),
            Lang::Lua => {
                spawn_lua_blocking! {
                    fixture,
                    Output.get_focused():toggle_powered()
                }
            }
        }

        assert!(output.with_state(|state| state.powered));
    });
}

#[test_log::test]
fn output_handle_make() {
    let (mut fixture, output, _) = set_up();

    fixture.spawn_blocking({
        let make = output.physical_properties().make;
        move || {
            assert_eq!(pinnacle_api::output::get_focused().unwrap().make(), make);
        }
    });

    let make = output.physical_properties().make;
    spawn_lua_blocking! {
        fixture,
        assert(Output.get_focused():make(), $make)
    }
}

#[test_log::test]
fn output_handle_model() {
    let (mut fixture, output, _) = set_up();

    fixture.spawn_blocking({
        let model = output.physical_properties().model;
        move || {
            assert_eq!(pinnacle_api::output::get_focused().unwrap().model(), model);
        }
    });

    let model = output.physical_properties().model;
    spawn_lua_blocking! {
        fixture,
        assert(Output.get_focused():model(), $model)
    }
}

#[test_log::test]
fn output_handle_serial() {
    let (mut fixture, output, _) = set_up();

    let serial = "this-is-a-serial-138421";

    output.with_state_mut(|state| {
        state.serial = serial.into();
    });

    fixture.spawn_blocking(move || {
        assert_eq!(
            pinnacle_api::output::get_focused().unwrap().serial(),
            serial
        );
    });

    spawn_lua_blocking! {
        fixture,
        assert(Output.get_focused():serial(), $serial)
    }
}

#[test_log::test]
fn output_handle_loc() {
    let (mut fixture, _, output) = set_up();

    let x = output.current_location().x;
    let y = output.current_location().y;

    fixture.spawn_blocking(move || {
        let loc = pinnacle_api::output::get_focused().unwrap().loc().unwrap();
        assert_eq!(loc.x, x);
        assert_eq!(loc.y, y);
    });

    spawn_lua_blocking! {
        fixture,
        local loc = Output.get_focused():loc()
        assert(loc.x, $x)
        assert(loc.y, $y)
    }
}

#[test_log::test]
fn output_handle_logical_size() {
    let (mut fixture, output, _) = set_up();

    let logical_size = fixture
        .pinnacle()
        .space
        .output_geometry(&output)
        .unwrap()
        .size;

    let logical_width = logical_size.w;
    let logical_height = logical_size.h;

    fixture.spawn_blocking(move || {
        let size = pinnacle_api::output::get_focused()
            .unwrap()
            .logical_size()
            .unwrap();
        assert_eq!(size.w, logical_width as u32);
        assert_eq!(size.h, logical_height as u32);
    });

    spawn_lua_blocking! {
        fixture,
        local size = Output.get_focused():logical_size()
        assert(size.width == $logical_width)
        assert(size.height == $logical_height)
    }

    let state = fixture.state();
    state.pinnacle.change_output_state(
        &mut state.backend,
        &output,
        None,
        None,
        Some(smithay::output::Scale::Fractional(2.0)),
        None,
    );

    let logical_size = fixture
        .pinnacle()
        .space
        .output_geometry(&output)
        .unwrap()
        .size;

    let logical_width = logical_size.w;
    let logical_height = logical_size.h;

    fixture.spawn_blocking(move || {
        let size = pinnacle_api::output::get_focused()
            .unwrap()
            .logical_size()
            .unwrap();
        assert_eq!(size.w, logical_width as u32);
        assert_eq!(size.h, logical_height as u32);
    });

    spawn_lua_blocking! {
        fixture,
        local size = Output.get_focused():logical_size()
        assert(size.width == $logical_width)
        assert(size.height == $logical_height)
    }
}

#[test_log::test]
fn output_handle_physical_size() {
    let (mut fixture, output, _) = set_up();

    let physical_size = output.physical_properties().size;
    let physical_width = physical_size.w;
    let physical_height = physical_size.h;

    fixture.spawn_blocking(move || {
        let size = pinnacle_api::output::get_focused().unwrap().physical_size();
        assert_eq!(size.w, physical_width as u32);
        assert_eq!(size.h, physical_height as u32);
    });

    spawn_lua_blocking! {
        fixture,
        local size = Output.get_focused():physical_size()
        assert(size.width == $physical_width)
        assert(size.height == $physical_height)
    }
}

#[test_log::test]
fn output_handle_current_mode() {
    let (mut fixture, output, _) = set_up();

    let mode = output.current_mode().unwrap();
    let mode_width = mode.size.w;
    let mode_height = mode.size.h;
    let mode_refresh = mode.refresh;

    fixture.spawn_blocking(move || {
        let mode = pinnacle_api::output::get_focused()
            .unwrap()
            .current_mode()
            .unwrap();
        assert_eq!(mode.size.w, mode_width as u32);
        assert_eq!(mode.size.h, mode_height as u32);
        assert_eq!(mode.refresh_rate_mhz, mode_refresh as u32);
    });

    spawn_lua_blocking! {
        fixture,
        local mode = Output.get_focused():current_mode()
        assert(mode.width == $mode_width)
        assert(mode.height == $mode_height)
        assert(mode.refresh_rate_mhz == $mode_refresh)
    }
}

#[test_log::test]
fn output_handle_preferred_mode() {
    let (mut fixture, output, _) = set_up();

    let mode = output.preferred_mode().unwrap();
    let mode_width = mode.size.w;
    let mode_height = mode.size.h;
    let mode_refresh = mode.refresh;

    fixture.spawn_blocking(move || {
        let mode = pinnacle_api::output::get_focused()
            .unwrap()
            .preferred_mode()
            .unwrap();
        assert_eq!(mode.size.w, mode_width as u32);
        assert_eq!(mode.size.h, mode_height as u32);
        assert_eq!(mode.refresh_rate_mhz, mode_refresh as u32);
    });

    spawn_lua_blocking! {
        fixture,
        local mode = Output.get_focused():preferred_mode()
        assert(mode.width == $mode_width)
        assert(mode.height == $mode_height)
        assert(mode.refresh_rate_mhz == $mode_refresh)
    }
}

#[test_log::test]
fn output_handle_modes() {
    let (mut fixture, output, _) = set_up();

    let first_mode = output.current_mode().unwrap();
    let first_mode_width = first_mode.size.w;
    let first_mode_height = first_mode.size.h;
    let first_mode_refresh = first_mode.refresh;

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
        output.add_mode(mode);
        output.with_state_mut(|state| state.modes.push(mode));
    }

    fixture.spawn_blocking(move || {
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
    });

    spawn_lua_blocking! {
        fixture,
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

#[test_log::test]
fn output_handle_focused() {
    let (mut fixture, output1, output2) = set_up();

    fixture.spawn_blocking({
        let output1_name = output1.name();
        let output2_name = output2.name();
        move || {
            let output1_focused = pinnacle_api::output::get_by_name(output1_name)
                .unwrap()
                .focused();
            let output2_focused = pinnacle_api::output::get_by_name(output2_name)
                .unwrap()
                .focused();

            assert!(output1_focused);
            assert!(!output2_focused);
        }
    });

    let output1_name = output1.name();
    let output2_name = output2.name();
    spawn_lua_blocking! {
        fixture,
        local output1_focused = Output.get_by_name($output1_name):focused()
        local output2_focused = Output.get_by_name($output2_name):focused()
        assert(output1_focused)
        assert(not output2_focused)
    }
}

#[test_log::test]
fn output_handle_tags() {
    let (mut fixture, _, _) = set_up();

    fixture.spawn_blocking(move || {
        let tags = pinnacle_api::output::get_focused()
            .unwrap()
            .tags()
            .collect::<Vec<_>>();

        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name(), "1");
    });

    spawn_lua_blocking! {
        fixture,
        local tags = Output.get_focused():tags()
        assert(#tags == 1)
        assert(tags[1]:name() == "1")
    }
}

#[test_log::test]
fn output_handle_scale() {
    let (mut fixture, output, _) = set_up();

    let state = fixture.state();
    state.pinnacle.change_output_state(
        &mut state.backend,
        &output,
        None,
        None,
        Some(smithay::output::Scale::Fractional(1.75)),
        None,
    );

    fixture.spawn_blocking(move || {
        let scale = pinnacle_api::output::get_focused().unwrap().scale();
        assert_eq!(scale, 1.75);
    });

    spawn_lua_blocking! {
        fixture,
        local scale = Output.get_focused():scale()
        assert(scale == 1.75)
    }
}

#[test_log::test]
fn output_handle_transform() {
    let (mut fixture, output, _) = set_up();

    let state = fixture.state();
    state.pinnacle.change_output_state(
        &mut state.backend,
        &output,
        None,
        Some(smithay::utils::Transform::Flipped180),
        None,
        None,
    );

    fixture.spawn_blocking(move || {
        let transform = pinnacle_api::output::get_focused().unwrap().transform();
        assert_eq!(transform, pinnacle_api::output::Transform::Flipped180);
    });

    spawn_lua_blocking! {
        fixture,
        local transform = Output.get_focused():transform()
        assert(transform == "flipped_180")
    }
}

#[test_log::test]
fn output_handle_enabled() {
    let (mut fixture, output1, output2) = set_up();

    fixture.pinnacle().set_output_enabled(&output2, false);

    fixture.spawn_blocking({
        let output1_name = output1.name();
        let output2_name = output2.name();
        move || {
            let output1_enabled = pinnacle_api::output::get_by_name(output1_name)
                .unwrap()
                .enabled();
            let output2_enabled = pinnacle_api::output::get_by_name(output2_name)
                .unwrap()
                .enabled();

            assert!(output1_enabled);
            assert!(!output2_enabled);
        }
    });

    let output1_name = output1.name();
    let output2_name = output2.name();
    spawn_lua_blocking! {
        fixture,
        local output1_enabled = Output.get_by_name($output1_name):enabled()
        local output2_enabled = Output.get_by_name($output2_name):enabled()

        assert(output1_enabled)
        assert(not output2_enabled)
    }
}

#[test_log::test]
fn output_handle_powered() {
    let (mut fixture, output1, output2) = set_up();

    fixture.state().set_output_powered(&output2, false);

    fixture.spawn_blocking({
        let output1_name = output1.name();
        let output2_name = output2.name();
        move || {
            let output1_powered = pinnacle_api::output::get_by_name(output1_name)
                .unwrap()
                .powered();
            let output2_powered = pinnacle_api::output::get_by_name(output2_name)
                .unwrap()
                .powered();

            assert!(output1_powered);
            assert!(!output2_powered);
        }
    });

    let output1_name = output1.name();
    let output2_name = output2.name();
    spawn_lua_blocking! {
        fixture,
        local output1_powered = Output.get_by_name($output1_name):powered()
        local output2_powered = Output.get_by_name($output2_name):powered()

        assert(output1_powered)
        assert(not output2_powered)
    }
}

#[test_log::test]
fn output_handle_focus() {
    for_each_api(|lang| {
        let (mut fixture, output1, output2) = set_up();

        assert_eq!(fixture.pinnacle().focused_output().unwrap(), &output1);

        let output2_name = output2.name();
        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                pinnacle_api::output::get_by_name(output2_name)
                    .unwrap()
                    .focus();
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Output.get_by_name($output2_name):focus()
            },
        }

        assert_eq!(fixture.pinnacle().focused_output().unwrap(), &output2);
    });
}

// TODO: for_each_output
// TODO: connect_signal
// TODO: keyboard_focus_stack
// TODO: keyboard_focus_stack_visible
