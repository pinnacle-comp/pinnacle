use std::{
    cell::RefCell,
    sync::atomic::{AtomicBool, Ordering},
};

use smithay::{
    reexports::wayland_protocols::ext::foreign_toplevel_list::v1::server::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    wayland::foreign_toplevel_list::ForeignToplevelHandle,
};

use crate::{
    decoration::DecorationSurface,
    delegate_snowcap_decoration,
    protocol::snowcap_decoration::{SnowcapDecorationHandler, SnowcapDecorationState},
    state::{State, WithState},
};

impl SnowcapDecorationHandler for State {
    fn decoration_state(&mut self) -> &mut SnowcapDecorationState {
        &mut self.pinnacle.snowcap_decoration_state
    }

    fn new_decoration_surface(
        &mut self,
        surface: crate::protocol::snowcap_decoration::DecorationSurface,
        handle: ExtForeignToplevelHandleV1,
    ) {
        let Some(window) = self
            .pinnacle
            .windows
            .iter()
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

        surface.with_pending_state(|state| {
            state.toplevel_size = Some(window.geometry().size);
        });
        surface.send_configure();

        let decoration_surface = DecorationSurface::new(surface);

        window.with_state_mut(|state| {
            state.decoration_surface = Some(decoration_surface);
        });
    }

    fn decoration_destroyed(
        &mut self,
        surface: crate::protocol::snowcap_decoration::DecorationSurface,
    ) {
        for win in self.pinnacle.windows.iter() {
            win.with_state_mut(|state| {
                if state
                    .decoration_surface
                    .as_ref()
                    .is_some_and(|deco| deco.decoration_surface() == &surface)
                {
                    state.decoration_surface.take();
                }
            });
        }
    }

    fn bounds_changed(&mut self, surface: crate::protocol::snowcap_decoration::DecorationSurface) {
        for win in self.pinnacle.windows.iter() {
            win.with_state_mut(|state| {
                if let Some(deco) = state.decoration_surface.as_ref()
                    && deco.decoration_surface() == &surface
                {
                    deco.with_state(|state| {
                        state.bounds_changed.store(true, Ordering::Relaxed);
                    });
                }
            });
        }
    }
}

delegate_snowcap_decoration!(State);

#[derive(Debug, Default)]
pub struct DecorationSurfaceState {
    pub bounds_changed: AtomicBool,
}

impl WithState for DecorationSurface {
    type State = DecorationSurfaceState;

    fn with_state<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&Self::State) -> T,
    {
        let state = self
            .user_data()
            .get_or_insert(RefCell::<DecorationSurfaceState>::default);
        func(&state.borrow())
    }

    fn with_state_mut<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&mut Self::State) -> T,
    {
        let state = self
            .user_data()
            .get_or_insert(RefCell::<DecorationSurfaceState>::default);
        func(&mut state.borrow_mut())
    }
}
