mod backend;
mod grab;
mod handlers;
mod input;
mod layout;
mod pointer;
mod tag;
mod window;
mod xdg;

use std::error::Error;

use backend::{winit::WinitData, Backend};
use smithay::{
    desktop::{Space, Window},
    input::{pointer::CursorImageStatus, SeatState},
    reexports::{
        calloop::{LoopHandle, LoopSignal},
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            Display,
        },
    },
    utils::{Clock, Logical, Monotonic, Point},
    wayland::{
        compositor::{CompositorClientState, CompositorState},
        data_device::DataDeviceState,
        output::OutputManagerState,
        shell::xdg::XdgShellState,
        shm::ShmState,
    },
};

fn main() -> Result<(), Box<dyn Error>> {
    crate::backend::winit::run_winit()?;

    Ok(())
}

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

    pub move_mode: bool,
}

pub struct CalloopData {
    pub display: Display<State<WinitData>>,
    pub state: State<WinitData>,
}

#[derive(Default)]
struct ClientState {
    pub compositor_state: CompositorClientState,
}
impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}

    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}

    // fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {}
}
