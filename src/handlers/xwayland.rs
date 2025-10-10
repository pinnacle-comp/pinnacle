// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    os::fd::OwnedFd,
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use smithay::{
    desktop::Window,
    input::pointer::CursorIcon,
    reexports::wayland_server::Client,
    utils::{Logical, Point, Rectangle, SERIAL_COUNTER, Size},
    wayland::selection::{
        SelectionTarget,
        data_device::{
            clear_data_device_selection, current_data_device_selection_userdata,
            request_data_device_client_selection, set_data_device_selection,
        },
        primary_selection::{
            clear_primary_selection, current_primary_selection_userdata,
            request_primary_client_selection, set_primary_selection,
        },
    },
    xwayland::{
        X11Surface, X11Wm, XWayland, XWaylandClientData, XWaylandEvent, XwmHandler,
        xwm::{Reorder, WmWindowProperty, XwmId},
    },
};
use tracing::{debug, error, info, trace, warn};

use crate::{
    api::signal::Signal,
    focus::keyboard::KeyboardFocusTarget,
    state::{Pinnacle, State, WithState},
    window::{
        Unmapped, UnmappedState, WindowElement, rules::ClientRequests,
        window_state::FullscreenOrMaximized,
    },
};

#[derive(Debug)]
pub struct XwaylandState {
    pub xwm: X11Wm,
    pub display_num: u32,
    pub client: Client,
    pub should_clients_self_scale: bool,
    pub current_scale: Option<f64>,
}

impl XwmHandler for State {
    fn xwm_state(&mut self, _xwm: XwmId) -> &mut X11Wm {
        &mut self.pinnacle.xwayland_state.as_mut().unwrap().xwm
    }

    fn new_window(&mut self, _xwm: XwmId, _window: X11Surface) {
        trace!(class = _window.class(), "XwmHandler::new_window");
    }

    fn new_override_redirect_window(&mut self, _xwm: XwmId, _window: X11Surface) {
        trace!(
            class = _window.class(),
            "XwmHandler::new_override_redirect_window"
        );
    }

    fn map_window_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        trace!(class = surface.class(), "XwmHandler::map_window_request");

        let exists = self
            .pinnacle
            .unmapped_windows
            .iter()
            .any(|unmapped| unmapped.window.x11_surface() == Some(&surface))
            || self.pinnacle.window_for_x11_surface(&surface).is_some();
        if exists {
            return;
        }

        let mut unmapped = Unmapped {
            window: WindowElement::new(Window::new_x11_window(surface)),
            activation_token_data: None,
            state: UnmappedState::WaitingForTags {
                client_requests: ClientRequests::default(),
            },
        };

        let handle = self
            .pinnacle
            .foreign_toplevel_list_state
            .new_toplevel::<State>(
                unmapped.window.title().unwrap_or_default(),
                unmapped.window.class().unwrap_or_default(),
            );
        unmapped.window.with_state_mut(|state| {
            assert!(state.foreign_toplevel_list_handle.is_none());
            state.foreign_toplevel_list_handle = Some(handle);
        });

        if unmapped.window.output(&self.pinnacle).is_some() {
            self.pinnacle.request_window_rules(&mut unmapped);
        } else if let Some(output) = self.pinnacle.focused_output()
            && output.with_state(|state| !state.tags.is_empty())
        {
            unmapped.window.set_tags_to_output(output);
            self.pinnacle.request_window_rules(&mut unmapped);
        }

