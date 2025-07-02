use std::{ffi::OsString, path::PathBuf, time::Duration};

use pinnacle::state::State;
use smithay::reexports::calloop::EventLoop;
use tempfile::TempDir;
use tokio::runtime::Runtime;

pub struct Server {
    pub event_loop: EventLoop<'static, State>,
    pub state: State,
    // Remove dir on drop
    pub _grpc_temp_dir: TempDir,
    pub runtime: Runtime,
    wayland_display: Option<OsString>,
}

impl Drop for Server {
    fn drop(&mut self) {
        let Some(mut wayland_display) = self.wayland_display.clone() else {
            return;
        };

        let runtime_dir = self
            .state
            .pinnacle
            .xdg_base_dirs
            .get_runtime_directory()
            .unwrap();

        let _ = std::fs::remove_file(runtime_dir.join(&wayland_display));

        wayland_display.push(".lock");
        let _ = std::fs::remove_file(runtime_dir.join(wayland_display));
    }
}

impl Server {
    pub fn new(create_socket: bool) -> Self {
        let event_loop = EventLoop::<State>::try_new().unwrap();

        let cli = pinnacle::cli::Cli {
            no_config: true,
            ..Default::default()
        };

        let mut state = State::new(
            pinnacle::cli::Backend::Dummy,
            event_loop.handle(),
            event_loop.get_signal(),
            PathBuf::from(""),
            Some(cli),
            create_socket,
        )
        .unwrap();

        let runtime = Runtime::new().unwrap();
        let _guard = runtime.enter();

        let grpc_temp_dir = tempfile::tempdir().unwrap();
        let grpc_dir = grpc_temp_dir.path();

        state.pinnacle.start_grpc_server(grpc_dir).unwrap();

        let wayland_display = create_socket.then_some(state.pinnacle.socket_name.clone());

        if create_socket {
            std::env::set_var("WAYLAND_DISPLAY", &state.pinnacle.socket_name);
        }

        Self {
            event_loop,
            state,
            _grpc_temp_dir: grpc_temp_dir,
            runtime,
            wayland_display,
        }
    }

    pub fn dispatch(&mut self) {
        self.event_loop
            .dispatch(Duration::ZERO, &mut self.state)
            .unwrap();
        self.state.on_event_loop_cycle_completion();
    }
}
