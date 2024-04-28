// SPDX-License-Identifier: GPL-3.0-or-later

use std::{process::Stdio, time::Duration};

use anyhow::anyhow;
use smithay::{
    desktop::Window,
    utils::{Logical, Point, Rectangle, Size, SERIAL_COUNTER},
    wayland::{
        seat::WaylandFocus,
        selection::{
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
    },
    xwayland::{
        xwm::{Reorder, WmWindowType, XwmId},
        X11Surface, X11Wm, XWayland, XWaylandEvent, XwmHandler,
    },
};
use tracing::{debug, error, trace, warn};

use crate::{
    cursor::Cursor,
    focus::keyboard::KeyboardFocusTarget,
    state::{Pinnacle, State, WithState},
    window::{window_state::FloatingOrTiled, WindowElement},
};

impl XwmHandler for State {
    fn xwm_state(&mut self, _xwm: XwmId) -> &mut X11Wm {
        self.pinnacle.xwm.as_mut().expect("xwm not in state")
    }

    fn new_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

    fn new_override_redirect_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

    fn map_window_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        trace!("XwmHandler::map_window_request");

        if surface.is_override_redirect() {
            // Steam games that reach this: Ori and the Will of the Wisps, Pizza Tower
            return;
        }

        let window = WindowElement::new(Window::new_x11_window(surface));
        self.pinnacle
            .space
            .map_element(window.clone(), (0, 0), true);
        let bbox = self
            .pinnacle
            .space
            .element_bbox(&window)
            .expect("called element_bbox on an unmapped window");

        let output_size = self
            .pinnacle
            .focused_output()
            .and_then(|op| self.pinnacle.space.output_geometry(op))
            .map(|geo| geo.size)
            .unwrap_or((2, 2).into());

        let output_loc = self
            .pinnacle
            .focused_output()
            .map(|op| op.current_location())
            .unwrap_or((0, 0).into());

        // Center the popup in the middle of the output.
        // Once I find a way to get an X11Surface's parent it will be centered on the parent if
        // applicable.
        let loc: Point<i32, Logical> = (
            output_loc.x + output_size.w / 2 - bbox.size.w / 2,
            output_loc.y + output_size.h / 2 - bbox.size.h / 2,
        )
            .into();

        let Some(surface) = window.x11_surface() else {
            unreachable!()
        };

        self.pinnacle.space.map_element(window.clone(), loc, true);
        surface.set_mapped(true).expect("failed to map x11 window");

        let bbox = Rectangle::from_loc_and_size(loc, bbox.size);

        debug!("map_window_request, configuring with bbox {bbox:?}");
        surface
            .configure(bbox)
            .expect("failed to configure x11 window");
        // TODO: ssd

        if let Some(output) = self.pinnacle.focused_output() {
            window.place_on_output(output);
        }

        if should_float(surface) {
            window.with_state_mut(|state| {
                state.floating_or_tiled = FloatingOrTiled::Floating(bbox);
            });
        }

        // TODO: will an unmap -> map duplicate the window
        self.pinnacle.windows.push(window.clone());
        self.pinnacle.raise_window(window.clone(), true);

        self.pinnacle.apply_window_rules(&window);

        if let Some(output) = window.output(&self.pinnacle) {
            output.with_state_mut(|state| state.focus_stack.set_focus(window.clone()));
            self.pinnacle.request_layout(&output);
        }

        self.pinnacle.loop_handle.insert_idle(move |state| {
            state
                .pinnacle
                .seat
                .get_keyboard()
                .expect("Seat had no keyboard") // FIXME: actually handle error
                .set_focus(
                    state,
                    Some(KeyboardFocusTarget::Window(window)),
                    SERIAL_COUNTER.next_serial(),
                );
        });
    }

    fn mapped_override_redirect_window(&mut self, _xwm: XwmId, surface: X11Surface) {
        trace!("XwmHandler::mapped_override_redirect_window");

        assert!(surface.is_override_redirect());

        let loc = surface.geometry().loc;

        let window = WindowElement::new(Window::new_x11_window(surface));

        self.pinnacle.windows.push(window.clone());

        if let Some(output) = self.pinnacle.focused_output() {
            window.place_on_output(output);
            // FIXME: setting focus here may possibly muck things up
            // |      or maybe they won't idk
            output.with_state_mut(|state| state.focus_stack.set_focus(window.clone()));
        }

        self.pinnacle.space.map_element(window.clone(), loc, true);
        self.pinnacle.raise_window(window.clone(), true);
    }

    fn map_window_notify(&mut self, _xwm: XwmId, window: X11Surface) {
        trace!("XwmHandler::map_window_notify");
        let Some(output) = window
            .wl_surface()
            .and_then(|s| self.pinnacle.window_for_surface(&s))
            .and_then(|win| win.output(&self.pinnacle))
        else {
            return;
        };
        self.schedule_render(&output);
    }

    fn unmapped_window(&mut self, _xwm: XwmId, surface: X11Surface) {
        trace!("XwmHandler::unmapped_window");
        for output in self.pinnacle.space.outputs() {
            output.with_state_mut(|state| {
                state.focus_stack.stack.retain(|win| {
                    win.wl_surface()
                        .is_some_and(|surf| Some(surf) != surface.wl_surface())
                })
            });
        }

        let win = self
            .pinnacle
            .space
            .elements()
            .find(|elem| matches!(elem.x11_surface(), Some(surf) if surf == &surface))
            .cloned();

        if let Some(win) = win {
            self.pinnacle
                .windows
                .retain(|elem| win.wl_surface() != elem.wl_surface());
            self.pinnacle
                .z_index_stack
                .retain(|elem| win.wl_surface() != elem.wl_surface());

            self.pinnacle.space.unmap_elem(&win);

            if let Some(output) = win.output(&self.pinnacle) {
                self.pinnacle.request_layout(&output);

                let focus = self
                    .pinnacle
                    .focused_window(&output)
                    .map(KeyboardFocusTarget::Window);

                if let Some(KeyboardFocusTarget::Window(win)) = &focus {
                    self.pinnacle.raise_window(win.clone(), true);
                    if let Some(toplevel) = win.toplevel() {
                        toplevel.send_configure();
                    }
                }

                self.pinnacle
                    .seat
                    .get_keyboard()
                    .expect("Seat had no keyboard")
                    .set_focus(self, focus, SERIAL_COUNTER.next_serial());

                self.schedule_render(&output);
            }
        }

        if !surface.is_override_redirect() {
            debug!("set mapped to false");
            surface.set_mapped(false).expect("failed to unmap x11 win");
        }
    }

    fn destroyed_window(&mut self, _xwm: XwmId, surface: X11Surface) {
        trace!("XwmHandler::destroyed_window");
        for output in self.pinnacle.space.outputs() {
            output.with_state_mut(|state| {
                state.focus_stack.stack.retain(|win| {
                    win.wl_surface()
                        .is_some_and(|surf| Some(surf) != surface.wl_surface())
                })
            });
        }

        let win = self
            .pinnacle
            .windows
            .iter()
            .find(|elem| {
                matches!(
                    elem.x11_surface(),
                    Some(surf) if surf.wl_surface() == surface.wl_surface()
                )
            })
            .cloned();

        if let Some(win) = win {
            debug!("removing x11 window from windows");

            // INFO: comparing the windows doesn't work so wlsurface it is
            // self.windows.retain(|elem| &win != elem);
            self.pinnacle
                .windows
                .retain(|elem| win.wl_surface() != elem.wl_surface());

            self.pinnacle
                .z_index_stack
                .retain(|elem| win.wl_surface() != elem.wl_surface());

            if let Some(output) = win.output(&self.pinnacle) {
                self.pinnacle.request_layout(&output);

                let focus = self
                    .pinnacle
                    .focused_window(&output)
                    .map(KeyboardFocusTarget::Window);

                if let Some(KeyboardFocusTarget::Window(win)) = &focus {
                    self.pinnacle.raise_window(win.clone(), true);
                    if let Some(toplevel) = win.toplevel() {
                        toplevel.send_configure();
                    }
                }

                self.pinnacle
                    .seat
                    .get_keyboard()
                    .expect("Seat had no keyboard")
                    .set_focus(self, focus, SERIAL_COUNTER.next_serial());

                self.schedule_render(&output);
            }
        }
        debug!("destroyed x11 window");
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
        trace!("XwmHandler::configure_request");
        let floating_or_override_redirect = self
            .pinnacle
            .windows
            .iter()
            .find(|win| win.x11_surface() == Some(&window))
            .map(|win| {
                win.is_x11_override_redirect()
                    || win.with_state(|state| state.floating_or_tiled.is_floating())
            })
            .unwrap_or(false);

        if floating_or_override_redirect {
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
        window
            .set_maximized(true)
            .expect("failed to set x11 win to maximized");

        let Some(window) = window
            .wl_surface()
            .and_then(|surf| self.pinnacle.window_for_surface(&surf))
        else {
            return;
        };

        if !window.with_state(|state| state.fullscreen_or_maximized.is_maximized()) {
            window.toggle_maximized();
        }
    }

    fn unmaximize_request(&mut self, _xwm: XwmId, window: X11Surface) {
        window
            .set_maximized(false)
            .expect("failed to set x11 win to maximized");

        let Some(window) = window
            .wl_surface()
            .and_then(|surf| self.pinnacle.window_for_surface(&surf))
        else {
            return;
        };

        if window.with_state(|state| state.fullscreen_or_maximized.is_maximized()) {
            window.toggle_maximized();
        }
    }

    fn fullscreen_request(&mut self, _xwm: XwmId, window: X11Surface) {
        window
            .set_fullscreen(true)
            .expect("failed to set x11 win to fullscreen");

        let Some(window) = window
            .wl_surface()
            .and_then(|surf| self.pinnacle.window_for_surface(&surf))
        else {
            return;
        };

        if !window.with_state(|state| state.fullscreen_or_maximized.is_fullscreen()) {
            window.toggle_fullscreen();
            if let Some(output) = window.output(&self.pinnacle) {
                self.pinnacle.request_layout(&output);
            }
        }
    }

    fn unfullscreen_request(&mut self, _xwm: XwmId, window: X11Surface) {
        window
            .set_fullscreen(false)
            .expect("failed to set x11 win to unfullscreen");

        let Some(window) = window
            .wl_surface()
            .and_then(|surf| self.pinnacle.window_for_surface(&surf))
        else {
            return;
        };

        if window.with_state(|state| state.fullscreen_or_maximized.is_fullscreen()) {
            window.toggle_fullscreen();
            if let Some(output) = window.output(&self.pinnacle) {
                self.pinnacle.request_layout(&output);
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

impl Pinnacle {
    pub fn fixup_xwayland_window_layering(&mut self) {
        let Some(xwm) = self.xwm.as_mut() else {
            return;
        };

        let x11_wins = self
            .space
            .elements()
            .filter(|win| win.is_on_active_tag())
            .filter_map(|win| win.x11_surface())
            .cloned()
            .collect::<Vec<_>>();

        for x11_win in x11_wins {
            if let Err(err) = xwm.raise_window(&x11_win) {
                warn!("Failed to raise xwayland window: {err}");
            }
        }
    }
}

/// Make assumptions on whether or not the surface should be floating.
///
/// This logic is taken from the Sway function `wants_floating` in sway/desktop/xwayland.c.
fn should_float(surface: &X11Surface) -> bool {
    let is_popup_by_type = surface.window_type().is_some_and(|typ| {
        matches!(
            typ,
            WmWindowType::Dialog
                | WmWindowType::Utility
                | WmWindowType::Toolbar
                | WmWindowType::Splash
        )
    });
    let is_popup_by_size = surface.size_hints().map_or(false, |size_hints| {
        let Some((min_w, min_h)) = size_hints.min_size else {
            return false;
        };
        let Some((max_w, max_h)) = size_hints.max_size else {
            return false;
        };
        min_w > 0 && min_h > 0 && (min_w == max_w || min_h == max_h)
    });
    surface.is_popup() || is_popup_by_type || is_popup_by_size
}

impl Pinnacle {
    pub fn start_xwayland(&mut self) -> anyhow::Result<()> {
        // TODO: xwayland keybaord grab state

        let (xwayland, client) = XWayland::spawn(
            &self.display_handle,
            None,
            std::iter::empty::<(String, String)>(),
            true,
            Stdio::null(),
            Stdio::null(),
            |_| (),
        )?;

        let display_handle = self.display_handle.clone();

        self.loop_handle
            .insert_source(xwayland, move |event, _, state| match event {
                XWaylandEvent::Ready {
                    x11_socket,
                    display_number,
                } => {
                    let mut wm = X11Wm::start_wm(
                        state.pinnacle.loop_handle.clone(),
                        display_handle.clone(),
                        x11_socket,
                        client.clone(),
                    )
                    .expect("failed to attach x11wm");

                    let cursor = Cursor::load();
                    let image = cursor.get_image(1, Duration::ZERO);
                    wm.set_cursor(
                        &image.pixels_rgba,
                        Size::from((image.width as u16, image.height as u16)),
                        Point::from((image.xhot as u16, image.yhot as u16)),
                    )
                    .expect("failed to set xwayland default cursor");

                    tracing::debug!("setting xwm and xdisplay");

                    state.pinnacle.xwm = Some(wm);
                    state.pinnacle.xdisplay = Some(display_number);

                    std::env::set_var("DISPLAY", format!(":{display_number}"));

                    if let Err(err) = state.pinnacle.start_config(Some(
                        state.pinnacle.config.dir(&state.pinnacle.xdg_base_dirs),
                    )) {
                        panic!("failed to start config: {err}");
                    }
                }
                XWaylandEvent::Error => {
                    warn!("XWayland crashed on startup");
                }
            })
            .map(|_| ())
            .map_err(|err| {
                anyhow!("Failed to insert the XWaylandSource into the event loop: {err}")
            })
    }
}
