use std::{
    io::Write,
    process::{Command, Stdio},
    time::Duration,
};

use pinnacle::{backend::dummy::setup_dummy, state::State};
use smithay::reexports::calloop::{
    self,
    channel::{Event, Sender},
};

use test_log::test;

fn run_lua(ident: &str, code: &str) {
    let code = format!(r#"require("pinnacle").setup(function({ident}) {code} end)"#);

    let mut child = Command::new("lua").stdin(Stdio::piped()).spawn().unwrap();

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow::anyhow!("failed to open child stdin"))
        .unwrap();

    stdin.write_all(code.as_bytes()).unwrap();

    drop(stdin);

    child.wait().unwrap();
}

#[allow(clippy::type_complexity)]
fn assert(
    sender: &Sender<Box<dyn FnOnce(&mut State) + Send>>,
    assert: impl FnOnce(&mut State) + Send + 'static,
) {
    sender.send(Box::new(assert)).unwrap();
}

fn sleep_secs(secs: u64) {
    std::thread::sleep(Duration::from_secs(secs));
}

macro_rules! run_lua {
    { |$ident:ident| $($body:tt)* } => {
        run_lua(stringify!($ident), stringify!($($body)*));
    };
}

fn test_lua_api(
    test: impl FnOnce(Sender<Box<dyn FnOnce(&mut State) + Send>>) + Send + 'static,
) -> anyhow::Result<()> {
    let (mut state, mut event_loop) = setup_dummy(true, None)?;

    let (sender, recv) = calloop::channel::channel::<Box<dyn FnOnce(&mut State) + Send>>();

    event_loop
        .handle()
        .insert_source(recv, |event, _, state| match event {
            Event::Msg(f) => f(state),
            Event::Closed => panic!(),
        })
        .map_err(|_| anyhow::anyhow!("failed to insert source"))?;

    let tempdir = tempfile::tempdir()?;

    state.start_grpc_server(tempdir.path())?;

    std::thread::spawn(move || test(sender));

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

    Ok(())
}

#[tokio::main]
#[test]
async fn window_count_with_tag_is_correct() -> anyhow::Result<()> {
    test_lua_api(|sender| {
        run_lua! { |Pinnacle|
            Pinnacle.tag.add(Pinnacle.output.get_all()[1], "1")
            Pinnacle.process.spawn("foot")
        }

        sleep_secs(1);

        assert(&sender, |state| assert_eq!(state.windows.len(), 1));

        sleep_secs(1);

        run_lua! { |Pinnacle|
            Pinnacle.quit()
        }
    })
}
