use indexmap::IndexSet;
use smithay::{
    desktop::WindowSurface,
    output::Output,
    reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1,
    utils::{Logical, Point, Size},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    state::{Pinnacle, WithState},
    tag::Tag,
};

use super::{
    window_state::{LayoutMode, WindowId},
    Unmapped, WindowElement,
};

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

#[derive(Debug, Default)]
pub struct WindowRuleState {
    pub pending_windows: HashMap<WindowElement, PendingWindowRuleRequest>,
    pub senders: Vec<(UnboundedSender<WindowRuleRequest>, Arc<AtomicU32>)>,
    current_request_id: u32,
}

#[derive(Debug, Clone, Default)]
pub struct WindowRules {
    pub layout_mode: Option<LayoutMode>,
    pub focused: Option<bool>,
    pub floating_loc: Option<Point<f64, Logical>>,
    pub floating_size: Option<Size<i32, Logical>>,
    pub decoration_mode: Option<zxdg_toplevel_decoration_v1::Mode>,
    pub tags: IndexSet<Tag>,
}

impl WindowRules {
    pub fn set_tags_to_output(&mut self, output: &Output) {
        super::set_tags_to_output(&mut self.tags, output);
    }
}

impl WindowRuleState {
    /// Returns whether a request was sent
    pub fn new_request(&mut self, window: &WindowElement) -> bool {
        let _span = tracy_client::span!("WindowRuleState::new_request");

        let window_rule_already_finished = match window.underlying_surface() {
            WindowSurface::Wayland(toplevel) => toplevel.is_initial_configure_sent(),
            WindowSurface::X11(surface) => surface.is_mapped(),
        };
        if window_rule_already_finished {
            return true;
        }

        if self.pending_windows.contains_key(window) {
            return true;
        }

        let request_id = self.current_request_id;
        self.current_request_id += 1;

        let mut waiting_on = Vec::new();
        self.senders.retain(|(sender, id)| {
            let sent = sender
                .send(WindowRuleRequest {
                    request_id,
                    window_id: window.with_state(|state| state.id),
                })
                .is_ok();

            if sent {
                waiting_on.push(id.clone());
            }

            sent
        });

        if waiting_on.is_empty() {
            return false;
        }

        let pending_request = PendingWindowRuleRequest {
            request_id,
            waiting_on,
        };

        self.pending_windows.insert(window.clone(), pending_request);

        true
    }

    pub fn new_sender(
        &mut self,
        sender: UnboundedSender<WindowRuleRequest>,
        id_ctr: Arc<AtomicU32>,
    ) {
        self.senders.push((sender, id_ctr));
    }

    pub fn finished_windows(&mut self) -> Vec<WindowElement> {
        let _span = tracy_client::span!("WindowRuleState::finished_windows");

        let mut finished = Vec::new();
        self.pending_windows.retain(|window, pending_request| {
            let still_pending = !pending_request.is_done();

            if !still_pending {
                finished.push(window.clone());
            }

            still_pending
        });
        finished
    }
}

pub struct WindowRuleRequest {
    pub request_id: u32,
    pub window_id: WindowId,
}

#[derive(Debug)]
pub struct PendingWindowRuleRequest {
    request_id: u32,
    waiting_on: Vec<Arc<AtomicU32>>,
}

impl PendingWindowRuleRequest {
    pub fn new(request_id: u32, waiting_on: Vec<Arc<AtomicU32>>) -> Self {
        Self {
            request_id,
            waiting_on,
        }
    }

    pub fn is_done(&self) -> bool {
        let _span = tracy_client::span!("PendingWindowRuleRequest::is_done");

        self.waiting_on
            .iter()
            .all(|id| id.load(Ordering::Acquire) >= self.request_id)
    }
}

impl Pinnacle {
    pub fn apply_window_rules(&self, unmapped: &Unmapped) {
        let WindowRules {
            layout_mode,
            focused: _,
            floating_loc,
            floating_size,
            decoration_mode,
            tags,
        } = &unmapped.window_rules;

        let layout_mode = layout_mode.unwrap_or(LayoutMode::new_tiled());

        unmapped.window.with_state_mut(|state| {
            state.layout_mode = layout_mode;
            state.floating_loc = *floating_loc;
            state.floating_size = floating_size.unwrap_or(state.floating_size);
            state.decoration_mode = *decoration_mode;
            state.tags = tags.clone();
        });

        self.configure_window_if_nontiled(&unmapped.window);

        if let WindowSurface::Wayland(toplevel) = unmapped.window.underlying_surface() {
            toplevel.with_pending_state(|state| {
                state.decoration_mode = *decoration_mode;
            });
            crate::handlers::decoration::update_kde_decoration_mode(
                toplevel.wl_surface(),
                decoration_mode.unwrap_or(zxdg_toplevel_decoration_v1::Mode::ClientSide),
            );
        }
    }

    /// Request window rules from the config.
    ///
    /// If there are no window rules set, immediately sends the initial configure for toplevels
    /// or maps x11 surfaces.
    pub fn request_window_rules(&mut self, unmapped: &Unmapped) {
        let window_rule_request_sent = self.window_rule_state.new_request(&unmapped.window);

        // If the above is false, then there are either
        //   a. No window rules in place, or
        //   b. all clients with window rules are dead
        //
        // In this case, send the initial configure here instead of waiting.
        if !window_rule_request_sent {
            self.apply_window_rules(unmapped);
            match unmapped.window.underlying_surface() {
                WindowSurface::Wayland(toplevel) => {
                    toplevel.send_configure();
                }
                WindowSurface::X11(surface) => {
                    let _ = surface.set_mapped(true);
                }
            }
        }
    }
}
