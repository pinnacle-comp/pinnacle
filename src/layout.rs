// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::{HashMap, HashSet};

use pinnacle_api_defs::pinnacle::layout::v0alpha1::{layout_request::Geometries, LayoutResponse};
use smithay::{
    desktop::{layer_map_for_output, WindowSurface},
    output::Output,
    utils::{Logical, Point, Rectangle, Serial},
    wayland::{compositor, shell::xdg::XdgToplevelSurfaceData},
};
use tokio::sync::mpsc::UnboundedSender;
use tonic::Status;
use tracing::warn;

use crate::{
    output::OutputName,
    state::{State, WithState},
    window::{
        window_state::{FloatingOrTiled, FullscreenOrMaximized},
        WindowElement,
    },
};

impl State {
    fn update_windows_with_geometries(
        &mut self,
        output: &Output,
        geometries: Vec<Rectangle<i32, Logical>>,
    ) {
        let windows_on_foc_tags = output.with_state(|state| {
            let focused_tags = state.focused_tags().collect::<Vec<_>>();
            self.windows
                .iter()
                .filter(|win| !win.is_x11_override_redirect())
                .filter(|win| {
                    win.with_state(|state| state.tags.iter().any(|tg| focused_tags.contains(&tg)))
                })
                .cloned()
                .collect::<Vec<_>>()
        });

        let tiled_windows = windows_on_foc_tags
            .iter()
            .filter(|win| {
                win.with_state(|state| {
                    state.floating_or_tiled.is_tiled() && state.fullscreen_or_maximized.is_neither()
                })
            })
            .cloned();

        let output_geo = self.space.output_geometry(output).expect("no output geo");

        let non_exclusive_geo = {
            let map = layer_map_for_output(output);
            map.non_exclusive_zone()
        };

        let mut zipped = tiled_windows.zip(geometries.into_iter().map(|mut geo| {
            geo.loc += output_geo.loc + non_exclusive_geo.loc;
            geo
        }));

        for (win, geo) in zipped.by_ref() {
            win.change_geometry(geo);
        }

        let (remaining_wins, _remaining_geos) = zipped.unzip::<_, _, Vec<_>, Vec<_>>();

        for win in remaining_wins {
            assert!(win.with_state(|state| state.floating_or_tiled.is_floating()));
            win.toggle_floating();
        }

        for window in windows_on_foc_tags.iter() {
            match window.with_state(|state| state.fullscreen_or_maximized) {
                FullscreenOrMaximized::Fullscreen => {
                    window.change_geometry(output_geo);
                }
                FullscreenOrMaximized::Maximized => {
                    window.change_geometry(Rectangle::from_loc_and_size(
                        output_geo.loc + non_exclusive_geo.loc,
                        non_exclusive_geo.size,
                    ));
                }
                FullscreenOrMaximized::Neither => {
                    if let FloatingOrTiled::Floating(rect) =
                        window.with_state(|state| state.floating_or_tiled)
                    {
                        window.change_geometry(rect);
                    }
                }
            }
        }

        let mut pending_wins = Vec::<(WindowElement, Serial)>::new();
        let mut non_pending_wins = Vec::<(Point<i32, Logical>, WindowElement)>::new();

        for win in windows_on_foc_tags.iter() {
            if win.with_state(|state| state.target_loc.is_some()) {
                match win.underlying_surface() {
                    WindowSurface::Wayland(toplevel) => {
                        let pending = compositor::with_states(toplevel.wl_surface(), |states| {
                            states
                                .data_map
                                .get::<XdgToplevelSurfaceData>()
                                .expect("XdgToplevelSurfaceData wasn't in surface's data map")
                                .lock()
                                .expect("Failed to lock Mutex<XdgToplevelSurfaceData>")
                                .has_pending_changes()
                        });

                        if pending {
                            pending_wins.push((win.clone(), toplevel.send_configure()))
                        } else {
                            let loc = win.with_state_mut(|state| state.target_loc.take());
                            if let Some(loc) = loc {
                                non_pending_wins.push((loc, win.clone()));
                            }
                        }
                    }
                    WindowSurface::X11(_) => {
                        let loc = win.with_state_mut(|state| state.target_loc.take());
                        if let Some(loc) = loc {
                            self.space.map_element(win.clone(), loc, false);
                        }
                    }
                }
            }
        }

        for (loc, window) in non_pending_wins {
            self.space.map_element(window, loc, false);
        }

        self.fixup_z_layering();
    }

