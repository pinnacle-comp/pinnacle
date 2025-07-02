use crate::{
    common::{fixture::Fixture, for_each_api, Lang},
    spawn_lua_blocking,
};

fn set_up() -> Fixture {
    let fixture = Fixture::new();

    fixture
        .runtime_handle()
        .block_on(pinnacle_api::connect())
        .unwrap();

    fixture
}

#[test_log::test]
fn pinnacle_set_last_error() {
    for_each_api(|lang| {
        let mut fixture = set_up();

        let error = "wibbly wobbly timey wimey";

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                pinnacle_api::pinnacle::set_last_error(error);
            }),
            Lang::Lua => {
                spawn_lua_blocking! {
                    fixture,
                    Pinnacle.set_last_error($error)
                }
            }
        }

        assert_eq!(fixture.pinnacle().config.last_error.as_deref(), Some(error));
    });
}

#[test_log::test]
fn pinnacle_take_last_error() {
    for_each_api(|lang| {
        let mut fixture = set_up();

        let error = "i've never watched doctor who";

        fixture.pinnacle().config.last_error = Some(error.to_string());

        match lang {
            Lang::Rust => fixture.spawn_blocking(move || {
                let err = pinnacle_api::pinnacle::take_last_error();
                assert_eq!(err.as_deref(), Some(error));

                let err = pinnacle_api::pinnacle::take_last_error();
                assert_eq!(err, None);
            }),
            Lang::Lua => {
                spawn_lua_blocking! {
                    fixture,
                    local error = Pinnacle.take_last_error()
                    assert(error == $error)

                    local error = Pinnacle.take_last_error()
                    assert(error == nil)
                }
            }
        }
    });
}
