use std::{
    fs::File,
    io::{Read, Write},
    time::Duration,
};

use pinnacle::{state::WithState, tag::Tag};
use pinnacle_api::layout::{LayoutGenerator, generators::MasterStack};
use smithay::{output::Output, utils::Rectangle};

use crate::{
    common::{Lang, fixture::Fixture, for_each_api},
    spawn_lua_blocking,
};

fn set_up() -> (Fixture, Output) {
    let mut fixture = Fixture::new_with_socket();

    let output = fixture.add_output(Rectangle::new((0, 0).into(), (1920, 1080).into()));
    output.with_state_mut(|state| {
        let tag = Tag::new("1".to_string());
        tag.set_active(true);
        state.add_tags([tag]);
    });
    fixture.pinnacle().focus_output(&output);

    fixture
        .runtime_handle()
        .block_on(pinnacle_api::connect())
        .unwrap();

    fixture.spawn_blocking(|| {
        pinnacle_api::layout::manage(|args| pinnacle_api::layout::LayoutResponse {
            root_node: MasterStack::default().layout(args.window_count),
            tree_id: 0,
        });
    });

    (fixture, output)
}

#[test_log::test]
fn process_spawn() {
    for_each_api(|lang| {
        let (mut fixture, ..) = set_up();
        let handle = fixture.runtime_handle();
        let _guard = handle.enter();

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::process::Command::new("alacritty")
                    .args(["-o", "general.ipc_socket=false"])
                    .spawn()
                    .unwrap();
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                assert(Process.spawn("alacritty", "-o", "general.ipc_socket=false"))
            },
        }

        fixture.dispatch_for(Duration::from_secs(1));

        assert_eq!(fixture.pinnacle().windows.len(), 1);
        assert_eq!(
            fixture.pinnacle().windows[0].class().as_deref(),
            Some("Alacritty")
        );

        fixture.dispatch_until(|fixture| {
            for win in fixture.pinnacle().windows.iter() {
                win.close();
            }
            fixture.pinnacle().windows.is_empty()
        });
    });
}

#[test_log::test]
fn process_spawn_unique() {
    for_each_api(|lang| {
        let (mut fixture, ..) = set_up();
        let handle = fixture.runtime_handle();
        let _guard = handle.enter();

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::process::Command::new("alacritty")
                    .args(["-o", "general.ipc_socket=false"])
                    .unique()
                    .spawn()
                    .unwrap();
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                assert(Process.spawn_unique("alacritty", "-o", "general.ipc_socket=false"))
            },
        }

        fixture.dispatch_for(Duration::from_secs(1));

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                assert!(
                    pinnacle_api::process::Command::new("alacritty")
                        .args(["-o", "general.ipc_socket=false"])
                        .unique()
                        .spawn()
                        .is_none()
                )
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                assert(not Process.spawn_unique("alacritty", "-o", "general.ipc_socket=false"))
            },
        }

        fixture.dispatch_for(Duration::from_secs(1));

        assert_eq!(fixture.pinnacle().windows.len(), 1);
        assert_eq!(
            fixture.pinnacle().windows[0].class().as_deref(),
            Some("Alacritty")
        );

        // FIXME: shell commands may be multiple args and those args
        // cause the dedup logic to not work, maybe split up to the
        // first whitespace to check instead
        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                assert!(
                    pinnacle_api::process::Command::with_shell(["bash", "-c"], "alacritty")
                        .unique()
                        .spawn()
                        .is_none()
                )
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                assert(not Process.command({
                    cmd = "alacritty",
                    shell_cmd = { "bash", "-c" },
                    unique = true,
                }):spawn())
            },
        }

        fixture.dispatch_for(Duration::from_secs(1));

        assert_eq!(fixture.pinnacle().windows.len(), 1);
        assert_eq!(
            fixture.pinnacle().windows[0].class().as_deref(),
            Some("Alacritty")
        );

        fixture.dispatch_until(|fixture| {
            for win in fixture.pinnacle().windows.iter() {
                win.close();
            }
            fixture.pinnacle().windows.is_empty()
        });
    });
}

