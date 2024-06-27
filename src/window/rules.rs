use smithay::{
    desktop::space::SpaceElement,
    reexports::{
        wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1,
        wayland_protocols_misc::server_decoration::server::org_kde_kwin_server_decoration,
    },
    utils::Point,
    wayland::compositor,
};

use crate::{
    handlers::decoration::KdeDecorationObject,
    state::{Pinnacle, WithState},
    window::window_state::FloatingOrTiled,
};

use super::WindowElement;

use std::num::NonZeroU32;

use crate::{output::OutputName, tag::TagId, window::window_state::FullscreenOrMaximized};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WindowRuleCondition {
    /// This condition is met when any of the conditions provided is met.
    pub cond_any: Option<Vec<WindowRuleCondition>>,
    /// This condition is met when all of the conditions provided are met.
    pub cond_all: Option<Vec<WindowRuleCondition>>,
    /// This condition is met when the class matches.
    pub class: Option<Vec<String>>,
    /// This condition is met when the title matches.
    pub title: Option<Vec<String>>,
    /// This condition is met when the tag matches.
    pub tag: Option<Vec<TagId>>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum AllOrAny {
    All,
    Any,
}

impl WindowRuleCondition {
    /// RefCell Safety: This method uses RefCells on `window`.
    pub fn is_met(&self, pinnacle: &Pinnacle, window: &WindowElement) -> bool {
        Self::is_met_inner(self, pinnacle, window, AllOrAny::All)
    }

    fn is_met_inner(
        &self,
        pinnacle: &Pinnacle,
        window: &WindowElement,
        all_or_any: AllOrAny,
    ) -> bool {
        let WindowRuleCondition {
            cond_any,
            cond_all,
            class,
            title,
            tag,
        } = self;

        match all_or_any {
            AllOrAny::All => {
                let cond_any = if let Some(cond_any) = cond_any {
                    cond_any
                        .iter()
                        .any(|cond| Self::is_met_inner(cond, pinnacle, window, AllOrAny::Any))
                } else {
                    true
                };
                let cond_all = if let Some(cond_all) = cond_all {
                    cond_all
                        .iter()
                        .all(|cond| Self::is_met_inner(cond, pinnacle, window, AllOrAny::All))
                } else {
                    true
                };
                let classes = if let Some(classes) = class {
                    classes
                        .iter()
                        .all(|class| window.class().as_ref() == Some(class))
                } else {
                    true
                };
                let titles = if let Some(titles) = title {
                    titles
                        .iter()
                        .all(|title| window.title().as_ref() == Some(title))
                } else {
                    true
                };
                let tags = if let Some(tag_ids) = tag {
                    let mut tags = tag_ids.iter().filter_map(|tag_id| tag_id.tag(pinnacle));
                    tags.all(|tag| window.with_state(|state| state.tags.contains(&tag)))
                } else {
                    true
                };

                cond_all && cond_any && classes && titles && tags
            }
            AllOrAny::Any => {
                let cond_any = if let Some(cond_any) = cond_any {
                    cond_any
                        .iter()
                        .any(|cond| Self::is_met_inner(cond, pinnacle, window, AllOrAny::Any))
                } else {
                    false
                };
                let cond_all = if let Some(cond_all) = cond_all {
                    cond_all
                        .iter()
                        .all(|cond| Self::is_met_inner(cond, pinnacle, window, AllOrAny::All))
                } else {
                    false
                };
                let classes = if let Some(classes) = class {
                    classes
                        .iter()
                        .any(|class| window.class().as_ref() == Some(class))
                } else {
                    false
                };
                let titles = if let Some(titles) = title {
                    titles
                        .iter()
                        .any(|title| window.title().as_ref() == Some(title))
                } else {
                    false
                };
                let tags = if let Some(tag_ids) = tag {
                    let mut tags = tag_ids.iter().filter_map(|tag_id| tag_id.tag(pinnacle));
                    tags.any(|tag| window.with_state(|state| state.tags.contains(&tag)))
                } else {
                    false
                };
                cond_all || cond_any || classes || titles || tags
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecorationMode {
    ClientSide,
    ServerSide,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WindowRule {
    /// Set the output the window will open on.
    pub output: Option<OutputName>,
    /// Set the tags the output will have on open.
    pub tags: Option<Vec<TagId>>,
    /// Set the window to floating or tiled on open.
    pub floating_or_tiled: Option<FloatingOrTiled>,
    /// Set the window to fullscreen, maximized, or force it to neither.
    pub fullscreen_or_maximized: Option<FullscreenOrMaximized>,
    /// Set the window's initial size.
    pub size: Option<(NonZeroU32, NonZeroU32)>,
    /// Set the window's initial location. If the window is tiled, it will snap to this position
    /// when set to floating.
    pub location: Option<(i32, i32)>,
    pub decoration_mode: Option<DecorationMode>,
}

impl Pinnacle {
    pub fn apply_window_rules(&mut self, window: &WindowElement) {
        tracing::debug!("Applying window rules");
        for (cond, rule) in self.config.window_rules.iter() {
            if cond.is_met(self, window) {
                let WindowRule {
                    output,
                    tags,
                    floating_or_tiled,
                    fullscreen_or_maximized,
                    size,
                    location, // FIXME: make f64
                    decoration_mode,
                } = rule;

                // TODO: If both `output` and `tags` are specified, `tags` will apply over
                // |     `output`.

                if let Some(output_name) = output {
                    if let Some(output) = output_name.output(self) {
                        let tags = output
                            .with_state(|state| state.focused_tags().cloned().collect::<Vec<_>>());

                        window.with_state_mut(|state| state.tags.clone_from(&tags));
                    }
                }

                if let Some(tag_ids) = tags {
                    let tags = tag_ids
                        .iter()
                        .filter_map(|tag_id| tag_id.tag(self))
                        .collect::<Vec<_>>();

                    window.with_state_mut(|state| state.tags.clone_from(&tags));
                }

                if let Some(floating_or_tiled) = floating_or_tiled {
                    window.with_state_mut(|state| {
                        state.window_state.set_floating(match floating_or_tiled {
                            FloatingOrTiled::Floating => true,
                            FloatingOrTiled::Tiled => false,
                        })
                    });
                }

                if let Some(fs_or_max) = fullscreen_or_maximized {
                    match fs_or_max {
                        FullscreenOrMaximized::Neither => (), // TODO:
                        FullscreenOrMaximized::Fullscreen => {
                            window.with_state_mut(|state| state.window_state.set_fullscreen(true));
                        }
                        FullscreenOrMaximized::Maximized => {
                            window.with_state_mut(|state| state.window_state.set_maximized(true))
                        }
                    }
                }

                if let Some((w, h)) = size {
                    let mut window_size = window.geometry().size;
                    window_size.w = u32::from(*w) as i32;
                    window_size.h = u32::from(*h) as i32;

                    window.with_state_mut(|state| {
                        state.floating_size = Some(window_size);
                    });
                }

                // FIXME: make this f64
                if let Some(location) = location {
                    window.with_state_mut(|state| {
                        state.floating_loc = Some(Point::from(*location).to_f64());
                    });
                }

                if let Some(decoration_mode) = decoration_mode {
                    tracing::debug!(?decoration_mode, toplevel = ?window.toplevel(), "Window rule with decoration mode");
                    window.with_state_mut(|state| {
                        state.decoration_mode = Some(*decoration_mode);
                    });
                    if let Some(toplevel) = window.toplevel() {
                        toplevel.with_pending_state(|state| {
                            state.decoration_mode = Some(match decoration_mode {
                                DecorationMode::ClientSide => {
                                    zxdg_toplevel_decoration_v1::Mode::ClientSide
                                }
                                DecorationMode::ServerSide => {
                                    zxdg_toplevel_decoration_v1::Mode::ServerSide
                                }
                            })
                        });

                        compositor::with_states(toplevel.wl_surface(), |states| {
                            let kde_decoration = states.data_map.get::<KdeDecorationObject>();
                            if let Some(kde_decoration) = kde_decoration {
                                if let Some(object) = kde_decoration
                                    .borrow()
                                    .as_ref()
                                    .and_then(|obj| obj.upgrade().ok())
                                {
                                    let mode = match decoration_mode {
                                        DecorationMode::ClientSide => {
                                            org_kde_kwin_server_decoration::Mode::Client
                                        }
                                        DecorationMode::ServerSide => {
                                            org_kde_kwin_server_decoration::Mode::Server
                                        }
                                    };
                                    tracing::debug!(?mode, "Window rule set KDE decoration mode");
                                    object.mode(mode);
                                }
                            }
                        });
                    }
                }
            }
        }
    }
}
