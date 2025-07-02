#![warn(unused)]

use std::{
    os::{
        fd::{AsFd, OwnedFd},
        unix::net::UnixStream,
    },
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};

use calloop_wayland_source::WaylandSource;
use smithay::reexports::{
    calloop::EventLoop,
    wayland_protocols::{
        wp::{
            single_pixel_buffer::v1::client::wp_single_pixel_buffer_manager_v1::WpSinglePixelBufferManagerV1,
            viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter},
        },
        xdg::shell::client::{
            xdg_surface::{self, XdgSurface},
            xdg_toplevel::{self, XdgToplevel},
            xdg_wm_base::{self, XdgWmBase},
        },
    },
};
use tracing::debug;
use wayland_client::{
    delegate_noop,
    globals::GlobalListContents,
    protocol::{
        wl_buffer::WlBuffer,
        wl_callback::{self, WlCallback},
        wl_compositor::WlCompositor,
        wl_display::WlDisplay,
        wl_registry::{self, WlRegistry},
        wl_surface::WlSurface,
    },
    Connection, Dispatch, Proxy, QueueHandle,
};

pub struct Client {
    id: ClientId,
    event_loop: EventLoop<'static, State>,
    state: State,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientId(u32);

static CLIENT_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

impl ClientId {
    fn next() -> Self {
        Self(CLIENT_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

struct State {
    conn: Connection,
    qh: QueueHandle<Self>,
    display: WlDisplay,
    compositor: Option<WlCompositor>,
    xdg_wm_base: Option<XdgWmBase>,
    single_pixel_buffer: Option<WpSinglePixelBufferManagerV1>,
    viewporter: Option<WpViewporter>,
    windows: Vec<Window>,
}

pub struct Window {
    qh: QueueHandle<State>,
    wl_surface: WlSurface,
    xdg_surface: XdgSurface,
    toplevel: XdgToplevel,
    viewport: WpViewport,
    single_pixel_buffer: WpSinglePixelBufferManagerV1,

    current_configure_serial: Option<u32>,
    pending_configure: PendingConfigure,
    pub close_requested: bool,
    pub fullscreen: bool,
    pub maximized: bool,
}

impl Drop for Window {
    fn drop(&mut self) {
        self.toplevel.destroy();
        self.xdg_surface.destroy();
        self.viewport.destroy();
        self.wl_surface.destroy();
    }
}

#[derive(Default, Debug)]
struct PendingConfigure {
    size: Option<(i32, i32)>,
    states: Option<Vec<xdg_toplevel::State>>,
    bounds: Option<(i32, i32)>,
}

impl Client {
    pub fn new(stream: UnixStream) -> Self {
        let conn = Connection::from_socket(stream).unwrap();

        let display = conn.display();

        let event_queue = conn.new_event_queue();

        let qh = event_queue.handle();

        let _registry = display.get_registry(&qh, ());

        let event_loop = EventLoop::try_new().unwrap();

        WaylandSource::new(conn.clone(), event_queue)
            .insert(event_loop.handle())
            .unwrap();

        let state = State {
            conn,
            qh,
            display,
            compositor: None,
            xdg_wm_base: None,
            single_pixel_buffer: None,
            viewporter: None,
            windows: Vec::new(),
        };

        Self {
            id: ClientId::next(),
            event_loop,
            state,
        }
    }

    pub fn window_for_surface(&mut self, surface: &WlSurface) -> &mut Window {
        self.state
            .windows
            .iter_mut()
            .find(|win| &win.wl_surface == surface)
            .unwrap()
    }

    pub fn event_loop_fd(&self) -> OwnedFd {
        self.event_loop.as_fd().try_clone_to_owned().unwrap()
    }

    pub fn send_sync(&self) -> Arc<AtomicBool> {
        self.state.send_sync()
    }

    pub fn dispatch(&mut self) {
        self.event_loop
            .dispatch(Duration::ZERO, &mut self.state)
            .unwrap();
    }

    pub fn id(&self) -> ClientId {
        self.id
    }

    pub fn create_window(&mut self) -> &mut Window {
        self.state.create_window()
    }

    pub fn close_window(&mut self, surface: &WlSurface) {
        self.state.windows.retain(|win| &win.surface() != surface);
    }
}

impl State {
    fn create_window(&mut self) -> &mut Window {
        let wl_surface = self
            .compositor
            .as_ref()
            .unwrap()
            .create_surface(&self.qh, ());
        let xdg_surface =
            self.xdg_wm_base
                .as_ref()
                .unwrap()
                .get_xdg_surface(&wl_surface, &self.qh, ());
        let toplevel = xdg_surface.get_toplevel(&self.qh, ());
        let viewport = self
            .viewporter
            .as_ref()
            .unwrap()
            .get_viewport(&wl_surface, &self.qh, ());

        let window = Window {
            qh: self.qh.clone(),
            single_pixel_buffer: self.single_pixel_buffer.clone().unwrap(),
            wl_surface,
            xdg_surface,
            toplevel,
            viewport,
            current_configure_serial: None,
            pending_configure: Default::default(),
            close_requested: false,
            fullscreen: false,
            maximized: false,
        };

        self.windows.push(window);
        self.windows.last_mut().unwrap()
    }

    fn send_sync(&self) -> Arc<AtomicBool> {
        let wait = Arc::new(AtomicBool::new(false));
        self.display.sync(&self.qh, wait.clone());
        self.conn.flush().unwrap();
        wait
    }
}

impl Window {
    pub fn surface(&self) -> WlSurface {
        self.wl_surface.clone()
    }

    pub fn current_serial(&self) -> Option<u32> {
        self.current_configure_serial
    }

    pub fn commit(&self) {
        debug!(?self.wl_surface, "committing");
        self.wl_surface.commit();
    }

    pub fn ack_and_commit(&mut self) {
        if let Some(current_configure_serial) = self.current_configure_serial.take() {
            debug!(?self.xdg_surface, current_configure_serial, "acking");
            self.xdg_surface.ack_configure(current_configure_serial);
        }

        self.commit();
    }

    pub fn attach_buffer(&self) {
        let buffer =
            self.single_pixel_buffer
                .create_u32_rgba_buffer(0, 0, 0, u32::MAX, &self.qh, ());
        self.wl_surface.attach(Some(&buffer), 0, 0);
    }

    pub fn set_app_id(&self, app_id: &str) {
        self.toplevel.set_app_id(app_id.to_string());
    }

    pub fn set_title(&self, title: &str) {
        self.toplevel.set_title(title.to_string());
    }

    pub fn set_min_size(&self, width: i32, height: i32) {
        self.toplevel.set_min_size(width, height);
    }

    pub fn set_max_size(&self, width: i32, height: i32) {
        self.toplevel.set_max_size(width, height);
    }

    pub fn set_size(&self, width: i32, height: i32) {
        self.viewport.set_destination(width, height);
    }
}

impl Dispatch<WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: <WlRegistry as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                if interface == WlCompositor::interface().name {
                    let version = u32::min(version, WlCompositor::interface().version);
                    state.compositor = Some(registry.bind(name, version, qhandle, ()));
                } else if interface == XdgWmBase::interface().name {
                    let version = u32::min(version, XdgWmBase::interface().version);
                    state.xdg_wm_base = Some(registry.bind(name, version, qhandle, ()));
                } else if interface == WpSinglePixelBufferManagerV1::interface().name {
                    let version =
                        u32::min(version, WpSinglePixelBufferManagerV1::interface().version);
                    state.single_pixel_buffer = Some(registry.bind(name, version, qhandle, ()));
                } else if interface == WpViewporter::interface().name {
                    let version = u32::min(version, WpViewporter::interface().version);
                    state.viewporter = Some(registry.bind(name, version, qhandle, ()));
                }
            }
            wl_registry::Event::GlobalRemove { name: _ } => (),
            _ => unreachable!(),
        }
    }
}

