pub(crate) mod config;
mod input_backend;
mod main_loop;

use std::{
    io,
    os::{
        fd::{AsRawFd, OwnedFd},
        unix::net::UnixStream,
    },
    sync::{
        atomic::{AtomicU32, Ordering},
        Once,
    },
    thread::{spawn, JoinHandle},
};

use smithay::{
    reexports::calloop::channel::{channel, Sender},
    utils::{Logical, Point},
};
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};
use wayland_sys::{
    client::{wayland_client_handle, wl_display, wl_proxy},
    common::{wl_fixed_t, wl_fixed_to_double},
    ffi_dispatch,
};
use wlcs::{
    extension_list,
    ffi_display_server_api::{
        WlcsExtensionDescriptor, WlcsIntegrationDescriptor, WlcsServerIntegration,
    },
    ffi_wrappers::wlcs_server,
    wlcs_server_integration, Pointer, Touch, Wlcs,
};

wlcs_server_integration!(PinnacleHandle);

#[derive(Debug)]
pub enum WlcsEvent {
    Stop,
    NewClient {
        stream: UnixStream,
        client_id: i32,
    },
    PositionWindow {
        client_id: i32,
        surface_id: u32,
        location: Point<i32, Logical>,
    },
    NewPointer {
        device_id: u32,
    },
    PointerMoveRelative {
        device_id: u32,
        delta: Point<f64, Logical>,
    },
    PointerMoveAbsolute {
        device_id: u32,
        position: Point<f64, Logical>,
    },
    PointerButton {
        device_id: u32,
        button_id: i32,
        pressed: bool,
    },
    NewTouch {
        device_id: u32,
    },
    TouchDown {
        device_id: u32,
        position: Point<f64, Logical>,
    },
    TouchMove {
        device_id: u32,
        position: Point<f64, Logical>,
    },
    TouchUp {
        device_id: u32,
    },
}

struct PinnacleConnection {
    sender: Sender<WlcsEvent>,
    join: JoinHandle<()>,
}

impl PinnacleConnection {
    fn start() -> Self {
        let (sender, receiver) = channel();
        let join = spawn(move || main_loop::run(receiver));
        Self { sender, join }
    }
}

struct PinnacleHandle {
    server_conn: Option<PinnacleConnection>,
}

static SUPPORTED_EXTENSIONS: &[WlcsExtensionDescriptor] = extension_list!(
    // Skip reasons:
    //   5 Missing extension: gtk_primary_selection_device_manager>= 1
    //   1 Missing extension: wlcs_non_existent_extension>= 1
    //  89 Missing extension: wl_shell>= 2
    //   1 Missing extension: xdg_not_really_an_extension>= 1
    //  30 Missing extension: zwlr_foreign_toplevel_manager_v1>= 1
    //  12 Missing extension: zwlr_virtual_pointer_manager_v1>= 1
    //  15 Missing extension: zwp_pointer_constraints_v1>= 1
    //   3 Missing extension: zwp_relative_pointer_manager_v1>= 1
    //  12 Missing extension: zwp_text_input_manager_v2>= 1
    //  11 Missing extension: zwp_text_input_manager_v3>= 1
    // 180 Missing extension: zxdg_shell_v6>= 1

    // mostly from https://github.com/Smithay/smithay/issues/781
    ("wl_compositor", 6),
    ("wl_subcompositor", 1),
    ("wl_shm", 1),
    ("wl_data_device_manager", 3),
    ("wl_seat", 9),
    ("wl_output", 4),
    ("wp_presentation", 1),
    ("wp_viewporter", 1),
    ("xdg_shell", 6),
    ("linux-dmabuf-v1", 5),
    ("security-context", 1),
    ("zwp_pointer_constraints_v1", 1),
    ("zwp_relative_pointer_manager_v1", 1),
);

static DESCRIPTOR: WlcsIntegrationDescriptor = WlcsIntegrationDescriptor {
    version: 1,
    num_extensions: SUPPORTED_EXTENSIONS.len(),
    supported_extensions: SUPPORTED_EXTENSIONS.as_ptr(),
};

static DEVICE_ID: AtomicU32 = AtomicU32::new(0);

fn new_device_id() -> u32 {
    DEVICE_ID.fetch_add(1, Ordering::Relaxed)
}

fn init() {
    let env_filter = EnvFilter::try_from_default_env();

    let stdout_env_filter = env_filter.unwrap_or_else(|_| EnvFilter::new("info"));
    let stdout_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_writer(std::io::stdout)
        .with_filter(stdout_env_filter);

    tracing_subscriber::registry().with(stdout_layer).init();
}

static INIT_ONCE: Once = Once::new();

impl Wlcs for PinnacleHandle {
    type Pointer = PointerHandle;
    type Touch = TouchHandle;

