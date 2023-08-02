// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0


use smithay::{
    reexports::wayland_server::Resource,
    utils::{Logical, Rectangle, SERIAL_COUNTER, Point},
    wayland::compositor::{self, CompositorHandler},
    xwayland::{
        xwm::{Reorder, XwmId, WmWindowType},
        X11Surface, X11Wm, XwmHandler,
    },
};

use crate::{
    backend::Backend,
    state::{CalloopData, WithState},
    window::{WindowBlocker, WindowElement, BLOCKER_COUNTER, window_state::Float}, focus::FocusTarget,
};

impl<B: Backend> XwmHandler for CalloopData<B> {
    fn xwm_state(&mut self, _xwm: XwmId) -> &mut X11Wm {
        self.state.xwm.as_mut().expect("xwm not in state")
    }

    fn new_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

    fn new_override_redirect_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

    fn map_window_request(&mut self, _xwm: XwmId, window: X11Surface) {
        tracing::trace!("map_window_request");
        let win_type = window.window_type();
        tracing::debug!("window type is {win_type:?}");
        // tracing::debug!("new x11 window from map_window_request");
        // tracing::debug!("window popup is {}", window.is_popup());

        // INFO: This check is here because it happened while launching Ori and the Will of the Wisps
        if window.is_override_redirect() { 
            let loc = window.geometry().loc;
            let window = WindowElement::X11(window);
            // tracing::debug!("mapped_override_redirect_window to loc {loc:?}");
            self.state.space.map_element(window, loc, true);
            return;
        }

        let window = WindowElement::X11(window);
        self.state.space.map_element(window.clone(), (0, 0), true);
        let bbox = self.state.space.element_bbox(&window).expect("called element_bbox on an unmapped window");

        let output_size = self.state
            .focus_state
            .focused_output
            .as_ref()
            .and_then(|op| self.state.space.output_geometry(op))
            .map(|geo| geo.size)
            .unwrap_or((2, 2).into());

        let output_loc = self.state
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
            output_loc.y + output_size.h / 2 - bbox.size.h / 2
        ).into();

        let WindowElement::X11(surface) = &window else { unreachable!() };
        surface.set_mapped(true).expect("failed to map x11 window");

        self.state.space.map_element(window.clone(), loc, true);
        let bbox = Rectangle::from_loc_and_size(loc, bbox.size);

        tracing::debug!("map_window_request, configuring with bbox {bbox:?}");
        surface
            .configure(bbox)
            .expect("failed to configure x11 window");
        // TODO: ssd