        self.pinnacle.unmapped_windows.push(unmapped);
    }

    fn mapped_override_redirect_window(&mut self, _xwm: XwmId, surface: X11Surface) {
        trace!(
            class = surface.class(),
            "XwmHandler::mapped_override_redirect_window"
        );

        let loc = surface.geometry().loc;

        let window = self
            .pinnacle
            .window_for_x11_surface(&surface)
            .cloned()
            .unwrap_or_else(|| {
                let window = WindowElement::new(Window::new_x11_window(surface));
                window.with_state_mut(|state| state.layout_mode.set_floating(true));
                self.pinnacle.windows.push(window.clone());
                window
            });

        if let Some(output) = self.pinnacle.focused_output() {
            window.set_tags_to_output(output);
        }

        self.pinnacle.map_window_to(&window, loc);
        self.pinnacle.raise_window(window.clone());
    }

    fn map_window_notify(&mut self, _xwm: XwmId, window: X11Surface) {
        trace!(class = window.class(), "XwmHandler::map_window_notify");

        let Some(idx) = self
            .pinnacle
            .unmapped_windows
            .iter()
            .position(|unmapped| unmapped.window.x11_surface() == Some(&window))
        else {
            return;
        };

        let unmapped = self.pinnacle.unmapped_windows.remove(idx);

        self.map_new_window(unmapped);
    }

    fn unmapped_window(&mut self, _xwm: XwmId, surface: X11Surface) {
        trace!(class = surface.class(), "XwmHandler::unmapped_window");
        self.remove_xwayland_window(surface);
    }

    fn destroyed_window(&mut self, _xwm: XwmId, surface: X11Surface) {
        trace!(class = surface.class(), "XwmHandler::destroyed_window");
        self.remove_xwayland_window(surface);
    }

    fn configure_request(
        &mut self,
        _xwm: XwmId,
        window: X11Surface,
        x: Option<i32>,
        y: Option<i32>,
        w: Option<u32>,
        h: Option<u32>,
        _reorder: Option<Reorder>,
    ) {
        trace!(
            class = window.class(),
            ?x,
            ?y,
            ?w,
            ?h,
            "XwmHandler::configure_request"
        );

        let should_configure = self
            .pinnacle
            .window_for_x11_surface(&window)
            .map(|win| win.with_state(|state| state.layout_mode.is_floating()))
            .unwrap_or(true);

        if should_configure {
            let mut geo = window.geometry();

            if let Some(x) = x {
                geo.loc.x = x;
            }
            if let Some(y) = y {
                geo.loc.y = y;
            }
            if let Some(w) = w {
                geo.size.w = w as i32;
            }
            if let Some(h) = h {
                geo.size.h = h as i32;
            }

            if let Err(err) = window.configure(geo) {
                warn!(?geo, "Failed to configure x11 win: {err}");
            }
        }
    }

    fn configure_notify(
        &mut self,
        _xwm: XwmId,
        surface: X11Surface,
        geometry: Rectangle<i32, Logical>,
        _above: Option<smithay::reexports::x11rb::protocol::xproto::Window>,
    ) {
        trace!(
            class = surface.class(),
            ?geometry,
            "XwmHandler::configure_notify"
        );

        if let Some(win) = self.pinnacle.window_for_x11_surface(&surface) {
            win.with_state(|state| {
                for deco in state.decoration_surfaces.iter() {
                    deco.decoration_surface().with_pending_state(|state| {
                        state.toplevel_size = Some(geometry.size);
                    });
                    deco.decoration_surface().send_pending_configure();
                }
            })
        }

        let Some(win) = self
            .pinnacle
            .space
            .elements()
            .find(|elem| {
                matches!(elem.x11_surface(), Some(surf) if surf == &surface)
                    && elem.is_x11_override_redirect()
            })
            .cloned()
        else {
            return;
        };

        self.pinnacle.map_window_to(&win, geometry.loc);
    }

    fn maximize_request(&mut self, _xwm: XwmId, window: X11Surface) {
        trace!(class = window.class(), "XwmHandler::maximize_request");

        if let Some(window) = self.pinnacle.window_for_x11_surface(&window).cloned() {
            self.pinnacle
                .update_window_layout_mode(&window, |layout_mode| {
                    layout_mode.set_client_maximized(true);
                });
        } else if let Some(unmapped) = self.pinnacle.unmapped_window_for_x11_surface_mut(&window) {
            match &mut unmapped.state {
                UnmappedState::WaitingForTags { client_requests } => {
                    client_requests.layout_mode = Some(FullscreenOrMaximized::Maximized);
                }
                UnmappedState::WaitingForRules {
                    rules: _,
                    client_requests,
                } => {
                    client_requests.layout_mode = Some(FullscreenOrMaximized::Maximized);
                }
                UnmappedState::PostInitialConfigure {
                    attempt_float_on_map,
                    ..
                } => {
                    let window = unmapped.window.clone();
                    window.with_state_mut(|state| state.layout_mode.set_client_maximized(true));
                    *attempt_float_on_map = false;
                    self.pinnacle.configure_window_if_nontiled(&window);
                }
            }
        }
    }

    fn unmaximize_request(&mut self, _xwm: XwmId, window: X11Surface) {
        trace!(class = window.class(), "XwmHandler::unmaximize_request");

        if let Some(window) = self.pinnacle.window_for_x11_surface(&window).cloned() {
            self.pinnacle
                .update_window_layout_mode(&window, |layout_mode| {
                    layout_mode.set_client_maximized(false);
                });
        } else if let Some(unmapped) = self.pinnacle.unmapped_window_for_x11_surface_mut(&window) {
            match &mut unmapped.state {
                UnmappedState::WaitingForTags { client_requests } => {
                    client_requests
                        .layout_mode
                        .take_if(|mode| matches!(mode, FullscreenOrMaximized::Maximized));
                }
                UnmappedState::WaitingForRules {
                    rules: _,
                    client_requests,
                } => {
                    client_requests
                        .layout_mode
                        .take_if(|mode| matches!(mode, FullscreenOrMaximized::Maximized));
                }
                UnmappedState::PostInitialConfigure { .. } => {
                    let window = unmapped.window.clone();
                    window.with_state_mut(|state| state.layout_mode.set_client_maximized(false));
                    self.pinnacle.configure_window_if_nontiled(&window);
                }
            }
        }
    }

    fn fullscreen_request(&mut self, _xwm: XwmId, window: X11Surface) {
        trace!(class = window.class(), "XwmHandler::fullscreen_request");

        if let Some(window) = self.pinnacle.window_for_x11_surface(&window).cloned() {
            self.pinnacle
                .update_window_layout_mode(&window, |layout_mode| {
                    layout_mode.set_client_fullscreen(true);
                });
        } else if let Some(unmapped) = self.pinnacle.unmapped_window_for_x11_surface_mut(&window) {
            match &mut unmapped.state {
                UnmappedState::WaitingForTags { client_requests } => {
                    client_requests.layout_mode = Some(FullscreenOrMaximized::Fullscreen);
                }
                UnmappedState::WaitingForRules {
                    rules: _,
                    client_requests,
                } => {
                    client_requests.layout_mode = Some(FullscreenOrMaximized::Fullscreen);
                }
                UnmappedState::PostInitialConfigure {
                    attempt_float_on_map,
                    ..
                } => {
                    let window = unmapped.window.clone();
                    window.with_state_mut(|state| state.layout_mode.set_client_fullscreen(true));
                    *attempt_float_on_map = false;
                    self.pinnacle.configure_window_if_nontiled(&window);
                }
            }
        }
    }

    fn unfullscreen_request(&mut self, _xwm: XwmId, window: X11Surface) {
        trace!(class = window.class(), "XwmHandler::unfullscreen_request");

        if let Some(window) = self.pinnacle.window_for_x11_surface(&window).cloned() {
            self.pinnacle
                .update_window_layout_mode(&window, |layout_mode| {
                    layout_mode.set_client_fullscreen(false);
                });
        } else if let Some(unmapped) = self.pinnacle.unmapped_window_for_x11_surface_mut(&window) {
            match &mut unmapped.state {
                UnmappedState::WaitingForTags { client_requests } => {
                    client_requests
                        .layout_mode
                        .take_if(|mode| matches!(mode, FullscreenOrMaximized::Fullscreen));
                }
                UnmappedState::WaitingForRules {
                    rules: _,
                    client_requests,
                } => {
                    client_requests
                        .layout_mode
                        .take_if(|mode| matches!(mode, FullscreenOrMaximized::Fullscreen));
                }
                UnmappedState::PostInitialConfigure { .. } => {
                    let window = unmapped.window.clone();
                    window.with_state_mut(|state| state.layout_mode.set_client_fullscreen(false));
                    self.pinnacle.configure_window_if_nontiled(&window);
                }
            }
        }
    }

    fn resize_request(
        &mut self,
        _xwm: XwmId,
        window: X11Surface,
        button: u32,
        resize_edge: smithay::xwayland::xwm::ResizeEdge,
    ) {
        let _span = tracy_client::span!("XwmHandler::resize_request");
        trace!(class = window.class(), "XwmHandler::resize_request");

        let Some(wl_surf) = window.wl_surface() else { return };
        let seat = self.pinnacle.seat.clone();

        self.resize_request_server(
            &wl_surf,
            &seat,
            SERIAL_COUNTER.next_serial(),
            resize_edge.into(),
            button,
        );
    }

    fn move_request(&mut self, _xwm: XwmId, window: X11Surface, button: u32) {
        let _span = tracy_client::span!("XwmHandler::move_request");
        trace!(class = window.class(), "XwmHandler::move_request");

        let Some(wl_surf) = window.wl_surface() else { return };
        let seat = self.pinnacle.seat.clone();

        self.move_request_server(&wl_surf, &seat, SERIAL_COUNTER.next_serial(), button);
    }

    fn allow_selection_access(&mut self, xwm: XwmId, _selection: SelectionTarget) -> bool {
        self.pinnacle
            .seat
            .get_keyboard()
            .and_then(|kb| kb.current_focus())
            .is_some_and(|focus| {
                if let KeyboardFocusTarget::Window(window) = focus
                    && let Some(surface) = window.x11_surface()
                {
                    return surface.xwm_id().expect("x11surface had no xwm id") == xwm;
                }
                false
            })
    }

    fn send_selection(
        &mut self,
        _xwm: XwmId,
        selection: SelectionTarget,
        mime_type: String,
        fd: OwnedFd,
    ) {
        debug!(?selection, ?mime_type, ?fd, "XwmHandler::send_selection");

        match selection {
            SelectionTarget::Clipboard => {
                if let Err(err) =
                    request_data_device_client_selection(&self.pinnacle.seat, mime_type, fd)
                {
                    error!(
                        ?err,
                        "Failed to request current wayland clipboard for XWayland"
                    );
                }
            }
            SelectionTarget::Primary => {
                if let Err(err) =
                    request_primary_client_selection(&self.pinnacle.seat, mime_type, fd)
                {
                    error!(
                        ?err,
                        "Failed to request current wayland primary selection for XWayland"
                    );
                }
            }
        }
    }

    fn new_selection(&mut self, _xwm: XwmId, selection: SelectionTarget, mime_types: Vec<String>) {
        debug!(?selection, ?mime_types, "XwmHandler::new_selection");

        match selection {
            SelectionTarget::Clipboard => {
                set_data_device_selection(
                    &self.pinnacle.display_handle,
                    &self.pinnacle.seat,
                    mime_types,
                    (),
                );
            }
            SelectionTarget::Primary => {
                set_primary_selection(
                    &self.pinnacle.display_handle,
                    &self.pinnacle.seat,
                    mime_types,
                    (),
                );
            }
        }
    }

    fn cleared_selection(&mut self, _xwm: XwmId, selection: SelectionTarget) {
        debug!(?selection, "XwmHandler::cleared_selection");

        match selection {
            SelectionTarget::Clipboard => {
                if current_data_device_selection_userdata(&self.pinnacle.seat).is_some() {
                    clear_data_device_selection(&self.pinnacle.display_handle, &self.pinnacle.seat);
                }
            }
            SelectionTarget::Primary => {
                if current_primary_selection_userdata(&self.pinnacle.seat).is_some() {
                    clear_primary_selection(&self.pinnacle.display_handle, &self.pinnacle.seat);
                }
            }
        }
    }

    fn property_notify(&mut self, _xwm: XwmId, window: X11Surface, property: WmWindowProperty) {
        let Some(window) = self.pinnacle.window_for_x11_surface(&window).cloned() else {
            return;
        };
        match property {
            WmWindowProperty::Title => {
                self.pinnacle
                    .signal_state
                    .window_title_changed
                    .signal(&window);

                let title = window.title().unwrap_or_default();
                window.with_state(|state| {
                    if let Some(handle) = state.foreign_toplevel_list_handle.as_ref() {
                        handle.send_title(&title);
                        handle.send_done();
                    }
                });
            }
            WmWindowProperty::Class => {
                let app_id = window.class().unwrap_or_default();
                window.with_state(|state| {
                    if let Some(handle) = state.foreign_toplevel_list_handle.as_ref() {
                        handle.send_app_id(&app_id);
                        handle.send_done();
                    }
                });
            }
            _ => (),
        }
    }
}

