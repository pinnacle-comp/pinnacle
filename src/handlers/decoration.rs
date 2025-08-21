use std::cell::RefCell;

use smithay::reexports::wayland_server::{Resource, Weak};
use smithay::{
    delegate_kde_decoration, delegate_xdg_decoration,
    reexports::{
        wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1,
        wayland_protocols_misc::server_decoration::server::org_kde_kwin_server_decoration::{
            self, OrgKdeKwinServerDecoration,
        },
        wayland_server::{WEnum, protocol::wl_surface::WlSurface},
    },
    wayland::{
        compositor,
        shell::{
            kde::decoration::{KdeDecorationHandler, KdeDecorationState},
            xdg::{ToplevelSurface, decoration::XdgDecorationHandler},
        },
    },
};
use tracing::debug;

use crate::state::{State, WithState};
use crate::window::UnmappedState;

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
            match &mut unmapped.state {
                UnmappedState::WaitingForTags { client_requests }
                | UnmappedState::WaitingForRules {
                    rules: _,
                    client_requests,
                } => {
                    client_requests.decoration_mode = Some(mode);

                    match mode {
                        zxdg_toplevel_decoration_v1::Mode::ServerSide => {
                            org_kde_kwin_server_decoration::Mode::Server
                        }
                        _ => org_kde_kwin_server_decoration::Mode::Client,
                    }
                }
                UnmappedState::PostInitialConfigure { .. } => {
                    let window = &unmapped.window;

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
                }
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
        } else if let Some(unmapped) = self
            .pinnacle
            .unmapped_window_for_surface_mut(toplevel.wl_surface())
        {
            match &mut unmapped.state {
                UnmappedState::WaitingForTags { client_requests } => {
                    client_requests.decoration_mode = None;
                }
                UnmappedState::WaitingForRules {
                    rules: _,
                    client_requests,
                } => {
                    client_requests.decoration_mode = None;
                }
                UnmappedState::PostInitialConfigure { .. } => {
                    let window = &unmapped.window;

                    let window_rule_mode = window.with_state(|state| state.decoration_mode);

                    toplevel.with_pending_state(|state| {
                        state.decoration_mode = window_rule_mode;
                    });

                    toplevel.send_pending_configure();
                }
            }
        }
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

type KdeDecorationObject = RefCell<KdeDecorationObjectInner>;

#[derive(Default)]
struct KdeDecorationObjectInner {
    protocol_obj: Option<Weak<OrgKdeKwinServerDecoration>>,
    last_requested_mode: Option<org_kde_kwin_server_decoration::Mode>,
}

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
            states
                .data_map
                .get_or_insert(KdeDecorationObject::default)
                .borrow_mut()
                .protocol_obj
                .replace(decoration.downgrade());
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
            // Server is responsible for preventing KDE decoration feedback loops
            let already_requested_mode = compositor::with_states(surface, |states| {
                let last_requested_mode = states
                    .data_map
                    .get_or_insert(KdeDecorationObject::default)
                    .borrow_mut()
                    .last_requested_mode
                    .replace(mode);
                last_requested_mode == Some(mode)
            });

            if already_requested_mode {
                return;
            }

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
            states
                .data_map
                .get_or_insert(KdeDecorationObject::default)
                .take();
        });
    }
}
delegate_kde_decoration!(State);

/// Updates the KDE decoration mode of a surface (if it has one) from an XDG decoration mode.
pub fn update_kde_decoration_mode(surface: &WlSurface, mode: zxdg_toplevel_decoration_v1::Mode) {
    compositor::with_states(surface, |states| {
        let decoration = states
            .data_map
            .get_or_insert(KdeDecorationObject::default)
            .borrow()
            .protocol_obj
            .as_ref()
            .and_then(|obj| obj.upgrade().ok());

        if let Some(decoration) = decoration {
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
    });
}
