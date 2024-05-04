use std::{collections::HashMap, path::Path};

use smithay::reexports::{calloop, wayland_server::Client};
use tracing::debug;

use crate::{
    state::{Pinnacle, WithState},
    tag::TagId,
};

use super::Backend;

#[derive(Default)]
pub struct Wlcs {
    pub clients: HashMap<i32, Client>,
}

impl Backend {
    pub fn wlcs_mut(&mut self) -> &mut Wlcs {
        let Backend::Dummy(dummy) = self else {
            unreachable!(r#"feature gated by "wlcs""#)
        };
        &mut dummy.wlcs_state
    }
}

impl Pinnacle {
    pub fn start_wlcs_config<F>(&mut self, socket_dir: &Path, run_config: F) -> anyhow::Result<()>
    where
        F: FnOnce() + Send + 'static,
    {
        // Clear state
        debug!("Clearing tags");
        for output in self.space.outputs() {
            output.with_state_mut(|state| state.tags.clear());
        }

        TagId::reset();

        debug!("Clearing input state");

        self.input_state.clear();

        self.config.clear(&self.loop_handle);

        self.signal_state.clear();

        self.input_state.reload_keybind = None;
        self.input_state.kill_keybind = None;

        if self.grpc_server_join_handle.is_none() {
            self.start_grpc_server(socket_dir)?;
        }

        let (pinger, ping_source) = calloop::ping::make_ping()?;

        let token = self
            .loop_handle
            .insert_source(ping_source, move |_, _, _state| {})?;

        std::thread::spawn(move || {
            run_config();
            pinger.ping();
        });

        self.config.config_reload_on_crash_token = Some(token);

        Ok(())
    }
}
