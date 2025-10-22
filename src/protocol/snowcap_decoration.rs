use std::sync::{Arc, Mutex, atomic::Ordering};

use smithay::{
    reexports::{
        wayland_protocols::ext::foreign_toplevel_list::v1::server::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
        wayland_server::{
            Client, DisplayHandle, GlobalDispatch, Resource, backend::GlobalId,
            protocol::wl_surface::WlSurface,
        },
    },
    utils::{IsAlive, Logical, Point, SERIAL_COUNTER, Serial, Size},
    wayland::compositor::{self, Cacheable},
};
use snowcap_protocols::snowcap_decoration_v1::server::{
    snowcap_decoration_manager_v1::SnowcapDecorationManagerV1,
    snowcap_decoration_surface_v1::SnowcapDecorationSurfaceV1,
};

use crate::protocol::snowcap_decoration::handlers::SnowcapDecorationSurfaceUserData;

pub mod handlers;

const DECORATION_SURFACE_ROLE: &str = "snowcap_decoration_surface_v1";

pub type DecorationSurfaceData = Mutex<DecorationSurfaceAttributes>;

pub struct DecorationSurfaceAttributes {
    surface: SnowcapDecorationSurfaceV1,
    pub initial_configure_sent: bool,
    pending_configures: Vec<DecorationSurfaceConfigure>,
    pub server_pending: Option<DecorationSurfaceState>,
    pub last_acked: Option<DecorationSurfaceConfigure>,
}

impl DecorationSurfaceAttributes {
    fn new(surface: SnowcapDecorationSurfaceV1) -> Self {
        Self {
            surface,
            initial_configure_sent: false,
            pending_configures: Vec::new(),
            server_pending: None,
            last_acked: None,
        }
    }

    fn ack_configure(&mut self, serial: Serial) -> Option<DecorationSurfaceConfigure> {
        let configure = self
            .pending_configures
            .iter()
            .find(|configure| configure.serial == serial)
            .cloned()?;

        self.last_acked = Some(configure.clone());

        self.pending_configures.retain(|c| c.serial > serial);

        Some(configure)
    }

    fn reset(&mut self) {
        self.initial_configure_sent = false;
        self.pending_configures = Vec::new();
        self.server_pending = None;
        self.last_acked = None;
    }

    fn current_server_state(&self) -> DecorationSurfaceState {
        self.pending_configures
            .last()
            .map(|c| &c.state)
            .or(self.last_acked.as_ref().map(|c| &c.state))
            .cloned()
            .unwrap_or_default()
    }

