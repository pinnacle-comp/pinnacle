// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    process::Stdio,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use smithay::{
    desktop::Window,
    input::pointer::CursorIcon,
    reexports::wayland_server::Client,
    utils::{Logical, Point, Rectangle, Size, SERIAL_COUNTER},
    wayland::selection::{
        data_device::{
            clear_data_device_selection, current_data_device_selection_userdata,
            request_data_device_client_selection, set_data_device_selection,
        },
        primary_selection::{
            clear_primary_selection, current_primary_selection_userdata,
            request_primary_client_selection, set_primary_selection,
        },
        SelectionTarget,
    },
    xwayland::{
        xwm::{Reorder, XwmId},
        X11Surface, X11Wm, XWayland, XWaylandClientData, XWaylandEvent, XwmHandler,
    },
};
use tracing::{debug, error, info, warn};

use crate::{
    focus::keyboard::KeyboardFocusTarget,
    state::{Pinnacle, State, WithState},
    window::WindowElement,
};

#[derive(Debug)]
pub struct XwaylandState {
    pub xwm: Option<X11Wm>,
    pub display_num: u32,
    pub client: Client,
    pub should_clients_self_scale: bool,
    pub current_scale: Option<i32>,
}

impl XwmHandler for State {
    fn xwm_state(&mut self, _xwm: XwmId) -> &mut X11Wm {
        self.pinnacle
            .xwayland_state
            .as_mut()
            .unwrap()
            .xwm
            .as_mut()
            .expect("xwm not in state")
    }

    fn new_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

    fn new_override_redirect_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

    fn map_window_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        let _span = tracy_client::span!("XwmHandler::map_window_request");

        if surface.is_override_redirect() {
            // Steam games that reach this: Ori and the Will of the Wisps, Pizza Tower
            return;
        }

        let window = WindowElement::new(Window::new_x11_window(surface));

        if let Some(output) = self.pinnacle.focused_output() {
            self.pinnacle.place_window_on_output(&window, output);
        }

        self.pinnacle.windows.push(window.clone());

        let window_rule_request_sent = self.pinnacle.window_rule_state.new_request(window.clone());
        if !window_rule_request_sent {
            window.x11_surface().unwrap().set_mapped(true).unwrap();
        }
    }

    fn mapped_override_redirect_window(&mut self, _xwm: XwmId, surface: X11Surface) {
        let _span = tracy_client::span!("XwmHandler::mapped_override_redirect_window");

        assert!(surface.is_override_redirect());

        let loc = surface.geometry().loc;

        let window = WindowElement::new(Window::new_x11_window(surface));

        self.pinnacle.windows.push(window.clone());

        if let Some(output) = self.pinnacle.focused_output() {
            self.pinnacle.place_window_on_output(&window, output);
        }

        self.pinnacle.space.map_element(window.clone(), loc, true);
        self.pinnacle.raise_window(window.clone(), true);

        for output in self.pinnacle.space.outputs_for_element(&window) {
            self.schedule_render(&output);
        }
    }

    fn map_window_notify(&mut self, _xwm: XwmId, window: X11Surface) {
        let _span = tracy_client::span!("XwmHandler::map_window_notify");

        let Some(window) = self
            .pinnacle
            .windows
            .iter()
            .find(|win| win.x11_surface() == Some(&window))
            .cloned()
        else {
            return;
        };

        self.map_new_window(&window);
    }

    fn unmapped_window(&mut self, _xwm: XwmId, surface: X11Surface) {
        let _span = tracy_client::span!("XwmHandler::unmapped_window");

        if !surface.is_override_redirect() {
            debug!("set mapped to false");
            surface.set_mapped(false).expect("failed to unmap x11 win");
        }

        self.remove_xwayland_window(surface);
    }

    fn destroyed_window(&mut self, _xwm: XwmId, surface: X11Surface) {
        let _span = tracy_client::span!("XwmHandler::destroyed_window");

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
        let _span = tracy_client::span!("XwmHandler::configure_request");

        let should_configure = self
            .pinnacle
            .windows
            .iter()
            .find(|win| win.x11_surface() == Some(&window))
            .map(|win| {
                win.is_x11_override_redirect()
                    || win.with_state(|state| state.window_state.is_floating())
            })
            .unwrap_or(true);
        // If we unwrap_or here then the window hasn't requested a map yet.
        // In that case, grant the configure. Xterm wants this to map properly, for example.

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
                error!("Failed to configure x11 win: {err}");
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
        let _span = tracy_client::span!("XwmHandler::configure_notify");

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

        self.pinnacle.space.map_element(win, geometry.loc, true);
    }

    fn maximize_request(&mut self, _xwm: XwmId, window: X11Surface) {
        let _span = tracy_client::span!("XwmHandler::maximize_request");

        let Some(window) = window
            .wl_surface()
            .and_then(|surf| self.pinnacle.window_for_surface(&surf))
        else {
            return;
        };

        window.with_state_mut(|state| state.window_state.set_maximized(true));
        self.update_window_state_and_layout(&window);
    }

    fn unmaximize_request(&mut self, _xwm: XwmId, window: X11Surface) {
        let _span = tracy_client::span!("XwmHandler::unmaximize_request");

        let Some(window) = window
            .wl_surface()
            .and_then(|surf| self.pinnacle.window_for_surface(&surf))
        else {
            return;
        };

        window.with_state_mut(|state| state.window_state.set_maximized(false));
        self.update_window_state_and_layout(&window);
    }

    fn fullscreen_request(&mut self, _xwm: XwmId, window: X11Surface) {
        let _span = tracy_client::span!("XwmHandler::fullscreen_request");

        let Some(window) = window
            .wl_surface()
            .and_then(|surf| self.pinnacle.window_for_surface(&surf))
        else {
            return;
        };

        window.with_state_mut(|state| state.window_state.set_fullscreen(true));
        self.update_window_state_and_layout(&window);
    }

    fn unfullscreen_request(&mut self, _xwm: XwmId, window: X11Surface) {
        let _span = tracy_client::span!("XwmHandler::unfullscreen_request");

        let Some(window) = window
            .wl_surface()
            .and_then(|surf| self.pinnacle.window_for_surface(&surf))
        else {
            return;
        };

        window.with_state_mut(|state| state.window_state.set_fullscreen(false));
        self.update_window_state_and_layout(&window);
    }

    fn resize_request(
        &mut self,
        _xwm: XwmId,
        window: X11Surface,
        button: u32,
        resize_edge: smithay::xwayland::xwm::ResizeEdge,
    ) {
        let _span = tracy_client::span!("XwmHandler::resize_request");

        let Some(wl_surf) = window.wl_surface() else { return };
        let seat = self.pinnacle.seat.clone();

        // We use the server one and not the client because windows like Steam don't provide
        // GrabStartData, so we need to create it ourselves.
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

        let Some(wl_surf) = window.wl_surface() else { return };
        let seat = self.pinnacle.seat.clone();

        // We use the server one and not the client because windows like Steam don't provide
        // GrabStartData, so we need to create it ourselves.
        self.move_request_server(&wl_surf, &seat, SERIAL_COUNTER.next_serial(), button);
    }

    fn allow_selection_access(&mut self, xwm: XwmId, _selection: SelectionTarget) -> bool {
        self.pinnacle
            .seat
            .get_keyboard()
            .and_then(|kb| kb.current_focus())
            .is_some_and(|focus| {
                if let KeyboardFocusTarget::Window(window) = focus {
                    if let Some(surface) = window.x11_surface() {
                        return surface.xwm_id().expect("x11surface had no xwm id") == xwm;
                    }
                }
                false
            })
    }

    fn send_selection(
        &mut self,
        _xwm: XwmId,
        selection: SelectionTarget,
        mime_type: String,
        fd: std::os::fd::OwnedFd,
    ) {
        let _span = tracy_client::span!("XwmHandler::send_selection");

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
        let _span = tracy_client::span!("XwmHandler::new_selection");

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
        let _span = tracy_client::span!("XwmHandler::cleared_selection");

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
}

