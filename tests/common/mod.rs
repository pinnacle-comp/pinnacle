use std::{panic::UnwindSafe, time::Duration};

use anyhow::anyhow;
use pinnacle::{backend::dummy::setup_dummy, config::StartupSettings, state::State};
use smithay::{
    output::Output,
    reexports::calloop::{
        self,
        channel::{Event, Sender},
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

pub fn test_api<F>(test: F) -> anyhow::Result<()>
where
    F: FnOnce(Sender<Box<dyn FnOnce(&mut State) + Send>>) -> anyhow::Result<()>
        + Send
        + UnwindSafe
        + 'static,
{
    const NO_XWAYLAND: bool = true;

    let (mut state, mut event_loop) = setup_dummy(StartupSettings {
        no_config: true,
        config_dir: None,
        no_xwayland: NO_XWAYLAND,
    })?;

    let (sender, recv) = calloop::channel::channel::<Box<dyn FnOnce(&mut State) + Send>>();

    event_loop
        .handle()
        .insert_source(recv, |event, _, state| match event {
            Event::Msg(f) => f(state),
            Event::Closed => (),
        })
        .map_err(|_| anyhow::anyhow!("failed to insert source"))?;

    let tempdir = tempfile::tempdir()?;

    state.pinnacle.start_grpc_server(tempdir.path())?;

    let loop_signal = event_loop.get_signal();

    if !NO_XWAYLAND {
        while state.pinnacle.xdisplay.is_none() {
            event_loop
                .dispatch(None, &mut state)
                .expect("dispatch failed");
            state.on_event_loop_cycle_completion();
        }
    }

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
