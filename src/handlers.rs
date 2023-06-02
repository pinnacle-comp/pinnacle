use smithay::{
    backend::renderer::utils,
    delegate_compositor, delegate_data_device, delegate_output, delegate_seat, delegate_shm,
    delegate_xdg_shell,
    desktop::Window,
    input::{pointer::CursorImageStatus, Seat, SeatHandler, SeatState},
    reexports::{
        calloop::Interest,
        wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge,
        wayland_server::{
            protocol::{wl_buffer::WlBuffer, wl_seat::WlSeat, wl_surface::WlSurface},
            Client, Resource,
        },
    },
    utils::Serial,
    wayland::{
        buffer::BufferHandler,
        compositor::{
            self, BufferAssignment, CompositorClientState, CompositorHandler, CompositorState,
            SurfaceAttributes,
        },
        data_device::{
            ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler,
        },
        dmabuf,
        shell::xdg::{
            Configure, PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler,
            XdgShellState, XdgToplevelSurfaceData,
        },
        shm::{ShmHandler, ShmState},
    },
};

use crate::{
    backend::Backend,
    layout::{
        automatic::{MasterStack, MasterStackSide},
        Layout,
    },
    state::{ClientState, State},
};

impl<B: Backend> BufferHandler for State<B> {
    fn buffer_destroyed(&mut self, _buffer: &WlBuffer) {}
}

impl<B: Backend> CompositorHandler for State<B> {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn new_surface(&mut self, surface: &WlSurface) {
        // yanked straight from anvil
        compositor::add_pre_commit_hook::<Self, _>(surface, |state, _display_handle, surface| {
            let maybe_dmabuf = compositor::with_states(surface, |surface_data| {
                surface_data
                    .cached_state
                    .pending::<SurfaceAttributes>()
                    .buffer
                    .as_ref()
                    .and_then(|assignment| match assignment {
                        BufferAssignment::NewBuffer(buffer) => dmabuf::get_dmabuf(buffer).ok(),
                        _ => None,
                    })
            });
            if let Some(dmabuf) = maybe_dmabuf {
                if let Ok((blocker, source)) = dmabuf.generate_blocker(Interest::READ) {
                    let client = surface.client().unwrap();
                    let res = state.loop_handle.insert_source(source, move |_, _, data| {
                        data.state
                            .client_compositor_state(&client)
                            .blocker_cleared(&mut data.state, &data.display.handle());
                        Ok(())
                    });
                    if res.is_ok() {
                        compositor::add_blocker(surface, blocker);
                    }
                }
            }
        });
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
                println!("initial configure");
                window.toplevel().send_configure();
                // println!(
                //     "ensured_configured: {}",
                //     window.toplevel().ensure_configured()
                // );
            }
        }

        crate::grab::resize_grab::handle_commit(&mut self.space, surface);
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }
}
delegate_compositor!(@<B: Backend> State<B>);

impl<B: Backend> ClientDndGrabHandler for State<B> {}
impl<B: Backend> ServerDndGrabHandler for State<B> {}

impl<B: Backend> DataDeviceHandler for State<B> {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}
delegate_data_device!(@<B: Backend> State<B>);

impl<B: Backend + 'static> SeatHandler for State<B> {
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
delegate_seat!(@<B: Backend> State<B>);

impl<B: Backend> ShmHandler for State<B> {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}
delegate_shm!(@<B: Backend> State<B>);

impl<B: Backend> XdgShellHandler for State<B> {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = Window::new(surface);
        self.space.map_element(window, (0, 0), true);

        let windows: Vec<Window> = self.space.elements().cloned().collect();
        let layout = MasterStack {
            windows: Vec::new(),
            side: MasterStackSide::Left,
        };

        layout.layout_windows(self, windows);
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        let windows: Vec<Window> = self.space.elements().cloned().collect();
        let layout = MasterStack {
            windows: Vec::new(),
            side: MasterStackSide::Left,
        };

        layout.layout_windows(self, windows);
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

    fn ack_configure(&mut self, surface: WlSurface, configure: Configure) {
        // println!("surface ack_configure: {:?}", configure);
    }

    // TODO: impl the rest of the fns in XdgShellHandler
}
delegate_xdg_shell!(@<B: Backend> State<B>);

delegate_output!(@<B: Backend> State<B>);