impl State {
    fn remove_xwayland_window(&mut self, surface: X11Surface) {
        let _span = tracy_client::span!("State::remove_xwayland_window");

        let Some(win) = self.pinnacle.window_for_x11_surface(&surface).cloned() else {
            return;
        };

        let should_layout =
            !win.is_x11_override_redirect() && win.with_state(|state| state.layout_mode.is_tiled());

        let output = win.output(&self.pinnacle);

        if let Some(output) = output.as_ref() {
            self.backend.with_renderer(|renderer| {
                win.capture_snapshot_and_store(
                    renderer,
                    output.current_scale().fractional_scale().into(),
                    1.0,
                );
            });
        }

        let outputs = self.pinnacle.space.outputs_for_element(&win);

        self.pinnacle.remove_window(&win, false);

        if let Some(output) = win.output(&self.pinnacle)
            && should_layout
        {
            self.pinnacle.request_layout(&output);
        }

        for output in outputs {
            self.schedule_render(&output);
        }
    }
}

impl Pinnacle {
    pub fn update_xwayland_stacking_order(&mut self) {
        let _span = tracy_client::span!("Pinnacle::update_xwayland_stacking_order");

        let Some(xwm) = self
            .xwayland_state
            .as_mut()
            .map(|xwayland| &mut xwayland.xwm)
        else {
            return;
        };

        let (active_windows, non_active_windows) = self
            .z_index_stack
            .iter()
            .filter_map(|z| z.window())
            .filter(|win| !win.is_x11_override_redirect())
            .partition::<Vec<_>, _>(|win| win.is_on_active_tag());

        let active_windows = active_windows.into_iter().flat_map(|win| win.x11_surface());
        let non_active_windows = non_active_windows
            .into_iter()
            .flat_map(|win| win.x11_surface());

        if let Err(err) =
            xwm.update_stacking_order_upwards(non_active_windows.chain(active_windows))
        {
            warn!("Failed to update xwayland stacking order: {err}");
        }
    }

