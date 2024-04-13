use wayland_sys::{client::{wl_display, wl_proxy}, common::wl_fixed_t};
use wlcs::{
    ffi_display_server_api::WlcsServerIntegration, ffi_wrappers::wlcs_server,
    wlcs_server_integration, Pointer, Touch, Wlcs,
};

wlcs_server_integration!(PinnacleHandle);

struct PinnacleHandle {
    // server: Option<(Sender<WlcsEvent>, JoinHandle<()>)>,
}


impl Wlcs for PinnacleHandle {
    type Pointer = PointerHandle;
    type Touch = TouchHandle;

    fn new() -> Self {
        todo!()
    }

    fn start(&mut self) {
        todo!()
    }

    fn stop(&mut self) {
        todo!()
    }

    fn create_client_socket(&self) -> std::io::Result<std::os::unix::prelude::OwnedFd> {
        todo!()
    }

    fn position_window_absolute(
        &self,
        display: *mut wl_display,
        surface: *mut wl_proxy,
        x: i32,
        y: i32,
    ) {
        todo!()
    }

    fn create_pointer(&mut self) -> Option<Self::Pointer> {
        todo!()
    }

    fn create_touch(&mut self) -> Option<Self::Touch> {
        todo!()
    }

    fn get_descriptor(&self) -> &wlcs::ffi_display_server_api::WlcsIntegrationDescriptor {
        todo!()
    }

    fn start_on_this_thread(&self, _event_loop: *mut wayland_sys::server::wl_event_loop) {}
}

struct PointerHandle {}

impl Pointer for PointerHandle {
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

struct TouchHandle {}

impl Touch for TouchHandle {
    fn touch_down(&mut self, x: wayland_sys::common::wl_fixed_t, y: wayland_sys::common::wl_fixed_t) {
        todo!()
    }

    fn touch_move(&mut self, x: wayland_sys::common::wl_fixed_t, y: wayland_sys::common::wl_fixed_t) {
        todo!()
    }

    fn touch_up(&mut self) {
        todo!()
    }
}
