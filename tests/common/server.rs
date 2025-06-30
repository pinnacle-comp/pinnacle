use std::{path::PathBuf, time::Duration};

use pinnacle::state::State;
use smithay::reexports::calloop::EventLoop;
use tempfile::TempDir;
use tokio::runtime::Runtime;

pub struct Server {
    pub event_loop: EventLoop<'static, State>,
    pub state: State,
    pub grpc_temp_dir: TempDir,
    pub runtime: Runtime,
}

impl Server {
    pub fn new() -> Self {
        let event_loop = EventLoop::<State>::try_new().unwrap();

        let mut state = State::new(
            pinnacle::cli::Backend::Dummy,
            event_loop.handle(),
            event_loop.get_signal(),
            PathBuf::from(""),
            None,
            false,
        )
        .unwrap();

        let runtime = Runtime::new().unwrap();
        let _guard = runtime.enter();

        let grpc_temp_dir = tempfile::tempdir().unwrap();
        let grpc_dir = grpc_temp_dir.path();

        state.pinnacle.start_grpc_server(grpc_dir).unwrap();

        Self {
            event_loop,
            state,
            grpc_temp_dir,
            runtime,
        }
    }

    pub fn dispatch(&mut self) {
        self.event_loop
            .dispatch(Duration::ZERO, &mut self.state)
            .unwrap();
        self.state.on_event_loop_cycle_completion();
    }
}