    /// Spawns an [`XWayland`] instance and inserts its event source into
    /// the event loop.
    ///
    /// Receives a boolean flag that becomes true once this finishes.
    pub fn insert_xwayland_source(&mut self, flag: Arc<AtomicBool>) -> anyhow::Result<()> {
        let _span = tracy_client::span!("Pinnacle::insert_xwayland_source");

        // TODO: xwayland keyboard grab state

        let (xwayland, client) = XWayland::spawn(
            &self.display_handle,
            None,
            std::iter::empty::<(String, String)>(),
            true,
            Stdio::null(),
            Stdio::null(),
            |_| (),
        )?;

        self.loop_handle
            .insert_source(xwayland, move |event, _, state| {
                match event {
                    XWaylandEvent::Ready {
                        x11_socket,
                        display_number,
                    } => {
                        let mut wm = match X11Wm::start_wm(
                            state.pinnacle.loop_handle.clone(),
                            x11_socket,
                            client.clone(),
                        ) {
                            Ok(wm) => wm,
                            Err(err) => {
                                error!("Failed to start xwayland wm: {err}");
                                return;
                            }
                        };

                        let cursor = state
                            .pinnacle
                            .cursor_state
                            .get_xcursor_images(CursorIcon::Default)
                            .unwrap();
                        let image = cursor
                            .image(Duration::ZERO, state.pinnacle.cursor_state.cursor_size(1));
                        if let Err(err) = wm.set_cursor(
                            &image.pixels_rgba,
                            Size::from((image.width as u16, image.height as u16)),
                            Point::from((image.xhot as u16, image.yhot as u16)),
                        ) {
                            warn!("Failed to set xwayland default cursor: {err}");
                        }

                        // SAFETY: All set_vars occur on the event loop thread
                        unsafe {
                            std::env::set_var("DISPLAY", format!(":{display_number}"));
                        }

                        state.pinnacle.xwayland_state = Some(XwaylandState {
                            xwm: wm,
                            display_num: display_number,
                            client: client.clone(),
                            should_clients_self_scale: false,
                            current_scale: None,
                        });

                        state.pinnacle.update_xwayland_scale();

                        info!("Xwayland started at :{display_number}");
                    }
                    XWaylandEvent::Error => {
                        state.pinnacle.xwayland_state.take();
                        warn!("XWayland crashed on startup");
                    }
                }

                flag.store(true, Ordering::Relaxed);
            })?;

        Ok(())
    }

