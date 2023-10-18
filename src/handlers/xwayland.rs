// SPDX-License-Identifier: GPL-3.0-or-later

use smithay::{
    utils::{Logical, Point, Rectangle, SERIAL_COUNTER},
    wayland::{
        selection::data_device::{
            clear_data_device_selection, current_data_device_selection_userdata,
            request_data_device_client_selection, set_data_device_selection,
        },
        selection::{
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
    focus::FocusTarget,
    state::{CalloopData, WithState},
    window::{window_state::FloatingOrTiled, WindowElement},
};

impl XwmHandler for CalloopData {
    fn xwm_state(&mut self, _xwm: XwmId) -> &mut X11Wm {
        self.state.xwm.as_mut().expect("xwm not in state")
    }

    fn new_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

    fn new_override_redirect_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

    fn map_window_request(&mut self, _xwm: XwmId, window: X11Surface) {
        tracing::trace!("map_window_request");
        let win_type = window.window_type();
        tracing::debug!("window type is {win_type:?}");

        // INFO: This check is here because it happened while launching Ori and the Will of the Wisps
        if window.is_override_redirect() {
            tracing::warn!("Dealt with override redirect window in map_window_request");
            let loc = window.geometry().loc;
            let window = WindowElement::X11(window);
            self.state.space.map_element(window, loc, true);
            return;
        }

        let window = WindowElement::X11(window);
        self.state.space.map_element(window.clone(), (0, 0), true);
        let bbox = self
            .state
            .space
            .element_bbox(&window)
            .expect("called element_bbox on an unmapped window");

        let output_size = self
            .state
            .focus_state
            .focused_output
            .as_ref()
            .and_then(|op| self.state.space.output_geometry(op))
            .map(|geo| geo.size)
            .unwrap_or((2, 2).into());

        let output_loc = self
            .state
            .focus_state
            .focused_output
            .as_ref()
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

        let WindowElement::X11(surface) = &window else {
            unreachable!()
        };
        surface.set_mapped(true).expect("failed to map x11 window");

        self.state.space.map_element(window.clone(), loc, true);
        let bbox = Rectangle::from_loc_and_size(loc, bbox.size);

        tracing::debug!("map_window_request, configuring with bbox {bbox:?}");
        surface
            .configure(bbox)
            .expect("failed to configure x11 window");
        // TODO: ssd

        window.with_state(|state| {
            state.tags = match (
                &self.state.focus_state.focused_output,
                self.state.space.outputs().next(),
            ) {
                (Some(output), _) | (None, Some(output)) => output.with_state(|state| {
                    let output_tags = state.focused_tags().cloned().collect::<Vec<_>>();
                    if !output_tags.is_empty() {
                        output_tags
                    } else if let Some(first_tag) = state.tags.first() {
                        vec![first_tag.clone()]
                    } else {
                        vec![]
                    }
                }),
                (None, None) => vec![],
            };

            tracing::debug!("new window, tags are {:?}", state.tags);
        });

        let WindowElement::X11(surface) = &window else {
            unreachable!()
        };

        if should_float(surface) {
            window.with_state(|state| {
                state.floating_or_tiled = FloatingOrTiled::Floating(bbox);
            });
        }

        self.state.windows.push(window.clone());

        self.state.focus_state.set_focus(window.clone());

        self.state.apply_window_rules(&window);

        if let Some(output) = window.output(&self.state) {
            self.state.update_windows(&output);
        }

        if let WindowElement::X11(s) = &window {
            tracing::debug!("new x11 win geo is {:?}", s.geometry());
        }

        self.state.loop_handle.insert_idle(move |data| {
            data.state
                    .seat
                    .get_keyboard()
                    .expect("Seat had no keyboard") // FIXME: actually handle error
                    .set_focus(
                        &mut data.state,
                        Some(FocusTarget::Window(window)),
                        SERIAL_COUNTER.next_serial(),
                    );
        });
    }

    fn mapped_override_redirect_window(&mut self, _xwm: XwmId, window: X11Surface) {
        tracing::debug!("mapped override redirect window");
        let win_type = window.window_type();
        tracing::debug!("window type is {win_type:?}");
        let loc = window.geometry().loc;
        tracing::debug!("or win geo is {:?}", window.geometry());

        self.state.override_redirect_windows.push(window.clone());

        let window = WindowElement::X11(window);
        window.with_state(|state| {
            state.tags = match (
                &self.state.focus_state.focused_output,
                self.state.space.outputs().next(),
            ) {
                (Some(output), _) | (None, Some(output)) => output.with_state(|state| {
                    let output_tags = state.focused_tags().cloned().collect::<Vec<_>>();
                    if !output_tags.is_empty() {
                        output_tags
                    } else if let Some(first_tag) = state.tags.first() {
                        vec![first_tag.clone()]
                    } else {
                        vec![]
                    }
                }),
                (None, None) => vec![],
            };
        });

        // tracing::debug!("mapped_override_redirect_window to loc {loc:?}");
        self.state.space.map_element(window.clone(), loc, true);
    }

    fn unmapped_window(&mut self, _xwm: XwmId, window: X11Surface) {
        tracing::debug!("UNMAPPED WINDOW");
        self.state.focus_state.focus_stack.retain(|win| {
            win.wl_surface()
                .is_some_and(|surf| Some(surf) != window.wl_surface())
        });
        let win = self
            .state
            .space
            .elements()
            .find(|elem| matches!(elem, WindowElement::X11(surface) if surface == &window))
            .cloned();
        if let Some(win) = win {
            self.state.space.unmap_elem(&win);
            if let Some(output) = win.output(&self.state) {
                self.state.update_windows(&output);
                let focus = self.state.focused_window(&output).map(FocusTarget::Window);
                if let Some(FocusTarget::Window(win)) = &focus {
                    tracing::debug!("Focusing on prev win");
                    self.state.space.raise_element(win, true);
                    if let WindowElement::Wayland(win) = &win {
                        win.toplevel().send_configure();
                    }
                }
                self.state
                    .seat
                    .get_keyboard()
                    .expect("Seat had no keyboard")
                    .set_focus(&mut self.state, focus, SERIAL_COUNTER.next_serial());

                self.state.schedule_render(&output);
            }
        }
        if !window.is_override_redirect() {
            tracing::debug!("set mapped to false");
            window.set_mapped(false).expect("failed to unmap x11 win");
        }
    }

    fn destroyed_window(&mut self, _xwm: XwmId, window: X11Surface) {
        if window.is_override_redirect() {
            self.state
                .override_redirect_windows
                .retain(|win| win != &window);
        }

        self.state.focus_state.focus_stack.retain(|win| {
            win.wl_surface()
                .is_some_and(|surf| Some(surf) != window.wl_surface())
        });
        let win = self
            .state
            .windows
            .iter()
            .find(|elem| {
                matches!(elem, WindowElement::X11(surface) if surface.wl_surface() == window.wl_surface())
            })
            .cloned();
        tracing::debug!("{win:?}");
        if let Some(win) = win {
            tracing::debug!("removing x11 window from windows");
            self.state
                .windows
                .retain(|elem| win.wl_surface() != elem.wl_surface());

            if let Some(output) = win.output(&self.state) {
                self.state.update_windows(&output);
                let focus = self.state.focused_window(&output).map(FocusTarget::Window);
                if let Some(FocusTarget::Window(win)) = &focus {
                    tracing::debug!("Focusing on prev win");
                    self.state.space.raise_element(win, true);
                    if let WindowElement::Wayland(win) = &win {
                        win.toplevel().send_configure();
                    }
                }
                self.state
                    .seat
                    .get_keyboard()
                    .expect("Seat had no keyboard")
                    .set_focus(&mut self.state, focus, SERIAL_COUNTER.next_serial());

                self.state.schedule_render(&output);
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
        tracing::debug!("configure_request with geo {geo:?}");
        if let Err(err) = window.configure(geo) {
            tracing::error!("Failed to configure x11 win: {err}");
        }
    }

    fn configure_notify(
        &mut self,
        _xwm: XwmId,
        window: X11Surface,
        geometry: Rectangle<i32, Logical>,
        _above: Option<smithay::reexports::x11rb::protocol::xproto::Window>,
    ) {
        let Some(win) = self
            .state
            .space
            .elements()
            .find(|elem| matches!(elem, WindowElement::X11(surface) if surface == &window))
            .cloned()
        else {
            return;
        };
        tracing::debug!("configure notify with geo: {geometry:?}");
        self.state.space.map_element(win, geometry.loc, true);
    }

    fn maximize_request(&mut self, _xwm: XwmId, window: X11Surface) {
        window
            .set_maximized(true)
            .expect("failed to set x11 win to maximized");
        let Some(window) = (|| self.state.window_for_surface(&window.wl_surface()?))() else {
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
        let Some(window) = (|| self.state.window_for_surface(&window.wl_surface()?))() else {
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
        // TODO: fix this mess
        let Some(window) = (|| self.state.window_for_surface(&window.wl_surface()?))() else {
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
        let Some(window) = (|| self.state.window_for_surface(&window.wl_surface()?))() else {
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
        let seat = self.state.seat.clone();

        // We use the server one and not the client because windows like Steam don't provide
        // GrabStartData, so we need to create it ourselves.
        crate::grab::resize_grab::resize_request_server(
            &mut self.state,
            &wl_surf,
            &seat,
            SERIAL_COUNTER.next_serial(),
            resize_edge.into(),
            button,
        );
    }

    fn move_request(&mut self, _xwm: XwmId, window: X11Surface, button: u32) {
        let Some(wl_surf) = window.wl_surface() else { return };
        let seat = self.state.seat.clone();

        // We use the server one and not the client because windows like Steam don't provide
        // GrabStartData, so we need to create it ourselves.
        crate::grab::move_grab::move_request_server(
            &mut self.state,
            &wl_surf,
            &seat,
            SERIAL_COUNTER.next_serial(),
            button,
        );
    }

    fn allow_selection_access(&mut self, xwm: XwmId, _selection: SelectionTarget) -> bool {
        self.state
            .seat
            .get_keyboard()
            .and_then(|kb| kb.current_focus())
            .is_some_and(|focus| {
                if let FocusTarget::Window(WindowElement::X11(surface)) = focus {
                    surface.xwm_id().expect("x11surface had no xwm id") == xwm
                } else {
                    false
                }
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
                    request_data_device_client_selection(&self.state.seat, mime_type, fd)
                {
                    tracing::error!(
                        ?err,
                        "Failed to request current wayland clipboard for XWayland"
                    );
                }
            }
            SelectionTarget::Primary => {
                if let Err(err) = request_primary_client_selection(&self.state.seat, mime_type, fd)
                {
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
                set_data_device_selection(
                    &self.state.display_handle,
                    &self.state.seat,
                    mime_types,
                    (),
                );
            }
            SelectionTarget::Primary => {
                set_primary_selection(&self.state.display_handle, &self.state.seat, mime_types, ());
            }
        }
    }

    fn cleared_selection(&mut self, _xwm: XwmId, selection: SelectionTarget) {
        match selection {
            SelectionTarget::Clipboard => {
                if current_data_device_selection_userdata(&self.state.seat).is_some() {
                    clear_data_device_selection(&self.state.display_handle, &self.state.seat);
                }
            }
            SelectionTarget::Primary => {
                if current_primary_selection_userdata(&self.state.seat).is_some() {
                    clear_primary_selection(&self.state.display_handle, &self.state.seat);
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
