// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0


use smithay::{
    reexports::wayland_server::Resource,
    utils::{Logical, Rectangle, SERIAL_COUNTER},
    wayland::compositor::{self, CompositorHandler},
    xwayland::{
        xwm::{Reorder, XwmId},
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
        tracing::debug!("-----MAP WINDOW REQUEST");
        // tracing::debug!("new x11 window from map_window_request");
        // tracing::debug!("window popup is {}", window.is_popup());
        //
        // TODO: TOMORROW: figure out why keyboard input isn't going to games (prolly you never
        // |     change keyboard focus)

        if window.is_override_redirect() {
            let loc = window.geometry().loc;
            let window = WindowElement::X11(window);
            // tracing::debug!("mapped_override_redirect_window to loc {loc:?}");
            self.state.space.map_element(window, loc, true);
            return;
        }
        window.set_mapped(true).expect("failed to map x11 window");
        let window = WindowElement::X11(window);
        self.state.space.map_element(window.clone(), (0, 0), true);
        let bbox = self.state.space.element_bbox(&window).expect("called element_bbox on an unmapped window");
        let WindowElement::X11(surface) = &window else { unreachable!() };
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

            window.with_state(|state| {
                let WindowElement::X11(surface) = &window else { unreachable!() };
                let is_popup = surface.window_type().is_some_and(|typ| !matches!(typ, smithay::xwayland::xwm::WmWindowType::Normal));
                if surface.is_popup() || is_popup || surface.min_size() == surface.max_size() {
                    state.floating = Float::Floating;
                }
            });


            // self.state.space.map_element(window.clone(), (0, 0), true);
            // self.state.space.raise_element(&window, true);
            // let WindowElement::X11(surface) = &window else { unreachable!() };
            // self.state.xwm.as_mut().unwrap().raise_window(surface).unwrap();


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


                        // data.state.loop_handle.insert_idle(move |dt| {
                        //
                        //     let WindowElement::X11(surface) = &clone else { unreachable!() };
                        //     let is_popup = surface.window_type().is_some_and(|typ| !matches!(typ, smithay::xwayland::xwm::WmWindowType::Normal));
                        //     if surface.is_popup() || is_popup || surface.min_size() == surface.max_size() {
                        //         if let Some(xwm) = dt.state.xwm.as_mut() {
                        //             tracing::debug!("raising x11 modal");
                        //             xwm.raise_window(surface).expect("failed to raise x11 win");
                        //             dt.state.space.raise_element(&clone, true);
                        //         }
                        //     }
                        // });
                    })
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
        let loc = window.geometry().loc;
        let window = WindowElement::X11(window);
        // tracing::debug!("mapped_override_redirect_window to loc {loc:?}");
        self.state.space.map_element(window, loc, true);
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
