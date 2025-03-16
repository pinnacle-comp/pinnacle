use smithay::{
    delegate_idle_inhibit, delegate_idle_notify,
    desktop::utils::surface_primary_scanout_output,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::IsAlive,
    wayland::{
        compositor,
        idle_inhibit::IdleInhibitHandler,
        idle_notify::{IdleNotifierHandler, IdleNotifierState},
    },
};

use crate::state::{Pinnacle, State};

impl IdleNotifierHandler for State {
    fn idle_notifier_state(&mut self) -> &mut IdleNotifierState<Self> {
        &mut self.pinnacle.idle_notifier_state
    }
}
delegate_idle_notify!(State);

impl IdleInhibitHandler for State {
    fn inhibit(&mut self, surface: WlSurface) {
        self.pinnacle.idle_inhibiting_surfaces.insert(surface);
        self.pinnacle.idle_notifier_state.set_is_inhibited(true);
    }

    fn uninhibit(&mut self, surface: WlSurface) {
        self.pinnacle.idle_inhibiting_surfaces.remove(&surface);
        self.pinnacle.refresh_idle_inhibit();
    }
}
delegate_idle_inhibit!(State);

impl Pinnacle {
    pub fn refresh_idle_inhibit(&mut self) {
        let _span = tracy_client::span!("Pinnacle::refresh_idle_inhibit");

        self.idle_inhibiting_surfaces.retain(|s| s.alive());

        let is_inhibited = self.idle_inhibiting_surfaces.iter().any(|surface| {
            compositor::with_states(surface, |states| {
                surface_primary_scanout_output(surface, states).is_some()
            })
        });

        self.idle_notifier_state.set_is_inhibited(is_inhibited);
    }
}
