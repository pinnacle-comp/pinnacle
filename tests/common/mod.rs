use std::{panic::UnwindSafe, time::Duration};

use pinnacle::{backend::dummy::setup_dummy, state::State};
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

pub fn test_api(
    test: impl FnOnce(Sender<Box<dyn FnOnce(&mut State) + Send>>) + Send + UnwindSafe + 'static,
) -> anyhow::Result<()> {
    let (mut state, mut event_loop) = setup_dummy(true, None)?;

    let (sender, recv) = calloop::channel::channel::<Box<dyn FnOnce(&mut State) + Send>>();

    event_loop
        .handle()
        .insert_source(recv, |event, _, state| match event {
            Event::Msg(f) => f(state),
            Event::Closed => (),
        })
        .map_err(|_| anyhow::anyhow!("failed to insert source"))?;

    let tempdir = tempfile::tempdir()?;

    state.start_grpc_server(tempdir.path())?;

    let loop_signal = event_loop.get_signal();

    let join_handle = tokio::task::spawn_blocking(move || {
        test(sender);
        loop_signal.stop();
    });

    event_loop.run(None, &mut state, |state| {
        state.fixup_z_layering();
        state.space.refresh();
        state.popup_manager.cleanup();

        state
            .display_handle
            .flush_clients()
            .expect("failed to flush client buffers");

        // TODO: couple these or something, this is really error-prone
        assert_eq!(
            state.windows.len(),
            state.z_index_stack.len(),
            "Length of `windows` and `z_index_stack` are different. \
                    If you see this, report it to the developer."
        );
    })?;

    if let Err(err) = tokio::task::block_in_place(|| {
        let handle = tokio::runtime::Handle::current();
        handle.block_on(join_handle)
    }) {
        panic!("{err:?}");
    }

    Ok(())
}

pub fn output_for_name(state: &State, name: &str) -> Output {
    state
        .space
        .outputs()
        .find(|op| op.name() == name)
        .unwrap()
        .clone()
}
