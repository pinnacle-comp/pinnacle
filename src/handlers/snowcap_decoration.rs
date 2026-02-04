use std::sync::atomic::Ordering;

use smithay::{
    desktop::PopupKind, reexports::wayland_protocols::ext::foreign_toplevel_list::v1::server::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1, wayland::{compositor, foreign_toplevel_list::ForeignToplevelHandle, shell::xdg}
};
use tracing::warn;

use crate::{
    decoration::DecorationSurface,
    delegate_snowcap_decoration,
    hook::add_decoration_pre_commit_hook,
    protocol::{
        self,
        snowcap_decoration::{SnowcapDecorationHandler, SnowcapDecorationState},
    },
    state::{State, WithState},
};

impl SnowcapDecorationHandler for State {
    fn decoration_state(&mut self) -> &mut SnowcapDecorationState {
        &mut self.pinnacle.snowcap_decoration_state
    }

    fn new_decoration_surface(
        &mut self,
        surface: protocol::snowcap_decoration::DecorationSurface,
        handle: ExtForeignToplevelHandleV1,
    ) {
        let Some(window) = self
            .pinnacle
            .windows
            .iter()
            .chain(
                self.pinnacle
                    .unmapped_windows
                    .iter()
                    .map(|unmapped| &unmapped.window),
            )
            .find(|win| {
                win.with_state(|state| {
                    state
                        .foreign_toplevel_list_handle
                        .as_ref()
                        .is_some_and(|fth| {
                            Some(fth.identifier())
                                == ForeignToplevelHandle::from_resource(&handle)
                                    .map(|fth| fth.identifier())
                        })
                })
            })
            .cloned()
        else {
            return;
        };

        let size = (*window).geometry().size;
        if !size.is_empty() {
            surface.with_pending_state(|state| {
                state.toplevel_size = Some(size);
            });
            surface.send_configure();
        }

        if let Some(dmabuf_hook) = self.pinnacle.dmabuf_hooks.remove(surface.wl_surface()) {
            compositor::remove_pre_commit_hook(surface.wl_surface(), dmabuf_hook);
        }

        let decoration_surface = DecorationSurface::new(surface);

        let hook = add_decoration_pre_commit_hook(&decoration_surface);

        decoration_surface.with_state_mut(|state| state.hook_id = Some(hook));

        window.with_state_mut(|state| {
            state.decoration_surfaces.push(decoration_surface);
        });
    }

    fn decoration_destroyed(&mut self, surface: protocol::snowcap_decoration::DecorationSurface) {
        for win in self.pinnacle.windows.iter().chain(
            self.pinnacle
                .unmapped_windows
                .iter()
                .map(|unmapped| &unmapped.window),
        ) {
            win.with_state_mut(|state| {
                state
                    .decoration_surfaces
                    .retain(|deco| deco.decoration_surface() != &surface);
            });
        }
    }

    fn bounds_changed(&mut self, surface: protocol::snowcap_decoration::DecorationSurface) {
        for win in self.pinnacle.windows.iter() {
            win.with_state_mut(|state| {
                if let Some(deco) = state
                    .decoration_surfaces
                    .iter()
                    .find(|deco| deco.decoration_surface() == &surface)
                {
                    deco.with_state(|state| {
                        state.bounds_changed.store(true, Ordering::Relaxed);
                    });
                }
            });
        }
    }

    fn new_popup(
        &mut self,
        _parent: protocol::snowcap_decoration::DecorationSurface,
        popup: xdg::PopupSurface,
    ) {
        if let Err(err) = self.pinnacle.position_popup(&popup) {
            warn!("Failed to position popup: {err}");
        }

        if let Err(err) = self
            .pinnacle
            .popup_manager
            .track_popup(PopupKind::from(popup))
        {
            warn!("Failed to track popup: {err}");
        }
    }
}

delegate_snowcap_decoration!(State);