impl State {
    fn remove_xwayland_window(&mut self, surface: X11Surface) {
        let _span = tracy_client::span!("State::remove_xwayland_window");

        let win = self
            .pinnacle
            .windows
            .iter()
            .find(|elem| elem.x11_surface() == Some(&surface))
            .cloned();
        if let Some(win) = win {
            let output = win.output(&self.pinnacle);

            if let Some(output) = output.as_ref() {
                self.capture_snapshots_on_output(output, []);
            }

            self.pinnacle.remove_window(&win, false);

            if let Some(output) = win.output(&self.pinnacle) {
                self.pinnacle.begin_layout_transaction(&output);
                self.pinnacle.request_layout(&output);

                self.update_keyboard_focus(&output);
                // FIXME: schedule renders on all the outputs this window intersected
                self.schedule_render(&output);
            }
        }
    }
}

impl Pinnacle {
    pub fn update_xwayland_stacking_order(&mut self) {
        let _span = tracy_client::span!("Pinnacle::update_xwayland_stacking_order");

        let Some(xwm) = self
            .xwayland_state
            .as_mut()
            .and_then(|xwayland| xwayland.xwm.as_mut())
        else {
            return;
        };

        let (active_windows, non_active_windows) = self
            .z_index_stack
            .iter()
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
                        state.pinnacle.xwayland_state = Some(XwaylandState {
                            xwm: None,
                            display_num: display_number,
                            client: client.clone(),
                            should_clients_self_scale: false,
                            current_scale: None,
                        });

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

                        state.pinnacle.xwayland_state.as_mut().unwrap().xwm = Some(wm);

                        // FIXME: don't set here, spawn client with the var directly
                        std::env::set_var("DISPLAY", format!(":{display_number}"));

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
                .keys()
                .map(|op| op.current_scale().integer_scale())
                .max()
                .unwrap_or(1)
        } else {
            1
        };

        if xwayland_state.current_scale == Some(new_scale) {
            return;
        }

        let geos = self
            .windows
            .iter()
            .filter_map(|win| {
                win.x11_surface()
                    .map(|surf| (surf.clone(), surf.geometry()))
            })
            .collect::<Vec<_>>();

        if let Some(xwm) = xwayland_state.xwm.as_mut() {
            let dpi = new_scale.abs() * 96 * 1024;
            if let Err(err) = xwm.set_xsettings(
                [
                    ("Xft/DPI".into(), dpi.into()),
                    ("Gdk/UnscaledDPI".into(), (dpi / new_scale).into()),
                    ("Gdk/WindowScalingFactor".into(), new_scale.into()),
                ]
                .into_iter(),
            ) {
                warn!("Failed to update XSETTINGS on scale change: {err}");
            }
        }

        xwayland_state
            .client
            .get_data::<XWaylandClientData>()
            .unwrap()
            .compositor_state
            .set_client_scale(new_scale as u32);

        for output in self.outputs.keys() {
            output.change_current_state(None, None, None, None);
        }

        for (surface, geo) in geos {
            if let Err(err) = surface.configure(geo) {
                warn!("Failed to update surface geo after scale change: {err}");
            }
        }

        xwayland_state.current_scale = Some(new_scale);

        self.update_xwayland_stacking_order();
    }
}
