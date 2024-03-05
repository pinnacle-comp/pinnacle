// SPDX-License-Identifier: GPL-3.0-or-later

use smithay::{
    desktop::Window,
    utils::{Logical, Point, Rectangle, SERIAL_COUNTER},
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
        X11Surface, X11Wm, XwmHandler,
    },
};

use crate::{
    focus::keyboard::KeyboardFocusTarget,
    state::{State, WithState},
    window::{window_state::FloatingOrTiled, WindowElement},
};

impl XwmHandler for State {
    fn xwm_state(&mut self, _xwm: XwmId) -> &mut X11Wm {
        self.xwm.as_mut().expect("xwm not in state")
    }

    fn new_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

    fn new_override_redirect_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

    fn map_window_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        tracing::trace!("map_window_request");

        assert!(!surface.is_override_redirect());

        let window = WindowElement::new(Window::new_x11_window(surface));
        self.space.map_element(window.clone(), (0, 0), true);
        let bbox = self
            .space
            .element_bbox(&window)
            .expect("called element_bbox on an unmapped window");

        let output_size = self
            .focused_output()
            .and_then(|op| self.space.output_geometry(op))
            .map(|geo| geo.size)
            .unwrap_or((2, 2).into());

        let output_loc = self
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

        self.space.map_element(window.clone(), loc, true);
        surface.set_mapped(true).expect("failed to map x11 window");

        let bbox = Rectangle::from_loc_and_size(loc, bbox.size);

        tracing::debug!("map_window_request, configuring with bbox {bbox:?}");
        surface
            .configure(bbox)
            .expect("failed to configure x11 window");
        // TODO: ssd

        if let Some(output) = self.focused_output() {
            window.place_on_output(output);
        }

        if should_float(surface) {
            window.with_state_mut(|state| {
                state.floating_or_tiled = FloatingOrTiled::Floating(bbox);
            });
        }

        // TODO: will an unmap -> map duplicate the window
        self.windows.push(window.clone());
        self.z_index_stack.set_focus(window.clone());

        self.apply_window_rules(&window);

        if let Some(output) = window.output(self) {
            output.with_state_mut(|state| state.focus_stack.set_focus(window.clone()));
            self.update_windows(&output);
        }

        self.loop_handle.insert_idle(move |state| {
            state
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
        tracing::trace!("mapped_override_redirect_window");

        assert!(surface.is_override_redirect());

        let loc = surface.geometry().loc;

        let window = WindowElement::new(Window::new_x11_window(surface));

        self.windows.push(window.clone());
        self.z_index_stack.set_focus(window.clone());

        if let Some(output) = self.focused_output() {
            window.place_on_output(output);
            // FIXME: setting focus here may possibly muck things up
            // |      or maybe they won't idk
            output.with_state_mut(|state| state.focus_stack.set_focus(window.clone()))
        }

        self.space.map_element(window, loc, true);
    }

    fn unmapped_window(&mut self, _xwm: XwmId, surface: X11Surface) {
        for output in self.space.outputs() {
            output.with_state_mut(|state| {
                state.focus_stack.stack.retain(|win| {
                    win.wl_surface()
                        .is_some_and(|surf| Some(surf) != surface.wl_surface())
                })
            });
        }

        let win = self
            .space
            .elements()
            .find(|elem| matches!(elem.x11_surface(), Some(surf) if surf == &surface))
            .cloned();

        if let Some(win) = win {
            self.windows
                .retain(|elem| win.wl_surface() != elem.wl_surface());
            self.z_index_stack
                .stack
                .retain(|elem| win.wl_surface() != elem.wl_surface());

            self.space.unmap_elem(&win);

            if let Some(output) = win.output(self) {
                self.update_windows(&output);

                let focus = self
                    .focused_window(&output)
                    .map(KeyboardFocusTarget::Window);

                if let Some(KeyboardFocusTarget::Window(win)) = &focus {
                    self.space.raise_element(win, true);
                    self.z_index_stack.set_focus(win.clone());
                    if let Some(toplevel) = win.toplevel() {
                        toplevel.send_configure();
                    }
                }

                self.seat
                    .get_keyboard()
                    .expect("Seat had no keyboard")
                    .set_focus(self, focus, SERIAL_COUNTER.next_serial());

                self.schedule_render(&output);
            }
        }

        if !surface.is_override_redirect() {
            tracing::debug!("set mapped to false");
            surface.set_mapped(false).expect("failed to unmap x11 win");
        }
    }

    fn destroyed_window(&mut self, _xwm: XwmId, surface: X11Surface) {
        for output in self.space.outputs() {
            output.with_state_mut(|state| {
                state.focus_stack.stack.retain(|win| {
                    win.wl_surface()
                        .is_some_and(|surf| Some(surf) != surface.wl_surface())
                })
            });
        }

        let win = self
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
            tracing::debug!("removing x11 window from windows");

            // INFO: comparing the windows doesn't work so wlsurface it is
            // self.windows.retain(|elem| &win != elem);
            self.windows
                .retain(|elem| win.wl_surface() != elem.wl_surface());

            self.z_index_stack
                .stack
                .retain(|elem| win.wl_surface() != elem.wl_surface());

            if let Some(output) = win.output(self) {
                self.update_windows(&output);

                let focus = self
                    .focused_window(&output)
                    .map(KeyboardFocusTarget::Window);

                if let Some(KeyboardFocusTarget::Window(win)) = &focus {
                    self.space.raise_element(win, true);
                    self.z_index_stack.set_focus(win.clone());
                    if let Some(toplevel) = win.toplevel() {
                        toplevel.send_configure();
                    }
                }

                self.seat
                    .get_keyboard()
                    .expect("Seat had no keyboard")
                    .set_focus(self, focus, SERIAL_COUNTER.next_serial());

                self.schedule_render(&output);
            }
        }
        tracing::debug!("destroyed x11 window");
    }

