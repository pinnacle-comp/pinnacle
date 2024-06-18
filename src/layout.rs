// SPDX-License-Identifier: GPL-3.0-or-later

pub mod transaction;

use std::collections::HashMap;

use pinnacle_api_defs::pinnacle::layout::v0alpha1::{layout_request::Geometries, LayoutResponse};
use smithay::{
    desktop::{layer_map_for_output, WindowSurface},
    output::Output,
    utils::{Logical, Rectangle, Serial},
};
use tokio::sync::mpsc::UnboundedSender;
use tonic::Status;
use tracing::warn;

use crate::{
    output::OutputName,
    render::util::snapshot::capture_snapshots_on_output,
    state::{Pinnacle, State, WithState},
    window::{
        window_state::{FloatingOrTiled, FullscreenOrMaximized},
        WindowElement,
    },
};

use self::transaction::LayoutTransaction;

impl Pinnacle {
    // FIXME: make layout calls use f64 loc
    fn update_windows_with_geometries(
        &mut self,
        output: &Output,
        geometries: Vec<Rectangle<i32, Logical>>,
    ) -> Vec<(WindowElement, Serial)> {
        let (windows_on_foc_tags, to_unmap) = output.with_state(|state| {
            let focused_tags = state.focused_tags().collect::<Vec<_>>();
            self.windows
                .iter()
                .filter(|win| win.output(self).as_ref() == Some(output))
                .cloned()
                .partition::<Vec<_>, _>(|win| {
                    win.with_state(|state| state.tags.iter().any(|tg| focused_tags.contains(&tg)))
                })
        });

        for win in to_unmap {
            self.space.unmap_elem(&win);
        }

        let tiled_windows = windows_on_foc_tags
            .iter()
            .filter(|win| !win.is_x11_override_redirect())
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
            win.change_geometry(geo.loc.to_f64(), geo.size);
        }

        let (remaining_wins, _remaining_geos) = zipped.unzip::<_, _, Vec<_>, Vec<_>>();

        for win in remaining_wins {
            assert!(win.with_state(|state| state.floating_or_tiled.is_floating()));
            win.toggle_floating();
        }

        for window in windows_on_foc_tags.iter() {
            match window.with_state(|state| state.fullscreen_or_maximized) {
                FullscreenOrMaximized::Fullscreen => {
                    window.change_geometry(output_geo.loc.to_f64(), output_geo.size);
                }
                FullscreenOrMaximized::Maximized => {
                    window.change_geometry(
                        (output_geo.loc + non_exclusive_geo.loc).to_f64(),
                        non_exclusive_geo.size,
                    );
                }
                FullscreenOrMaximized::Neither => {
                    if let FloatingOrTiled::Floating { loc, size } =
                        window.with_state(|state| state.floating_or_tiled)
                    {
                        window.change_geometry(loc, size);
                    }
                }
            }
        }

        let mut pending_wins = Vec::<(WindowElement, Serial)>::new();

        for win in windows_on_foc_tags.iter() {
            if let WindowSurface::Wayland(toplevel) = win.underlying_surface() {
                if let Some(serial) = toplevel.send_pending_configure() {
                    pending_wins.push((win.clone(), serial));
                }
            }

            // TODO: get rid of target_loc
            let loc = win.with_state_mut(|state| state.target_loc.take());
            if let Some(loc) = loc {
                self.space.map_element(win.clone(), loc, false);
            }
        }

        self.fixup_z_layering();

        pending_wins
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
            self.layout_state.pending_swap = true;
        }
    }
}

/// A monotonically increasing identifier for layout requests.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct LayoutRequestId(u32);

#[derive(Debug, Default)]
pub struct LayoutState {
    pub layout_request_sender: Option<UnboundedSender<Result<LayoutResponse, Status>>>,
    pub pending_swap: bool,
    pending_requests: HashMap<Output, LayoutRequestId>,
    fulfilled_requests: HashMap<Output, LayoutRequestId>,
    current_id: LayoutRequestId,
}

impl LayoutState {
    fn next_id(&mut self) -> LayoutRequestId {
        self.current_id.0 += 1;
        self.current_id
    }
}

impl Pinnacle {
    pub fn request_layout(&mut self, output: &Output) {
        if self
            .outputs
            .get(output)
            .is_some_and(|global| global.is_none())
        {
            return;
        }

        let id = self.layout_state.next_id();
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

        self.layout_state
            .pending_requests
            .insert(output.clone(), id);

        let _ = sender.send(Ok(LayoutResponse {
            request_id: Some(id.0),
            output_name: Some(output.name()),
            window_ids,
            tag_ids,
            output_width: Some(output_width as u32),
            output_height: Some(output_height as u32),
        }));
    }
}

impl State {
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
        let Some(output) = OutputName(output_name).output(&self.pinnacle) else {
            anyhow::bail!("Output was invalid");
        };

        let Some(current_pending) = self
            .pinnacle
            .layout_state
            .pending_requests
            .get(&output)
            .copied()
        else {
            anyhow::bail!("attempted to layout without request");
        };

        if current_pending > request_id {
            anyhow::bail!("Attempted to layout but a new request came in");
        }
        if current_pending < request_id {
            anyhow::bail!("Attempted to layout but request is newer");
        }

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

        self.pinnacle.layout_state.pending_requests.remove(&output);
        self.pinnacle
            .layout_state
            .fulfilled_requests
            .insert(output.clone(), current_pending);

        let snapshots = self.backend.with_renderer(|renderer| {
            capture_snapshots_on_output(&mut self.pinnacle, renderer, &output, [])
        });

        let pending_windows = self
            .pinnacle
            .update_windows_with_geometries(&output, geometries);

        output.with_state_mut(|state| {
            if let Some(ts) = state.layout_transaction.as_mut() {
                ts.update_pending(pending_windows);
            } else if let Some((fs_and_up_snapshots, under_fs_snapshots)) = snapshots {
                state.layout_transaction = Some(LayoutTransaction::new(
                    self.pinnacle.loop_handle.clone(),
                    fs_and_up_snapshots,
                    under_fs_snapshots,
                    pending_windows,
                ));
            }
        });

        self.schedule_render(&output);

        self.pinnacle.layout_state.pending_swap = false;

        Ok(())
    }
}
