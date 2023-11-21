use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use smithay::{
    utils::Serial,
    wayland::{
        compositor::{self, Blocker, BlockerState},
        shell::xdg::XdgToplevelSurfaceData,
    },
};

use super::WindowElement;

#[derive(Debug, Clone)]
pub struct TiledWindowBlocker {
    wins_and_serials: Arc<Vec<(WindowElement, Serial)>>,
    start_time: Instant,
}

impl TiledWindowBlocker {
    pub fn new(wins_and_serials: impl IntoIterator<Item = (WindowElement, Serial)>) -> Self {
        let wins_and_serials = wins_and_serials.into_iter().collect::<Vec<_>>();

        Self {
            wins_and_serials: Arc::new(wins_and_serials),
            start_time: Instant::now(),
        }
    }

    // From cosmic-comp
    pub fn ready(&self) -> bool {
        let too_long_since_start =
            Instant::now().duration_since(self.start_time) >= Duration::from_millis(500);

        let all_windows_acked = self.wins_and_serials.iter().all(|(win, serial)| match win {
            WindowElement::Wayland(win) => {
                compositor::with_states(win.toplevel().wl_surface(), |states| {
                    let attrs = states
                        .data_map
                        .get::<XdgToplevelSurfaceData>()
                        .expect("no XdgToplevelSurfaceData")
                        .lock()
                        .expect("failed to lock mutex");

                    attrs
                        .configure_serial
                        .as_ref()
                        .map(|s| s >= serial)
                        .unwrap_or(false)
                })
            }
            WindowElement::X11(_) => true,
        });

        tracing::debug!(
            "blocker ready is {}",
            too_long_since_start || all_windows_acked
        );

        too_long_since_start || all_windows_acked
    }
}

impl Blocker for TiledWindowBlocker {
    fn state(&self) -> BlockerState {
        if self.ready() {
            BlockerState::Released
        } else {
            BlockerState::Pending
        }
    }
}