    fn new() -> Self {
        INIT_ONCE.call_once(init);
        Self { server_conn: None }
    }

    fn start(&mut self) {
        self.server_conn = Some(PinnacleConnection::start());
    }

    fn stop(&mut self) {
        if let Some(conn) = self.server_conn.take() {
            let _ = conn.sender.send(WlcsEvent::Stop);
            let _ = conn.join.join();
        }
    }

    fn create_client_socket(&self) -> io::Result<OwnedFd> {
        info!("new client start");
        let conn = self
            .server_conn
            .as_ref()
            .ok_or(io::Error::from(io::ErrorKind::NotFound))?;

        let (client, server) = UnixStream::pair()?;

        conn.sender
            .send(WlcsEvent::NewClient {
                stream: server,
                client_id: client.as_raw_fd(),
            })
            .map_err(|e| {
                warn!("failed to send NewClient event");
                io::Error::new(io::ErrorKind::ConnectionReset, e)
            })?;

        info!("new client end");
        Ok(client.into())
    }

    fn position_window_absolute(
        &self,
        display: *mut wl_display,
        surface: *mut wl_proxy,
        x: i32,
        y: i32,
    ) {
        if let Some(conn) = &self.server_conn {
            let client_id =
                unsafe { ffi_dispatch!(wayland_client_handle(), wl_display_get_fd, display) };
            let surface_id =
                unsafe { ffi_dispatch!(wayland_client_handle(), wl_proxy_get_id, surface) };
            conn.sender
                .send(WlcsEvent::PositionWindow {
                    client_id,
                    surface_id,
                    location: (x, y).into(),
                })
                .expect("failed to send position_window_absolute");
        }
    }

    fn create_pointer(&mut self) -> Option<Self::Pointer> {
        let device_id = new_device_id();
        self.server_conn
            .as_ref()
            .map(|conn| conn.sender.clone())
            .map(|sender| {
                sender
                    .send(WlcsEvent::NewPointer { device_id })
                    .expect("failed to send new_pointer");
                PointerHandle { device_id, sender }
            })
    }

    fn create_touch(&mut self) -> Option<Self::Touch> {
        let device_id = new_device_id();
        self.server_conn
            .as_ref()
            .map(|conn| conn.sender.clone())
            .map(|sender| {
                sender
                    .send(WlcsEvent::NewTouch { device_id })
                    .expect("failed to send new_touch");
                TouchHandle { device_id, sender }
            })
    }

    fn get_descriptor(&self) -> &WlcsIntegrationDescriptor {
        &DESCRIPTOR
    }
}

struct PointerHandle {
    device_id: u32,
    sender: Sender<WlcsEvent>,
}

impl Pointer for PointerHandle {
    fn move_absolute(&mut self, x: wl_fixed_t, y: wl_fixed_t) {
        self.sender
            .send(WlcsEvent::PointerMoveAbsolute {
                device_id: self.device_id,
                position: (wl_fixed_to_double(x), wl_fixed_to_double(y)).into(),
            })
            .expect("failed to send move_absolute");
    }

    fn move_relative(&mut self, dx: wl_fixed_t, dy: wl_fixed_t) {
        self.sender
            .send(WlcsEvent::PointerMoveRelative {
                device_id: self.device_id,
                delta: (wl_fixed_to_double(dx), wl_fixed_to_double(dy)).into(),
            })
            .expect("failed to send move_relative");
    }

    fn button_up(&mut self, button: i32) {
        self.sender
            .send(WlcsEvent::PointerButton {
                device_id: self.device_id,
                button_id: button,
                pressed: false,
            })
            .expect("failed to send button_up");
    }

    fn button_down(&mut self, button: i32) {
        self.sender
            .send(WlcsEvent::PointerButton {
                device_id: self.device_id,
                button_id: button,
                pressed: true,
            })
            .expect("failed to send button_down");
    }
}

struct TouchHandle {
    device_id: u32,
    sender: Sender<WlcsEvent>,
}

impl Touch for TouchHandle {
    fn touch_down(&mut self, x: wl_fixed_t, y: wl_fixed_t) {
        self.sender
            .send(WlcsEvent::TouchDown {
                device_id: self.device_id,
                position: (wl_fixed_to_double(x), wl_fixed_to_double(y)).into(),
            })
            .expect("failed to send touch_down");
    }

    fn touch_move(&mut self, x: wl_fixed_t, y: wl_fixed_t) {
        self.sender
            .send(WlcsEvent::TouchMove {
                device_id: self.device_id,
                position: (wl_fixed_to_double(x), wl_fixed_to_double(y)).into(),
            })
            .expect("failed to send touch_move");
    }

    fn touch_up(&mut self) {
        self.sender
            .send(WlcsEvent::TouchUp {
                device_id: self.device_id,
            })
            .expect("failed to send touch_up");
    }
}
