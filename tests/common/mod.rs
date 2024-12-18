use std::{panic::UnwindSafe, path::PathBuf, sync::Mutex, time::Duration};

use anyhow::anyhow;
use pinnacle::{state::State, tag::TagId};
use smithay::{
    output::Output,
    reexports::calloop::{
        self,
        channel::{Event, Sender},
        EventLoop,
    },
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

pub fn sleep_millis(millis: u64) {
    std::thread::sleep(Duration::from_millis(millis));
}

static MUTEX: Mutex<()> = Mutex::new(());

pub fn test_api<F>(test: F) -> anyhow::Result<()>
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

    let (sender, recv) = calloop::channel::channel::<Box<dyn FnOnce(&mut State) + Send>>();

    event_loop
        .handle()
        .insert_source(recv, |event, _, state| match event {
            Event::Msg(f) => f(state),
            Event::Closed => (),
        })
        .map_err(|_| anyhow::anyhow!("failed to insert source"))?;

    let tempdir = tempfile::tempdir()?;

    TagId::reset();

    state.pinnacle.start_grpc_server(tempdir.path())?;

    let loop_signal = event_loop.get_signal();

    let join_handle = std::thread::spawn(move || -> anyhow::Result<()> {
        let res = test(sender);
        loop_signal.stop();
        res
    });

    event_loop.run(None, &mut state, |state| {
        state.on_event_loop_cycle_completion();
    })?;

    join_handle.join().map_err(|_| anyhow!("thread panicked"))?
}

pub fn output_for_name(state: &State, name: &str) -> Output {
    state
        .pinnacle
        .space
        .outputs()
        .find(|op| op.name() == name)
        .unwrap()
        .clone()
}
