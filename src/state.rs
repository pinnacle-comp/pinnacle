use std::ffi::OsString;

use smithay::{
    desktop::{Space, Window},
    input::{pointer::CursorImageStatus, SeatState},
    reexports::{
        calloop::{LoopHandle, LoopSignal},
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::wl_surface::WlSurface,
            Display,
        },
    },
    utils::{Clock, Logical, Monotonic, Point},
    wayland::{
        compositor::{CompositorClientState, CompositorState},
        data_device::DataDeviceState,
        fractional_scale::FractionalScaleManagerState,
        output::OutputManagerState,
        seat::WaylandFocus,
        shell::xdg::XdgShellState,
        shm::ShmState,
        viewporter::ViewporterState,
    },
};

use crate::backend::{winit::WinitData, Backend};

pub struct State<B: Backend> {
    pub backend_data: B,
    pub loop_signal: LoopSignal,
    pub loop_handle: LoopHandle<'static, CalloopData>,
    pub clock: Clock<Monotonic>,
    pub compositor_state: CompositorState,
    pub data_device_state: DataDeviceState,
    pub seat_state: SeatState<Self>,
    pub shm_state: ShmState,
    pub space: Space<Window>,
    pub cursor_status: CursorImageStatus,
    pub pointer_location: Point<f64, Logical>,
    pub output_manager_state: OutputManagerState,
    pub xdg_shell_state: XdgShellState,
    pub viewporter_state: ViewporterState,
    pub fractional_scale_manager_state: FractionalScaleManagerState,

    pub move_mode: bool,
    pub socket_name: OsString,
}

pub struct CalloopData {
    pub display: Display<State<WinitData>>,
    pub state: State<WinitData>,
}

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}
impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}

    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}

    // fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {}
}

impl<B: Backend> State<B> {
    pub fn window_for_surface(&self, surface: &WlSurface) -> Option<Window> {
        self.space
            .elements()
            .find(|window| window.wl_surface().map(|s| s == *surface).unwrap_or(false))
            .cloned()
    }
}
