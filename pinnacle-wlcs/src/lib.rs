use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::os::unix::prelude::OwnedFd;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread::JoinHandle;

use pinnacle::wlcs::WlcsEvent;
use smithay::reexports::calloop;
use smithay::reexports::calloop::channel::Sender;
use wayland_sys::client::{wayland_client_handle, wl_display, wl_proxy};
use wayland_sys::common::wl_fixed_t;
use wayland_sys::ffi_dispatch;
use wlcs::ffi_display_server_api::WlcsServerIntegration;
use wlcs::ffi_wrappers::wlcs_server;
use wlcs::{self, wlcs_server_integration, Wlcs};

wlcs_server_integration!(PinnacleDisplayServerHandle);

static DEVICE_ID: AtomicU32 = AtomicU32::new(0);

struct PinnacleDisplayServerHandle {
    server: Option<(Sender<WlcsEvent>, JoinHandle<()>)>,
}

impl Wlcs for PinnacleDisplayServerHandle {
    type Pointer = WlcsPointerHandle;

    type Touch = WlcsTouchHandle;

    fn new() -> Self {
        Self { server: None }
    }

    fn start(&mut self) {
        let (sender, recv) = calloop::channel::channel::<WlcsEvent>();
        let join_handle = start_pinnacle_wlcs(recv);
        self.server = Some((sender, join_handle));
    }

    fn stop(&mut self) {
        if let Some((sender, join)) = self.server.take() {
            let _ = sender.send(WlcsEvent::Exit);
            let _ = join.join();
        }
    }

    fn create_client_socket(&self) -> io::Result<OwnedFd> {
        if let Some((sender, _)) = &self.server {
            if let Ok((client_side, server_side)) = UnixStream::pair() {
                if let Err(e) = sender.send(WlcsEvent::NewClient {
                    stream: server_side,
                    client_id: client_side.as_raw_fd(),
                }) {
                    return Err(io::Error::new(io::ErrorKind::ConnectionReset, e));
                }
                return Ok(client_side.into());
            }
        }
        Err(io::Error::from(io::ErrorKind::NotFound))
    }

    fn position_window_absolute(
        &self,
        display: *mut wl_display,
        surface: *mut wl_proxy,
        x: i32,
        y: i32,
    ) {
        // SAFETY: No clue lol just copied this from wlcs_anvil
        let client_id =
            unsafe { ffi_dispatch!(wayland_client_handle(), wl_display_get_fd, display) };
        let surface_id =
            unsafe { ffi_dispatch!(wayland_client_handle(), wl_proxy_get_id, surface) };
        if let Some((sender, _)) = &self.server {
            let _ = sender.send(WlcsEvent::PositionWindow {
                client_id,
                surface_id,
                location: (x, y).into(),
            });
        }
    }

    fn create_pointer(&mut self) -> Option<Self::Pointer> {
        let Some(ref server) = self.server else {
            return None;
        };
        Some(WlcsPointerHandle {
            device_id: DEVICE_ID.fetch_add(1, Ordering::Relaxed),
            sender: server.0.clone(),
        })
    }

    fn create_touch(&mut self) -> Option<Self::Touch> {
        todo!()
    }

    fn get_descriptor(&self) -> &wlcs::ffi_display_server_api::WlcsIntegrationDescriptor {
        todo!()
    }
}

struct WlcsPointerHandle {
    device_id: u32,
    sender: Sender<WlcsEvent>,
}

impl wlcs::Pointer for WlcsPointerHandle {
    fn move_absolute(&mut self, x: wl_fixed_t, y: wl_fixed_t) {
        todo!()
    }

    fn move_relative(&mut self, dx: wl_fixed_t, dy: wl_fixed_t) {
        todo!()
    }

    fn button_up(&mut self, button: i32) {
        todo!()
    }

    fn button_down(&mut self, button: i32) {
        todo!()
    }
}

struct WlcsTouchHandle {
    device_id: u32,
    sender: Sender<WlcsEvent>,
}

impl wlcs::Touch for WlcsTouchHandle {
    fn touch_down(&mut self, x: wl_fixed_t, y: wl_fixed_t) {
        todo!()
    }

    fn touch_move(&mut self, x: wl_fixed_t, y: wl_fixed_t) {
        todo!()
    }

    fn touch_up(&mut self) {
        todo!()
    }
}

fn start_pinnacle_wlcs(channel: calloop::channel::Channel<WlcsEvent>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let (state, event_loop) = pinnacle::backend::winit::setup_winit(true, None, channel)
            .expect("failed to start winit for wlcs");
    })
}