#[test_log::test]
fn process_spawn_once() {
    for_each_api(|lang| {
        let (mut fixture, ..) = set_up();
        let handle = fixture.runtime_handle();
        let _guard = handle.enter();

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::process::Command::new("alacritty")
                    .args(["-o", "general.ipc_socket=false"])
                    .once()
                    .spawn()
                    .unwrap();
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                assert(Process.spawn_once("alacritty", "-o", "general.ipc_socket=false"))
            },
        }

        fixture.dispatch_for(Duration::from_secs(1));

        assert_eq!(fixture.pinnacle().windows.len(), 1);
        assert_eq!(
            fixture.pinnacle().windows[0].class().as_deref(),
            Some("Alacritty")
        );

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::window::get_focused().unwrap().close();
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Window.get_focused():close()
            },
        }

        fixture.dispatch_for(Duration::from_secs(1));

        assert_eq!(fixture.pinnacle().windows.len(), 0);

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                assert!(
                    pinnacle_api::process::Command::new("alacritty")
                        .args(["-o", "general.ipc_socket=false"])
                        .once()
                        .spawn()
                        .is_none()
                );
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                assert(not Process.spawn_once("alacritty", "-o", "general.ipc_socket=false"))
            },
        }

        fixture.dispatch_for(Duration::from_secs(1));

        assert_eq!(fixture.pinnacle().windows.len(), 0);

        fixture.dispatch_until(|fixture| {
            for win in fixture.pinnacle().windows.iter() {
                win.close();
            }
            fixture.pinnacle().windows.is_empty()
        });
    });
}

#[test_log::test]
fn process_stdio() {
    let (mut fixture, ..) = set_up();
    let handle = fixture.runtime_handle();
    let _guard = handle.enter();

    fixture.spawn_blocking(|| {
        // Turning the tokio stuff into files to sidestep async

        let mut child =
            pinnacle_api::process::Command::with_shell(["bash", "-c"], "echo 'hello there'")
                .pipe_stdout()
                .spawn()
                .unwrap();
        let mut out = String::new();
        let mut stdout: File = child.stdout.take().unwrap().into_owned_fd().unwrap().into();
        stdout.read_to_string(&mut out).unwrap();
        assert_eq!(out, "hello there\n");

        let mut child =
            pinnacle_api::process::Command::with_shell(["bash", "-c"], "echo 'hello there' 1>&2")
                .pipe_stderr()
                .spawn()
                .unwrap();
        let mut err = String::new();
        let mut stderr: File = child.stderr.take().unwrap().into_owned_fd().unwrap().into();
        stderr.read_to_string(&mut err).unwrap();
        assert_eq!(err, "hello there\n");

        let mut child = pinnacle_api::process::Command::new("cat")
            .pipe_stdin()
            .pipe_stdout()
            .spawn()
            .unwrap();
        let mut stdin: File = child.stdin.take().unwrap().into_owned_fd().unwrap().into();
        stdin.write_all(b"sussus amogus").unwrap();
        drop(stdin);
        let mut out = String::new();
        let mut stdout: File = child.stdout.take().unwrap().into_owned_fd().unwrap().into();
        stdout.read_to_string(&mut out).unwrap();
        assert_eq!(out, "sussus amogus");
    });

    spawn_lua_blocking! {
        fixture,

        local child = Process.command({
            cmd = "echo 'hello there'",
            shell_cmd = { "bash", "-c" },
            pipe_stdout = true,
        }):spawn()
        local out = child.stdout:read()
        assert(out == "hello there")

        local child = Process.command({
            cmd = "echo 'hello there' 1>&2",
            shell_cmd = { "bash", "-c" },
            pipe_stderr = true,
        }):spawn()
        local err = child.stderr:read()
        assert(err == "hello there")

        local child = Process.command({
            cmd = "cat",
            pipe_stdin = true,
            pipe_stdout = true,
        }):spawn()
        child.stdin:write("sussus amogus")
        child.stdin:flush()
        child.stdin:close()
        local out = child.stdout:read("*a")
        assert(out == "sussus amogus")
    }
}

#[test_log::test]
fn process_stdio_with_no_pipes_has_no_child_stdio() {
    let (mut fixture, ..) = set_up();
    let handle = fixture.runtime_handle();
    let _guard = handle.enter();

    fixture.spawn_blocking(|| {
        let child =
            pinnacle_api::process::Command::with_shell(["bash", "-c"], "echo 'hello there'")
                .spawn()
                .unwrap();

        assert!(child.stdout.is_none());
    });

    fixture.dispatch_for(Duration::from_secs(1));

    spawn_lua_blocking! {
        fixture,

        local child = Process.command({
            cmd = "echo 'hello there'",
            shell_cmd = { "bash", "-c" },
        }):spawn()
        assert(child.stdout == nil)
    }
}

#[test_log::test]
fn process_set_env() {
    for_each_api(|lang| {
        let (mut fixture, ..) = set_up();
        let handle = fixture.runtime_handle();
        let _guard = handle.enter();

        match lang {
            Lang::Rust => fixture.spawn_blocking(|| {
                pinnacle_api::process::set_env("SILK", "SONG");
            }),
            Lang::Lua => spawn_lua_blocking! {
                fixture,
                Process.set_env("SILK", "SONG")
            },
        }

        assert_eq!(
            fixture.pinnacle().config.process_envs.get("SILK"),
            Some(&"SONG".to_string())
        );
    });
}