    // Yoinked from le cosmic:
    // https://github.com/pop-os/cosmic-comp/pull/779
    pub fn update_xwayland_scale(&mut self) {
        let Some(xwayland_state) = self.xwayland_state.as_mut() else {
            return;
        };

        let new_scale = if xwayland_state.should_clients_self_scale {
            self.outputs
                .iter()
                .map(|op| op.current_scale().fractional_scale())
                .max_by(|a, b| a.total_cmp(b))
                .unwrap_or(1.0)
        } else {
            1.0
        };

        if xwayland_state.current_scale == Some(new_scale) {
            return;
        }

        let dpi = new_scale.abs() * 96.0 * 1024.0;
        if let Err(err) = xwayland_state.xwm.set_xsettings(
            [
                ("Xft/DPI".into(), (dpi.round() as i32).into()),
                (
                    "Gdk/UnscaledDPI".into(),
                    ((dpi / new_scale).round() as i32).into(),
                ),
                (
                    "Gdk/WindowScalingFactor".into(),
                    (new_scale.round() as i32).into(),
                ),
            ]
            .into_iter(),
        ) {
            warn!("Failed to update XSETTINGS on scale change: {err}");
        }

        xwayland_state
            .client
            .get_data::<XWaylandClientData>()
            .unwrap()
            .compositor_state
            .set_client_scale(new_scale);

        for output in self.outputs.iter() {
            output.change_current_state(None, None, None, None);
        }

        let geos = self
            .windows
            .iter()
            .filter_map(|win| {
                win.x11_surface()
                    .map(|surf| (surf.clone(), surf.geometry()))
            })
            .collect::<Vec<_>>();

        for (surface, geo) in geos {
            if let Err(err) = surface.configure(geo) {
                warn!("Failed to update surface geo after scale change: {err}");
            }
        }

        xwayland_state.current_scale = Some(new_scale);

        self.update_xwayland_stacking_order();
    }

    fn window_for_x11_surface(&self, surface: &X11Surface) -> Option<&WindowElement> {
        self.windows
            .iter()
            .find(|win| win.x11_surface() == Some(surface))
    }

    fn unmapped_window_for_x11_surface_mut(
        &mut self,
        surface: &X11Surface,
    ) -> Option<&mut Unmapped> {
        self.unmapped_windows
            .iter_mut()
            .find(|unmapped| unmapped.window.x11_surface() == Some(surface))
    }
}