    fn has_pending_changes(&self) -> bool {
        self.server_pending
            .as_ref()
            .map(|s| *s != self.current_server_state())
            .unwrap_or(false)
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Bounds {
    pub top: u32,
    pub bottom: u32,
    pub left: u32,
    pub right: u32,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DecorationSurfaceState {
    /// The size of the decorated toplevel
    pub toplevel_size: Option<Size<i32, Logical>>,
}

#[derive(Clone, Default, Debug)]
pub struct DecorationSurfaceCachedState {
    pub location: Point<i32, Logical>,
    pub bounds: Bounds,
    pub z_index: i32,
    pub last_acked: Option<DecorationSurfaceConfigure>,
}

impl Cacheable for DecorationSurfaceCachedState {
    fn commit(&mut self, _dh: &smithay::reexports::wayland_server::DisplayHandle) -> Self {
        self.clone()
    }

    fn merge_into(self, into: &mut Self, _dh: &smithay::reexports::wayland_server::DisplayHandle) {
        *into = self
    }
}

#[derive(Debug, Clone)]
pub struct DecorationSurfaceConfigure {
    pub state: DecorationSurfaceState,
    pub serial: Serial,
}

#[derive(Debug, Clone)]
pub struct SnowcapDecorationState {
    known_decorations: Arc<Mutex<Vec<DecorationSurface>>>,
    decoration_global: GlobalId,
}

pub struct SnowcapDecorationGlobalData {
    filter: Box<dyn Fn(&Client) -> bool + Send + Sync + 'static>,
}

impl SnowcapDecorationState {
    pub fn new<D>(display: &DisplayHandle) -> Self
    where
        D: GlobalDispatch<SnowcapDecorationManagerV1, SnowcapDecorationGlobalData> + 'static,
    {
        Self::new_with_filter::<D, _>(display, |_| true)
    }

    pub fn new_with_filter<D, F>(display: &DisplayHandle, filter: F) -> Self
    where
        D: GlobalDispatch<SnowcapDecorationManagerV1, SnowcapDecorationGlobalData> + 'static,
        F: Fn(&Client) -> bool + Send + Sync + 'static,
    {
        let decoration_global = display.create_global::<D, SnowcapDecorationManagerV1, _>(
            1,
            SnowcapDecorationGlobalData {
                filter: Box::new(filter),
            },
        );

        Self {
            known_decorations: Default::default(),
            decoration_global,
        }
    }

    pub fn decoration_global(&self) -> GlobalId {
        self.decoration_global.clone()
    }
}

pub trait SnowcapDecorationHandler {
    fn decoration_state(&mut self) -> &mut SnowcapDecorationState;

    fn new_decoration_surface(
        &mut self,
        surface: DecorationSurface,
        handle: ExtForeignToplevelHandleV1,
    );

    fn decoration_destroyed(&mut self, surface: DecorationSurface) {
        let _ = surface;
    }

    fn bounds_changed(&mut self, surface: DecorationSurface) {
        let _ = surface;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecorationSurface {
    wl_surface: WlSurface,
    decoration_surface: SnowcapDecorationSurfaceV1,
}

impl IsAlive for DecorationSurface {
    fn alive(&self) -> bool {
        let decoration_alive = self
            .decoration_surface
            .data::<SnowcapDecorationSurfaceUserData>()
            .unwrap()
            .alive_tracker
            .load(Ordering::Acquire);
        self.wl_surface.alive() && decoration_alive
    }
}

impl DecorationSurface {
    fn get_pending_state(
        &self,
        attributes: &mut DecorationSurfaceAttributes,
    ) -> Option<DecorationSurfaceState> {
        if !attributes.initial_configure_sent {
            return Some(
                attributes
                    .server_pending
                    .take()
                    .unwrap_or_else(|| attributes.current_server_state().clone()),
            );
        }

        if !attributes.has_pending_changes() {
            return None;
        }

        attributes.server_pending.take()
    }

    pub fn send_pending_configure(&self) -> Option<Serial> {
        if self.has_pending_changes() {
            Some(self.send_configure())
        } else {
            None
        }
    }

    pub fn send_configure(&self) -> Serial {
        let configure = compositor::with_states(&self.wl_surface, |states| {
            let mut attrs = states
                .data_map
                .get::<Mutex<DecorationSurfaceAttributes>>()
                .unwrap()
                .lock()
                .unwrap();

            let state = self
                .get_pending_state(&mut attrs)
                .unwrap_or_else(|| attrs.current_server_state().clone());

            let configure = DecorationSurfaceConfigure {
                state,
                serial: SERIAL_COUNTER.next_serial(),
            };

            attrs.pending_configures.push(configure.clone());
            attrs.initial_configure_sent = true;

            configure
        });

        let (width, height) = configure.state.toplevel_size.unwrap_or_default().into();
        let serial = configure.serial;
        self.decoration_surface
            .configure(serial.into(), width as u32, height as u32);
        serial
    }

    pub fn wl_surface(&self) -> &WlSurface {
        &self.wl_surface
    }

    pub fn with_pending_state<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&mut DecorationSurfaceState) -> T,
    {
        compositor::with_states(&self.wl_surface, |states| {
            let mut attributes = states
                .data_map
                .get::<Mutex<DecorationSurfaceAttributes>>()
                .unwrap()
                .lock()
                .unwrap();
            if attributes.server_pending.is_none() {
                attributes.server_pending = Some(attributes.current_server_state().clone());
            }

            let server_pending = attributes.server_pending.as_mut().unwrap();
            f(server_pending)
        })
    }

    pub fn has_pending_changes(&self) -> bool {
        compositor::with_states(&self.wl_surface, |states| {
            let attributes = states
                .data_map
                .get::<Mutex<DecorationSurfaceAttributes>>()
                .unwrap()
                .lock()
                .unwrap();

            !attributes.initial_configure_sent || attributes.has_pending_changes()
        })
    }

    pub fn decoration_surface(&self) -> &SnowcapDecorationSurfaceV1 {
        &self.decoration_surface
    }

    pub fn with_cached_state<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&DecorationSurfaceCachedState) -> T,
    {
        compositor::with_states(&self.wl_surface, |states| {
            let mut guard = states.cached_state.get::<DecorationSurfaceCachedState>();
            f(guard.current())
        })
    }

    pub fn with_committed_state<F, T>(&self, f: F) -> T
    where
        F: FnOnce(Option<&DecorationSurfaceState>) -> T,
    {
        self.with_cached_state(move |state| f(state.last_acked.as_ref().map(|c| &c.state)))
    }
}

#[macro_export]
macro_rules! delegate_snowcap_decoration {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        type __SnowcapDecorationManagerV1 =
            ::snowcap_protocols::snowcap_decoration_v1::server::snowcap_decoration_manager_v1::SnowcapDecorationManagerV1;
        type __SnowcapDecorationSurfaceV1 =
            ::snowcap_protocols::snowcap_decoration_v1::server::snowcap_decoration_surface_v1::SnowcapDecorationSurfaceV1;

        ::smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            __SnowcapDecorationManagerV1: ()
        ] => $crate::protocol::snowcap_decoration::SnowcapDecorationState);
        ::smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            __SnowcapDecorationSurfaceV1: $crate::protocol::snowcap_decoration::handlers::SnowcapDecorationSurfaceUserData
        ] => $crate::protocol::snowcap_decoration::SnowcapDecorationState);

        ::smithay::reexports::wayland_server::delegate_global_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            __SnowcapDecorationManagerV1: $crate::protocol::snowcap_decoration::SnowcapDecorationGlobalData
        ] => $crate::protocol::snowcap_decoration::SnowcapDecorationState);
    };
}