        // TODO: this is a duplicate of the code in new_toplevel,
        // |     move into its own function
        {
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

            let WindowElement::X11(surface) = &window else { unreachable!() };

            if should_float(surface) {
                window.with_state(|state| {
                    state.floating = Float::Floating;
                    // TODO: this doesn't correctly tile intellij idea
                });
            }

            let windows_on_output = self
                .state
                .windows
                .iter()
                .filter(|win| {
                    win.with_state(|state| {
                        self.state
                            .focus_state
                            .focused_output
                            .as_ref()
                            .expect("no focused output")
                            .with_state(|op_state| {
                                op_state
                                    .tags
                                    .iter()
                                    .any(|tag| state.tags.iter().any(|tg| tg == tag))
                            })
                    })
                })
                .cloned()
                .collect::<Vec<_>>();

            self.state.windows.push(window.clone());
            if let Some(focused_output) = self.state.focus_state.focused_output.clone() {
                focused_output.with_state(|state| {
                    let first_tag = state.focused_tags().next();
                    if let Some(first_tag) = first_tag {
                        first_tag.layout().layout(
                            self.state.windows.clone(),
                            state.focused_tags().cloned().collect(),
                            &mut self.state,
                            &focused_output,
                        );
                    }
                });
                BLOCKER_COUNTER.store(1, std::sync::atomic::Ordering::SeqCst);
                tracing::debug!(
                    "blocker {}",
                    BLOCKER_COUNTER.load(std::sync::atomic::Ordering::SeqCst)
                );
                for win in windows_on_output.iter() {
                    if let Some(surf) = win.wl_surface() {
                        compositor::add_blocker(&surf, WindowBlocker);
                    }
                }
                let clone = window.clone();
                self.state.loop_handle.insert_idle(move |data| {
                    crate::state::schedule_on_commit(data, vec![clone.clone()], move |data| {
                        BLOCKER_COUNTER.store(0, std::sync::atomic::Ordering::SeqCst);
                        tracing::debug!(
                            "blocker {}",
                            BLOCKER_COUNTER.load(std::sync::atomic::Ordering::SeqCst)
                        );
                        for client in windows_on_output
                            .iter()
                            .filter_map(|win| win.wl_surface()?.client())
                        {
                            data.state
                                .client_compositor_state(&client)
                                .blocker_cleared(&mut data.state, &data.display.handle())
                        }

                        // Schedule the popup to raise when all windows have committed after having
                        // their blockers cleared
                        crate::state::schedule_on_commit(data, windows_on_output, move |dt| {
                            let WindowElement::X11(surface) = &clone else { unreachable!() };
                            if should_float(surface) {
                                if let Some(xwm) = dt.state.xwm.as_mut() {
                                    tracing::debug!("raising x11 popup");
                                    xwm.raise_window(surface).expect("failed to raise x11 win");
                                    dt.state.space.raise_element(&clone, true);
                                }
                            }
                        });
                    });
                });
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
    }

    // fn map_window_notify(&mut self, xwm: XwmId, window: X11Surface) {
    //     //
    // }

    fn mapped_override_redirect_window(&mut self, _xwm: XwmId, window: X11Surface) {
        tracing::info!("MAPPED OVERRIDE REDIRECT WINDOW");
        let win_type = window.window_type();
        tracing::debug!("window type is {win_type:?}");
        let loc = window.geometry().loc;
        let window = WindowElement::X11(window);
        // tracing::debug!("mapped_override_redirect_window to loc {loc:?}");
        self.state.space.map_element(window.clone(), loc, true);
    }

    fn unmapped_window(&mut self, _xwm: XwmId, window: X11Surface) {
        tracing::debug!("UNMAPPED WINDOW");
        let win = self
            .state
            .space
            .elements()
            .find(|elem| {
                matches!(elem, 
                    WindowElement::X11(surface) if surface == &window)
            })
            .cloned();
        if let Some(win) = win {
            self.state.space.unmap_elem(&win);
            // self.state.windows.retain(|elem| &win != elem);
            // if win.with_state(|state| state.floating.is_tiled()) {
            //     if let Some(output) = win.output(&self.state) {
            //         self.state.re_layout(&output);
            //     }
            // }
        }
        if !window.is_override_redirect() {
            tracing::debug!("set mapped to false");
            window.set_mapped(false).expect("failed to unmap x11 win");
        }
    }

    fn destroyed_window(&mut self, _xwm: XwmId, window: X11Surface) {
        let win = self
            .state
            .windows
            .iter()
            .find(|elem| {
                matches!(elem, 
                    WindowElement::X11(surface) if surface.wl_surface() == window.wl_surface())
            })
            .cloned();
        tracing::debug!("{win:?}");
        if let Some(win) = win {
            tracing::debug!("removing x11 window from windows");
            self.state.windows.retain(|elem| win.wl_surface() != elem.wl_surface());
            if win.with_state(|state| state.floating.is_tiled()) {
                if let Some(output) = win.output(&self.state) {
                    self.state.re_layout(&output);
                }
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
        if window.is_override_redirect() {
            let keyboard = self.state.seat.get_keyboard().unwrap();
            keyboard.set_focus(&mut self.state, Some(FocusTarget::Window(WindowElement::X11(window.clone()))), SERIAL_COUNTER.next_serial());
        }
        // tracing::debug!("x11 configure_notify");
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
        // for output in self.state.space.outputs_for_element(&win) {
        //     win.send_frame(&output, self.state.clock.now(), Some(Duration::ZERO), surface_primary_scanout_output);
        // }
        // TODO: anvil has a TODO here
    }

    // fn maximize_request(&mut self, xwm: XwmId, window: X11Surface) {
    //     // TODO:
    // }
    //
    // fn unmaximize_request(&mut self, xwm: XwmId, window: X11Surface) {
    //     // TODO:
    // }
    //
    // fn fullscreen_request(&mut self, xwm: XwmId, window: X11Surface) {
    //     // TODO:
    //     // window.set_fullscreen(true).unwrap();
    // }
    //
    // fn unfullscreen_request(&mut self, xwm: XwmId, window: X11Surface) {
    //     // TODO:
    // }

    fn resize_request(
        &mut self,
        _xwm: XwmId,
        _window: X11Surface,
        _button: u32,
        _resize_edge: smithay::xwayland::xwm::ResizeEdge,
    ) {
        // TODO:
    }

    fn move_request(&mut self, _xwm: XwmId, _window: X11Surface, _button: u32) {
        // TODO:
    }

    // TODO: allow_selection_access

    // TODO: send_selection

    // TODO: new_selection

    // TODO: cleared_selection
}

/// Make assumptions on whether or not the surface should be floating.
///
/// This logic is taken from the Sway function `wants_floating` in sway/desktop/xwayland.c.
fn should_float(surface: &X11Surface) -> bool {
    let is_popup_by_type = surface.window_type().is_some_and(|typ| {
        matches!(typ, 
            WmWindowType::Dialog 
            | WmWindowType::Utility 
            | WmWindowType::Toolbar 
            | WmWindowType::Splash)
    });
    let is_popup_by_size = surface.size_hints().map_or(false, |size_hints| {
        let Some((min_w, min_h)) = size_hints.min_size else { return false };
        let Some((max_w, max_h)) = size_hints.max_size else { return false };
        min_w > 0 && min_h > 0 && (min_w == max_w || min_h == max_h)
    });
    surface.is_popup() || is_popup_by_type || is_popup_by_size
}