    /// Swaps two windows in the main window vec and updates all windows.
    pub fn swap_window_positions(&mut self, win1: &WindowElement, win2: &WindowElement) {
        let win1_index = self.windows.iter().position(|win| win == win1);
        let win2_index = self.windows.iter().position(|win| win == win2);

        if let (Some(first), Some(second)) = (win1_index, win2_index) {
            self.windows.swap(first, second);
            if let Some(output) = win1.output(self) {
                self.request_layout(&output);
            }
        }
    }
}

/// A monotonically increasing identifier for layout requests.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct LayoutRequestId(pub u32);

#[derive(Debug, Default)]
pub struct LayoutState {
    pub layout_request_sender: Option<UnboundedSender<Result<LayoutResponse, Status>>>,
    id_maps: HashMap<Output, LayoutRequestId>,
    pending_requests: HashMap<Output, Vec<(LayoutRequestId, Vec<WindowElement>)>>,
    old_requests: HashMap<Output, HashSet<LayoutRequestId>>,
}

impl State {
    pub fn request_layout(&mut self, output: &Output) {
        let Some(sender) = self.layout_state.layout_request_sender.as_ref() else {
            warn!("Layout requested but no client has connected to the layout service");
            return;
        };

        let windows_on_foc_tags = output.with_state(|state| {
            let focused_tags = state.focused_tags().collect::<Vec<_>>();
            self.windows
                .iter()
                .filter(|win| !win.is_x11_override_redirect())
                .filter(|win| {
                    win.with_state(|state| state.tags.iter().any(|tg| focused_tags.contains(&tg)))
                })
                .cloned()
                .collect::<Vec<_>>()
        });

        let windows = windows_on_foc_tags
            .iter()
            .filter(|win| {
                win.with_state(|state| {
                    state.floating_or_tiled.is_tiled() && state.fullscreen_or_maximized.is_neither()
                })
            })
            .cloned()
            .collect::<Vec<_>>();

        let (output_width, output_height) = {
            let map = layer_map_for_output(output);
            let zone = map.non_exclusive_zone();
            (zone.size.w, zone.size.h)
        };

        let window_ids = windows
            .iter()
            .map(|win| win.with_state(|state| state.id.0))
            .collect::<Vec<_>>();

        let tag_ids =
            output.with_state(|state| state.focused_tags().map(|tag| tag.id().0).collect());

        let id = self
            .layout_state
            .id_maps
            .entry(output.clone())
            .or_insert(LayoutRequestId(0));

        self.layout_state
            .pending_requests
            .entry(output.clone())
            .or_default()
            .push((*id, windows));

        // TODO: error
        let _ = sender.send(Ok(LayoutResponse {
            request_id: Some(id.0),
            output_name: Some(output.name()),
            window_ids,
            tag_ids,
            output_width: Some(output_width as u32),
            output_height: Some(output_height as u32),
        }));

        *id = LayoutRequestId(id.0 + 1);
    }

    pub fn apply_layout(&mut self, geometries: Geometries) -> anyhow::Result<()> {
        let Geometries {
            request_id: Some(request_id),
            output_name: Some(output_name),
            geometries,
        } = geometries
        else {
            anyhow::bail!("One or more `geometries` fields were None");
        };

        let request_id = LayoutRequestId(request_id);
        let Some(output) = OutputName(output_name).output(self) else {
            anyhow::bail!("Output was invalid");
        };

        let old_requests = self
            .layout_state
            .old_requests
            .entry(output.clone())
            .or_default();

        if old_requests.contains(&request_id) {
            anyhow::bail!("Attempted to layout but the request was already fulfilled");
        }

        let pending = self
            .layout_state
            .pending_requests
            .entry(output.clone())
            .or_default();

        let Some(latest) = pending.last().map(|(id, _)| *id) else {
            anyhow::bail!("Attempted to layout but the request was nonexistent A");
        };

        if latest == request_id {
            pending.pop();
        } else if let Some(pos) = pending
            .split_last()
            .and_then(|(_, rest)| rest.iter().position(|(id, _)| id == &request_id))
        {
            // Ignore stale requests
            old_requests.insert(request_id);
            pending.remove(pos);
            return Ok(());
        } else {
            anyhow::bail!("Attempted to layout but the request was nonexistent B");
        };

        let geometries = geometries
            .into_iter()
            .map(|geo| {
                Some(Rectangle::<i32, Logical>::from_loc_and_size(
                    (geo.x?, geo.y?),
                    (i32::max(geo.width?, 1), i32::max(geo.height?, 1)),
                ))
            })
            .collect::<Option<Vec<_>>>();

        let Some(geometries) = geometries else {
            anyhow::bail!("Attempted to layout but one or more dimensions were null");
        };

        self.update_windows_with_geometries(&output, geometries);

        self.schedule_render(&output);

        Ok(())
    }
}
