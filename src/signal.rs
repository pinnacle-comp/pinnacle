use pinnacle_api_defs::pinnacle::signal::{self, v0alpha1::ListenResponse};

use crate::state::State;

impl State {
    /// Send a signal to clients if they have connected to `signal`.
    ///
    /// Note: There is currently no association between `signal` and what `data` is supposed to
    /// return, so the responsibility of making these match up is up to the caller.
    pub fn send_signal<F>(&self, signal: signal::v0alpha1::Signal, data: F)
    where
        F: FnOnce() -> signal::v0alpha1::listen_response::Signal,
    {
        if !self.connected_signals.contains(&signal) {
            return;
        }

        let Some(sender) = self.signal_sender.as_ref() else {
            return;
        };

        if let Err(err) = sender.send(Ok(ListenResponse {
            signal: Some(data()),
        })) {
            tracing::error!("Error sending signal to config client: {err}");
        }
    }
}
