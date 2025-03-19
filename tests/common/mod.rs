#![allow(unused)]

pub mod grpc;
pub mod lua;
pub mod rust;

use std::{
    panic::UnwindSafe,
    path::PathBuf,
    sync::{LazyLock, Mutex},
    time::Duration,
};

use anyhow::anyhow;
use pinnacle::{state::State, tag::TagId, window::window_state::WindowId};
use smithay::{
    reexports::calloop::{
        self,
        channel::{Event, Sender},
        EventLoop,
    },
    utils::{Logical, Physical, Point, Size, Transform},
};

pub const PINNACLE_1_OUTPUT_NAME: &str = "pinnacle-1";
pub const PINNACLE_1_OUTPUT_MAKE: &str = "Pinnacle";
pub const PINNACLE_1_OUTPUT_MODEL: &str = "Dummy Output";
pub static PINNACLE_1_OUTPUT_LOC: LazyLock<Point<i32, Logical>> = LazyLock::new(Point::default);
pub static PINNACLE_1_OUTPUT_SIZE: LazyLock<Size<i32, Physical>> =
    LazyLock::new(|| Size::from((1920, 1080)));
pub const PINNACLE_1_OUTPUT_REFRESH: i32 = 60000;
pub const PINNACLE_1_OUTPUT_SCALE: f64 = 1.0;
pub const PINNACLE_1_OUTPUT_TRANSFORM: Transform = Transform::Normal;

pub struct StateSender {
    sender: Sender<Box<dyn FnOnce(&mut State) + Send>>,
}

impl StateSender {
    pub fn with_state(&self, with_state: impl FnOnce(&mut State) + Send + 'static) {
        self.sender.send(Box::new(with_state)).unwrap();
    }
}

pub fn sleep_secs(secs: u64) {
    std::thread::sleep(Duration::from_secs(secs));
}

pub fn sleep_millis(millis: u64) {
    std::thread::sleep(Duration::from_millis(millis));
}

static MUTEX: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Lua,
    Rust,
}

#[tokio::main]
pub async fn test_api(test: fn(&StateSender, Lang) -> anyhow::Result<()>) -> anyhow::Result<()> {
    let _guard = match MUTEX.lock() {
        Ok(guard) => guard,
        Err(err) => {
            MUTEX.clear_poison();
            err.into_inner()
        }
    };

    test_with_lang(test, Lang::Lua)?;
    test_with_lang(test, Lang::Rust)?;

    Ok(())
}

fn test_with_lang(
    test: fn(&StateSender, Lang) -> anyhow::Result<()>,
    lang: Lang,
) -> anyhow::Result<()> {
    let mut event_loop = EventLoop::<State>::try_new()?;
    let mut state = State::new(
        pinnacle::cli::Backend::Dummy,
        event_loop.handle(),
        event_loop.get_signal(),
        PathBuf::from(""),
        None,
    )?;

    state.pinnacle.new_output(
        PINNACLE_1_OUTPUT_NAME,
        PINNACLE_1_OUTPUT_MAKE,
        PINNACLE_1_OUTPUT_MODEL,
        *PINNACLE_1_OUTPUT_LOC,
        *PINNACLE_1_OUTPUT_SIZE,
        PINNACLE_1_OUTPUT_REFRESH,
        PINNACLE_1_OUTPUT_SCALE,
        PINNACLE_1_OUTPUT_TRANSFORM,
    );

    std::env::set_var("WAYLAND_DISPLAY", &state.pinnacle.socket_name);

    let (sender, recv) = calloop::channel::channel::<Box<dyn FnOnce(&mut State) + Send>>();

    event_loop
        .handle()
        .insert_source(recv, |event, _, state| match event {
            Event::Msg(f) => f(state),
            Event::Closed => (),
        })
        .map_err(|_| anyhow::anyhow!("failed to insert source"))?;

    let tempdir = tempfile::tempdir()?;

    WindowId::reset();
    TagId::reset();

    state.pinnacle.start_grpc_server(tempdir.path())?;

    let loop_signal = event_loop.get_signal();

    let state_sender = StateSender { sender };

    let join_handle = std::thread::spawn(move || -> anyhow::Result<()> {
        let res = test(&state_sender, lang);
        state_sender.with_state(teardown);
        loop_signal.stop();
        res
    });

    event_loop.run(Duration::from_secs(1), &mut state, |state| {
        state.on_event_loop_cycle_completion();

        for output in state.pinnacle.outputs.keys() {
            for window in state.pinnacle.space.elements_for_output(output) {
                window.send_frame(
                    output,
                    state.pinnacle.clock.now(),
                    Some(Duration::ZERO),
                    |_, _| Some(output.clone()),
                );
            }
        }
    })?;

    join_handle
        .join()
        .map_err(|_| anyhow!("thread panicked"))??;

    let runtime_dir = state
        .pinnacle
        .xdg_base_dirs
        .get_runtime_directory()
        .unwrap();

    let _ = std::fs::remove_file(runtime_dir.join(std::env::var("WAYLAND_DISPLAY").unwrap()));
    let _ =
        std::fs::remove_file(runtime_dir.join(std::env::var("WAYLAND_DISPLAY").unwrap() + ".lock"));

    Ok(())
}

fn teardown(state: &mut State) {
    for win in state.pinnacle.windows.iter() {
        win.close();
    }
}

#[macro_export]
macro_rules! catch {
    ($f:expr) => {{
        let _ = std::panic::catch_unwind(|| {
            let _ = $f;
        });
    }};
}
