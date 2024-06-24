use std::cell::RefCell;

use smithay::reexports::wayland_server::{Resource, Weak};
use smithay::{
    delegate_kde_decoration, delegate_xdg_decoration,
    reexports::{
        wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1,
        wayland_protocols_misc::server_decoration::server::org_kde_kwin_server_decoration::{
            self, OrgKdeKwinServerDecoration,
        },
        wayland_server::{protocol::wl_surface::WlSurface, WEnum},
    },
    wayland::{
        compositor,
        shell::{
            kde::decoration::{KdeDecorationHandler, KdeDecorationState},
            xdg::{decoration::XdgDecorationHandler, ToplevelSurface, XdgToplevelSurfaceData},
        },
    },
};
use tracing::debug;

use crate::{
    state::{State, WithState},
    window::rules::DecorationMode,
};

impl State {
    fn new_decoration(
        &mut self,
        toplevel: ToplevelSurface,
    ) -> org_kde_kwin_server_decoration::Mode {
        let window_rule_mode = self
            .pinnacle
            .window_for_surface(toplevel.wl_surface())
            .or_else(|| {
                self.pinnacle
                    .unmapped_window_for_surface(toplevel.wl_surface())
            })
            .and_then(|window| window.with_state(|state| state.decoration_mode))
            .map(|mode| match mode {
                DecorationMode::ClientSide => zxdg_toplevel_decoration_v1::Mode::ClientSide,
                DecorationMode::ServerSide => zxdg_toplevel_decoration_v1::Mode::ServerSide,
            });

        tracing::debug!(?window_rule_mode, "new_decoration");

        toplevel.with_pending_state(|state| {
            state.decoration_mode =
                Some(window_rule_mode.unwrap_or(zxdg_toplevel_decoration_v1::Mode::ClientSide));
        });

        window_rule_mode
            .and_then(|mode| match mode {
                zxdg_toplevel_decoration_v1::Mode::ClientSide => {
                    Some(org_kde_kwin_server_decoration::Mode::Client)
                }
                zxdg_toplevel_decoration_v1::Mode::ServerSide => {
                    Some(org_kde_kwin_server_decoration::Mode::Server)
                }
                _ => None,
            })
            .unwrap_or(org_kde_kwin_server_decoration::Mode::Client)
    }

    fn request_mode(
        &mut self,
        toplevel: ToplevelSurface,
        mode: zxdg_toplevel_decoration_v1::Mode,
    ) -> org_kde_kwin_server_decoration::Mode {
        let window_rule_mode = self
            .pinnacle
            .window_for_surface(toplevel.wl_surface())
            .or_else(|| {
                self.pinnacle
                    .unmapped_window_for_surface(toplevel.wl_surface())
            })
            .and_then(|window| window.with_state(|state| state.decoration_mode))
            .map(|mode| match mode {
                DecorationMode::ClientSide => zxdg_toplevel_decoration_v1::Mode::ClientSide,
                DecorationMode::ServerSide => zxdg_toplevel_decoration_v1::Mode::ServerSide,
            });

        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(window_rule_mode.unwrap_or(mode));
        });

        let initial_configure_sent = compositor::with_states(toplevel.wl_surface(), |states| {
            states
                .data_map
                .get::<XdgToplevelSurfaceData>()
                .unwrap()
                .lock()
                .unwrap()
                .initial_configure_sent
        });
        if initial_configure_sent {
            toplevel.send_pending_configure();
        }

        match window_rule_mode.unwrap_or(mode) {
            zxdg_toplevel_decoration_v1::Mode::ServerSide => {
                org_kde_kwin_server_decoration::Mode::Server
            }
            _ => org_kde_kwin_server_decoration::Mode::Client,
        }
    }

    fn unset_mode(&mut self, toplevel: ToplevelSurface) {
        let window_rule_mode = self
            .pinnacle
            .window_for_surface(toplevel.wl_surface())
            .or_else(|| {
                self.pinnacle
                    .unmapped_window_for_surface(toplevel.wl_surface())
            })
            .and_then(|window| window.with_state(|state| state.decoration_mode))
            .map(|mode| match mode {
                DecorationMode::ClientSide => zxdg_toplevel_decoration_v1::Mode::ClientSide,
                DecorationMode::ServerSide => zxdg_toplevel_decoration_v1::Mode::ServerSide,
            });

        toplevel.with_pending_state(|state| {
            state.decoration_mode = window_rule_mode;
        });

        let initial_configure_sent = compositor::with_states(toplevel.wl_surface(), |states| {
            states
                .data_map
                .get::<XdgToplevelSurfaceData>()
                .unwrap()
                .lock()
                .unwrap()
                .initial_configure_sent
        });
        if initial_configure_sent {
            toplevel.send_pending_configure();
        }
    }
}