    fn configure_request(
        &mut self,
        _xwm: XwmId,
        window: X11Surface,
        _x: Option<i32>,
        _y: Option<i32>,
        w: Option<u32>,
        h: Option<u32>,
        _reorder: Option<Reorder>,
    ) {
        let mut geo = window.geometry();
        if let Some(w) = w {
            geo.size.w = w as i32;
        }
        if let Some(h) = h {
            geo.size.h = h as i32;
        }

        if let Err(err) = window.configure(geo) {
            tracing::error!("Failed to configure x11 win: {err}");
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
            .space
            .elements()
            .find(|elem| matches!(elem.x11_surface(), Some(surf) if surf == &surface))
            .cloned()
        else {
            return;
        };

        self.space.map_element(win, geometry.loc, true);
    }

    fn maximize_request(&mut self, _xwm: XwmId, window: X11Surface) {
        window
            .set_maximized(true)
            .expect("failed to set x11 win to maximized");

        let Some(window) = window
            .wl_surface()
            .and_then(|surf| self.window_for_surface(&surf))
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
            .and_then(|surf| self.window_for_surface(&surf))
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
            .and_then(|surf| self.window_for_surface(&surf))
        else {
            return;
        };

        if !window.with_state(|state| state.fullscreen_or_maximized.is_fullscreen()) {
            window.toggle_fullscreen();
        }
    }

    fn unfullscreen_request(&mut self, _xwm: XwmId, window: X11Surface) {
        window
            .set_fullscreen(false)
            .expect("failed to set x11 win to unfullscreen");

        let Some(window) = window
            .wl_surface()
            .and_then(|surf| self.window_for_surface(&surf))
        else {
            return;
        };

        if window.with_state(|state| state.fullscreen_or_maximized.is_fullscreen()) {
            window.toggle_fullscreen();
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
        let seat = self.seat.clone();

        // We use the server one and not the client because windows like Steam don't provide
        // GrabStartData, so we need to create it ourselves.
        crate::grab::resize_grab::resize_request_server(
            self,
            &wl_surf,
            &seat,
            SERIAL_COUNTER.next_serial(),
            resize_edge.into(),
            button,
        );
    }

    fn move_request(&mut self, _xwm: XwmId, window: X11Surface, button: u32) {
        let Some(wl_surf) = window.wl_surface() else { return };
        let seat = self.seat.clone();

        // We use the server one and not the client because windows like Steam don't provide
        // GrabStartData, so we need to create it ourselves.
        crate::grab::move_grab::move_request_server(
            self,
            &wl_surf,
            &seat,
            SERIAL_COUNTER.next_serial(),
            button,
        );
    }

    fn allow_selection_access(&mut self, xwm: XwmId, _selection: SelectionTarget) -> bool {
        self.seat
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
                if let Err(err) = request_data_device_client_selection(&self.seat, mime_type, fd) {
                    tracing::error!(
                        ?err,
                        "Failed to request current wayland clipboard for XWayland"
                    );
                }
            }
            SelectionTarget::Primary => {
                if let Err(err) = request_primary_client_selection(&self.seat, mime_type, fd) {
                    tracing::error!(
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
                set_data_device_selection(&self.display_handle, &self.seat, mime_types, ());
            }
            SelectionTarget::Primary => {
                set_primary_selection(&self.display_handle, &self.seat, mime_types, ());
            }
        }
    }

    fn cleared_selection(&mut self, _xwm: XwmId, selection: SelectionTarget) {
        match selection {
            SelectionTarget::Clipboard => {
                if current_data_device_selection_userdata(&self.seat).is_some() {
                    clear_data_device_selection(&self.display_handle, &self.seat);
                }
            }
            SelectionTarget::Primary => {
                if current_primary_selection_userdata(&self.seat).is_some() {
                    clear_primary_selection(&self.display_handle, &self.seat);
                }
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