impl Dispatch<WlCallback, Arc<AtomicBool>> for State {
    fn event(
        _state: &mut Self,
        _proxy: &WlCallback,
        event: <WlCallback as wayland_client::Proxy>::Event,
        data: &Arc<AtomicBool>,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wl_callback::Event::Done { .. } => data.store(true, Ordering::Relaxed),
            _ => panic!("new event"),
        }
    }
}

impl Dispatch<WlRegistry, GlobalListContents> for State {
    fn event(
        _state: &mut Self,
        _proxy: &WlRegistry,
        _event: <WlRegistry as wayland_client::Proxy>::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        // TODO: Hotplugged outputs
    }
}

impl Dispatch<XdgWmBase, ()> for State {
    fn event(
        _state: &mut Self,
        proxy: &XdgWmBase,
        event: <XdgWmBase as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            xdg_wm_base::Event::Ping { serial } => proxy.pong(serial),
            _ => todo!(),
        }
    }
}

impl Dispatch<WlSurface, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &WlSurface,
        _event: <WlSurface as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<XdgSurface, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &XdgSurface,
        event: <XdgSurface as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            xdg_surface::Event::Configure { serial } => {
                let window = state
                    .windows
                    .iter_mut()
                    .find(|win| &win.xdg_surface == proxy)
                    .unwrap();

                window.current_configure_serial = Some(serial);

                debug!(?window.pending_configure, ?serial, "received configure");

                let PendingConfigure {
                    size,
                    states,
                    bounds: _,
                } = std::mem::take(&mut window.pending_configure);

                if let Some((mut w, mut h)) = size {
                    if w == 0 {
                        w = 640;
                    }
                    if h == 0 {
                        h = 480;
                    }
                    window.viewport.set_destination(w, h);
                }

                if let Some(states) = states {
                    window.fullscreen = states.contains(&xdg_toplevel::State::Fullscreen);
                    window.maximized = states.contains(&xdg_toplevel::State::Maximized);
                }
            }
            _ => todo!(),
        }
    }
}

impl Dispatch<XdgToplevel, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &XdgToplevel,
        event: <XdgToplevel as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        let window = state
            .windows
            .iter_mut()
            .find(|win| &win.toplevel == proxy)
            .unwrap();

        match event {
            xdg_toplevel::Event::Configure {
                width,
                height,
                states,
            } => {
                window.pending_configure.size = Some((width, height));

                let states = states
                    .chunks_exact(4)
                    .map(|slice| <[u8; 4]>::try_from(slice).unwrap())
                    .map(u32::from_ne_bytes)
                    .flat_map(xdg_toplevel::State::try_from)
                    .collect();

                window.pending_configure.states = Some(states);
            }
            xdg_toplevel::Event::Close => window.close_requested = true,
            xdg_toplevel::Event::ConfigureBounds { width, height } => {
                window.pending_configure.bounds = Some((width, height));
            }
            xdg_toplevel::Event::WmCapabilities { .. } => (),
            _ => panic!(),
        }
    }
}

delegate_noop!(State: WlCompositor);
delegate_noop!(State: WpSinglePixelBufferManagerV1);
delegate_noop!(State: WpViewporter);
delegate_noop!(State: WpViewport);
delegate_noop!(State: ignore WlBuffer);
