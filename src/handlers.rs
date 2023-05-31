use smithay::{
    backend::renderer::utils,
    delegate_compositor, delegate_data_device, delegate_output, delegate_seat, delegate_shm,
    delegate_xdg_shell,
    desktop::Window,
    input::{pointer::CursorImageStatus, Seat, SeatHandler, SeatState},
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge,
        wayland_server::{
            protocol::{wl_buffer::WlBuffer, wl_seat::WlSeat, wl_surface::WlSurface},
            Client,
        },
    },
    utils::Serial,
    wayland::{
        buffer::BufferHandler,
        compositor::{self, CompositorClientState, CompositorHandler, CompositorState},
        data_device::{
            ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler,
        },
        shell::xdg::{
            PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
            XdgToplevelSurfaceData,
        },
        shm::{ShmHandler, ShmState},
    },
};

use crate::{
    layout::{
        automatic::{MasterStack, MasterStackSide},
        Layout,
    },
    ClientState, State,
};

impl BufferHandler for State {
    fn buffer_destroyed(&mut self, _buffer: &WlBuffer) {}
}

impl CompositorHandler for State {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        // println!("CompositorHandler commit()");

        utils::on_commit_buffer_handler::<Self>(surface);

        if !compositor::is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = compositor::get_parent(&root) {
                root = parent;
            }
            if let Some(window) = self
                .space
                .elements()
                .find(|w| w.toplevel().wl_surface() == &root)
            {
                // println!("window.on_commit");
                window.on_commit();
            }
        };

        if let Some(window) = self
            .space
            .elements()
            .find(|w| w.toplevel().wl_surface() == surface)
            .cloned()
        {
            let initial_configure_sent = compositor::with_states(surface, |states| {
                states
                    .data_map
                    .get::<XdgToplevelSurfaceData>()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .initial_configure_sent
            });
            // println!("initial_configure_sent is {}", initial_configure_sent);

            if !initial_configure_sent {
                // println!("initial configure");
                window.toplevel().send_configure();
            }
        }

        crate::grab::resize_grab::handle_commit(&mut self.space, surface);
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }
}
delegate_compositor!(State);

impl ClientDndGrabHandler for State {}
impl ServerDndGrabHandler for State {}

impl DataDeviceHandler for State {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}
delegate_data_device!(State);

impl SeatHandler for State {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn cursor_image(&mut self, _seat: &Seat<Self>, image: CursorImageStatus) {
        self.cursor_status = image;
    }

    fn focus_changed(&mut self, _seat: &Seat<Self>, _focused: Option<&Self::KeyboardFocus>) {}
}
delegate_seat!(State);

impl ShmHandler for State {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}
delegate_shm!(State);

impl XdgShellHandler for State {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    // TODO: this shouldn't call send_configure
    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = Window::new(surface);
        self.space.map_element(window, (0, 0), true);

        let windows: Vec<Window> = self.space.elements().cloned().collect();
        let layout = MasterStack {
            windows: Vec::new(),
            side: MasterStackSide::Left,
        };

        layout.layout_windows(self, windows);

        // TODO: refresh all window geometries
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        let windows: Vec<Window> = self.space.elements().cloned().collect();
        let layout = MasterStack {
            windows: Vec::new(),
            side: MasterStackSide::Left,
        };

        layout.layout_windows(self, windows);
        // TODO: refresh geometries
    }

    fn new_popup(&mut self, surface: PopupSurface, positioner: PositionerState) {}

    fn move_request(&mut self, surface: ToplevelSurface, seat: WlSeat, serial: Serial) {
        crate::xdg::request::move_request(
            self,
            &surface,
            &Seat::from_resource(&seat).unwrap(),
            serial,
        );
    }

    fn resize_request(
        &mut self,
        surface: ToplevelSurface,
        seat: WlSeat,
        serial: Serial,
        edges: ResizeEdge,
    ) {
        const BUTTON_LEFT: u32 = 0x110;
        crate::xdg::request::resize_request(
            self,
            &surface,
            &Seat::from_resource(&seat).unwrap(),
            serial,
            edges,
            BUTTON_LEFT,
        );
    }

    fn grab(&mut self, surface: PopupSurface, seat: WlSeat, serial: Serial) {}

    // TODO: impl the rest of the fns in XdgShellHandler
}
delegate_xdg_shell!(State);

delegate_output!(State);
