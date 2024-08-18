use std::{panic::UnwindSafe, path::PathBuf, time::Duration};

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

pub fn test_api<F>(test: F) -> anyhow::Result<()>
where
    F: FnOnce(Sender<Box<dyn FnOnce(&mut State) + Send>>) -> anyhow::Result<()>
        + Send
        + UnwindSafe
        + 'static,
{
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

/* #[cfg(test)] //teste da fase vermelha
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;
    use pinnacle::state::State;
    use pinnacle::cli::Backend;

    // Função auxiliar para criar um estado de teste
    fn setup_test_state() -> State {
        let event_loop = EventLoop::<State>::try_new().unwrap();
        State::new(
            Backend::Dummy,
            event_loop.handle(),
            event_loop.get_signal(),
            PathBuf::from(""),
            None,
        )
        .unwrap()
    }

    #[test]
    fn test_attempt_connection_success() {
        let mut state = setup_test_state();

        // Deve passar se a conexão for bem-sucedida na primeira tentativa
        assert!(attempt_connection(&mut state).is_ok());
    }

    #[test]
    fn test_attempt_connection_failure() {
        let mut state = setup_test_state();

        // Simula falhas contínuas na conexão
        state.pinnacle.set_grpc_failure_mode(true);

        // Deve falhar após todas as tentativas
        assert!(attempt_connection(&mut state).is_err());
    }

    #[test]
    fn test_attempt_connection_retries() {
        let mut state = setup_test_state();

        // Simula falha na primeira tentativa e sucesso na segunda
        state.pinnacle.set_grpc_failure_mode(true);
        state.pinnacle.set_grpc_failure_mode_on_attempt(2, false);

        // A função deve eventualmente ter sucesso
        assert!(attempt_connection(&mut state).is_ok());
    }
 } //*

//teste de fase verde

use std::{path::PathBuf, sync::Arc, time::Duration};

// Número máximo de tentativas de reconexão
const MAX_RETRY_ATTEMPTS: usize = 5;
// Intervalo entre tentativas de reconexão
const RETRY_DELAY: Duration = Duration::from_secs(2);

fn attempt_connection(state: &mut State) -> Result<(), String> {
    for attempt in 1..=MAX_RETRY_ATTEMPTS {
        match state.pinnacle.start_grpc_server(&PathBuf::from("")) {
            Ok(_) => {
                println!("Connection successful on attempt {}", attempt);
                return Ok(());
            }
            Err(e) => {
                println!(
                    "Connection attempt {} failed: {}. Retrying in {:?}...",
                    attempt, e, RETRY_DELAY
                );
                std::thread::sleep(RETRY_DELAY);
            }
        }
    }
    Err("Failed to establish connection after multiple attempts.".to_string())
}
