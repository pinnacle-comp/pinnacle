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
            xdg::{decoration::XdgDecorationHandler, ToplevelSurface},
        },
    },
};
use tracing::debug;

use crate::state::{State, WithState};

impl State {
    fn new_decoration(
        &mut self,
        toplevel: ToplevelSurface,
    ) -> org_kde_kwin_server_decoration::Mode {
        let _span = tracy_client::span!("State::new_decoration");

        let window_rule_mode = self
            .pinnacle
            .window_for_surface(toplevel.wl_surface())
            .or_else(|| {
                self.pinnacle
                    .unmapped_window_for_surface(toplevel.wl_surface())
                    .map(|unmapped| &unmapped.window)
            })
            .and_then(|window| window.with_state(|state| state.decoration_mode));

        toplevel.with_pending_state(|state| {
            state.decoration_mode = window_rule_mode;
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
        let _span = tracy_client::span!("State::request_mode");

        if let Some(window) = self.pinnacle.window_for_surface(toplevel.wl_surface()) {
            let window_rule_mode = window.with_state(|state| state.decoration_mode);

            toplevel.with_pending_state(|state| {
                state.decoration_mode = Some(window_rule_mode.unwrap_or(mode));
            });

            toplevel.send_configure();

            match window_rule_mode.unwrap_or(mode) {
                zxdg_toplevel_decoration_v1::Mode::ServerSide => {
                    org_kde_kwin_server_decoration::Mode::Server
                }
                _ => org_kde_kwin_server_decoration::Mode::Client,
            }
        } else if let Some(unmapped) = self
            .pinnacle
            .unmapped_window_for_surface_mut(toplevel.wl_surface())
        {
            if unmapped.window_rules.decoration_mode.is_none() {
                unmapped.window_rules.decoration_mode = Some(mode);
            }

            match unmapped.window_rules.decoration_mode.unwrap_or(mode) {
                zxdg_toplevel_decoration_v1::Mode::ServerSide => {
                    org_kde_kwin_server_decoration::Mode::Server
                }
                _ => org_kde_kwin_server_decoration::Mode::Client,
            }
        } else {
            org_kde_kwin_server_decoration::Mode::Client
        }
    }

    fn unset_mode(&mut self, toplevel: ToplevelSurface) {
        let _span = tracy_client::span!("State::unset_mode");

        if let Some(window) = self.pinnacle.window_for_surface(toplevel.wl_surface()) {
            let window_rule_mode = window.with_state(|state| state.decoration_mode);

            toplevel.with_pending_state(|state| {
                state.decoration_mode = window_rule_mode;
            });

            toplevel.send_pending_configure();
        }
        // FIXME: for unmapped windows:
        // An unset cannot tell whether the decoration mode in a window rule
        // was set by the config or by a decoration protocol.
        // We are ignoring unsets here until this is fixed.
    }
}

impl XdgDecorationHandler for State {
    fn new_decoration(&mut self, toplevel: ToplevelSurface) {
        let _span = tracy_client::span!("XdgDecorationHandler::new_decoration");
        self.new_decoration(toplevel);
    }

    fn request_mode(&mut self, toplevel: ToplevelSurface, mode: zxdg_toplevel_decoration_v1::Mode) {
        let _span = tracy_client::span!("XdgDecorationHandler::request_mode");
        self.request_mode(toplevel, mode);
    }

    fn unset_mode(&mut self, toplevel: ToplevelSurface) {
        let _span = tracy_client::span!("XdgDecorationHandler::unset_mode");
        self.unset_mode(toplevel);
    }
}
delegate_xdg_decoration!(State);

type KdeDecorationObject = RefCell<Option<Weak<OrgKdeKwinServerDecoration>>>;

impl KdeDecorationHandler for State {
    fn kde_decoration_state(&self) -> &KdeDecorationState {
        &self.pinnacle.kde_decoration_state
    }

    fn new_decoration(&mut self, surface: &WlSurface, decoration: &OrgKdeKwinServerDecoration) {
        let _span = tracy_client::span!("KdeDecorationHandler::new_decoration");

        let Some(toplevel) = self
            .pinnacle
            .window_for_surface(surface)
            .or_else(|| {
                self.pinnacle
                    .unmapped_window_for_surface(surface)
                    .map(|unmapped| &unmapped.window)
            })
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
        let _span = tracy_client::span!("KdeDecorationHandler::request_mode");

        let Some(toplevel) = self
            .pinnacle
            .window_for_surface(surface)
            .or_else(|| {
                self.pinnacle
                    .unmapped_window_for_surface(surface)
                    .map(|unmapped| &unmapped.window)
            })
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
        let _span = tracy_client::span!("KdeDecorationHandler::release");

        let Some(toplevel) = self
            .pinnacle
            .window_for_surface(surface)
            .or_else(|| {
                self.pinnacle
                    .unmapped_window_for_surface(surface)
                    .map(|unmapped| &unmapped.window)
            })
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

/// Updates the KDE decoration mode of a surface (if it has one) from an XDG decoration mode.
pub fn update_kde_decoration_mode(surface: &WlSurface, mode: zxdg_toplevel_decoration_v1::Mode) {
    compositor::with_states(surface, |states| {
        let kde_decoration = states.data_map.get::<KdeDecorationObject>();
        if let Some(kde_decoration) = kde_decoration {
            if let Some(decoration) = kde_decoration
                .borrow()
                .as_ref()
                .and_then(|obj| obj.upgrade().ok())
            {
                let mode = match mode {
                    zxdg_toplevel_decoration_v1::Mode::ServerSide => {
                        org_kde_kwin_server_decoration::Mode::Server
                    }
                    zxdg_toplevel_decoration_v1::Mode::ClientSide | _ => {
                        org_kde_kwin_server_decoration::Mode::Client
                    }
                };
                decoration.mode(mode);
            }
        }
    });
}
