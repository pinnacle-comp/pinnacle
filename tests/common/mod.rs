pub mod grpc;
pub mod lua;
pub mod rust;

use std::{panic::UnwindSafe, path::PathBuf, sync::Mutex, time::Duration};

use anyhow::anyhow;
use pinnacle::{state::State, tag::TagId, window::window_state::WindowId};
use smithay::reexports::calloop::{
    self,
    channel::{Event, Sender},
    EventLoop,
};

#[allow(clippy::type_complexity)]
pub fn with_state(
    sender: &Sender<Box<dyn FnOnce(&mut State) + Send>>,
    with_state: impl FnOnce(&mut State) + Send + 'static,
) {
    sender.send(Box::new(with_state)).unwrap();
}

pub fn sleep_secs(secs: u64) {
    std::thread::sleep(Duration::from_secs(secs));
}

// This is actually used
#[allow(dead_code)]
pub fn sleep_millis(millis: u64) {
    std::thread::sleep(Duration::from_millis(millis));
}

static MUTEX: Mutex<()> = Mutex::new(());

#[tokio::main]
pub async fn test_api<F>(test: F) -> anyhow::Result<()>
where
    F: FnOnce(Sender<Box<dyn FnOnce(&mut State) + Send>>) -> anyhow::Result<()>
        + Send
        + UnwindSafe
        + 'static,
{
    let _guard = match MUTEX.lock() {
        Ok(guard) => guard,
        Err(err) => {
            MUTEX.clear_poison();
            err.into_inner()
        }
    };

    let mut event_loop = EventLoop::<State>::try_new()?;
    let mut state = State::new(
        pinnacle::cli::Backend::Dummy,
        event_loop.handle(),
        event_loop.get_signal(),
        PathBuf::from(""),
        None,
    )?;

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

    let join_handle = std::thread::spawn(move || -> anyhow::Result<()> {
        let res = test(sender.clone());
        with_state(&sender, teardown);
        loop_signal.stop();
        res
    });

    event_loop.run(Duration::from_secs(1), &mut state, |state| {
        state.on_event_loop_cycle_completion();
    })?;

    join_handle.join().map_err(|_| anyhow!("thread panicked"))?
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