impl XdgDecorationHandler for State {
    fn new_decoration(&mut self, toplevel: ToplevelSurface) {
        self.new_decoration(toplevel);
    }

    fn request_mode(&mut self, toplevel: ToplevelSurface, mode: zxdg_toplevel_decoration_v1::Mode) {
        self.request_mode(toplevel, mode);
    }

    fn unset_mode(&mut self, toplevel: ToplevelSurface) {
        self.unset_mode(toplevel);
    }
}
delegate_xdg_decoration!(State);

pub type KdeDecorationObject = RefCell<Option<Weak<OrgKdeKwinServerDecoration>>>;

impl KdeDecorationHandler for State {
    fn kde_decoration_state(&self) -> &KdeDecorationState {
        &self.pinnacle.kde_decoration_state
    }

    fn new_decoration(&mut self, surface: &WlSurface, decoration: &OrgKdeKwinServerDecoration) {
        let Some(toplevel) = self
            .pinnacle
            .window_for_surface(surface)
            .or_else(|| self.pinnacle.unmapped_window_for_surface(surface))
            .and_then(|win| win.toplevel().cloned())
        else {
            debug!("kde-decoration: New decoration but no toplevel");
            return;
        };

        let kde_mode = self.new_decoration(toplevel);

        decoration.mode(kde_mode);

        compositor::with_states(surface, |states| {
            if !states
                .data_map
                .insert_if_missing(|| RefCell::new(Some(decoration.downgrade())))
            {
                states
                    .data_map
                    .get::<KdeDecorationObject>()
                    .unwrap()
                    .borrow_mut()
                    .replace(decoration.downgrade());
            }
        });
    }

    fn request_mode(
        &mut self,
        surface: &WlSurface,
        decoration: &OrgKdeKwinServerDecoration,
        mode: WEnum<org_kde_kwin_server_decoration::Mode>,
    ) {
        let Some(toplevel) = self
            .pinnacle
            .window_for_surface(surface)
            .or_else(|| self.pinnacle.unmapped_window_for_surface(surface))
            .and_then(|win| win.toplevel().cloned())
        else {
            debug!("kde-decoration: Request mode but no toplevel");
            return;
        };

        if let WEnum::Value(mode) = mode {
            let kde_mode = self.request_mode(
                toplevel,
                match mode {
                    org_kde_kwin_server_decoration::Mode::Server => {
                        zxdg_toplevel_decoration_v1::Mode::ServerSide
                    }
                    _ => zxdg_toplevel_decoration_v1::Mode::ClientSide,
                },
            );
            decoration.mode(kde_mode);
        }
    }

    fn release(&mut self, _decoration: &OrgKdeKwinServerDecoration, surface: &WlSurface) {
        let Some(toplevel) = self
            .pinnacle
            .window_for_surface(surface)
            .or_else(|| self.pinnacle.unmapped_window_for_surface(surface))
            .and_then(|win| win.toplevel().cloned())
        else {
            debug!("kde-decoration: Release mode but no toplevel");
            return;
        };

        self.unset_mode(toplevel);

        compositor::with_states(surface, |states| {
            let kde_decoration = states.data_map.get::<KdeDecorationObject>();
            if let Some(decoration) = kde_decoration {
                decoration.borrow_mut().take();
            }
        });
    }
}
delegate_kde_decoration!(State);
