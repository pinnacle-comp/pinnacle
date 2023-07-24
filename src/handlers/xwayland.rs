// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use smithay::{
    desktop::space::SpaceElement,
    input::pointer::Focus,
    reexports::wayland_server::Resource,
    utils::{Rectangle, SERIAL_COUNTER},
    wayland::compositor::{self, CompositorHandler},
    xwayland::{xwm::XwmId, X11Wm, XwmHandler},
};

use crate::{
    backend::Backend,
    grab::resize_grab::{ResizeSurfaceGrab, ResizeSurfaceState},
    state::{CalloopData, WithState},
    window::{WindowBlocker, WindowElement, BLOCKER_COUNTER},
};

impl<B: Backend> XwmHandler for CalloopData<B> {
    fn xwm_state(&mut self, xwm: XwmId) -> &mut X11Wm {
        self.state.xwm.as_mut().expect("xwm not in state")
    }

    fn new_window(&mut self, xwm: XwmId, window: smithay::xwayland::X11Surface) {}

    fn new_override_redirect_window(&mut self, xwm: XwmId, window: smithay::xwayland::X11Surface) {}

    fn map_window_request(&mut self, xwm: XwmId, window: smithay::xwayland::X11Surface) {
        tracing::debug!("new x11 window from map_window_request");
        window.set_mapped(true).expect("failed to map x11 window");
        let window = WindowElement::X11(window);
        // TODO: place the window in the space
        self.state.space.map_element(window.clone(), (0, 0), true);
        let bbox = self
            .state
            .space
            .element_bbox(&window)
            .expect("failed to get x11 bbox");
        let WindowElement::X11(surface) = &window else { unreachable!() };
        surface
            .configure(Some(bbox))
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
                            .unwrap()
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
            // self.space.map_element(window.clone(), (0, 0), true);
            if let Some(focused_output) = self.state.focus_state.focused_output.clone() {
                focused_output.with_state(|state| {
                    let first_tag = state.focused_tags().next();
                    if let Some(first_tag) = first_tag {
                        first_tag.layout().layout(
                            self.state.windows.clone(),
                            state.focused_tags().cloned().collect(),
                            &self.state.space,
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
                self.state.loop_handle.insert_idle(|data| {
                    crate::state::schedule_on_commit(data, vec![clone], move |data| {
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
                        window.wl_surface(),
                        SERIAL_COUNTER.next_serial(),
                    );
            });
        }
    }

    fn mapped_override_redirect_window(
        &mut self,
        xwm: XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        let loc = window.geometry().loc;
        let window = WindowElement::X11(window);
        self.state.space.map_element(window, loc, true);
    }

    fn unmapped_window(&mut self, xwm: XwmId, window: smithay::xwayland::X11Surface) {
        let win = self
            .state
            .space
            .elements()
            .find(|elem| matches!(elem, WindowElement::X11(surface) if surface == &window))
            .cloned();
        if let Some(win) = win {
            self.state.space.unmap_elem(&win);
        }
        if !window.is_override_redirect() {
            window.set_mapped(false).expect("failed to unmap x11 win");
        }
    }

    fn destroyed_window(&mut self, xwm: XwmId, window: smithay::xwayland::X11Surface) {}

    fn configure_request(
        &mut self,
        xwm: XwmId,
        window: smithay::xwayland::X11Surface,
        x: Option<i32>,
        y: Option<i32>,
        w: Option<u32>,
        h: Option<u32>,
        reorder: Option<smithay::xwayland::xwm::Reorder>,
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
        xwm: XwmId,
        window: smithay::xwayland::X11Surface,
        geometry: smithay::utils::Rectangle<i32, smithay::utils::Logical>,
        above: Option<smithay::reexports::x11rb::protocol::xproto::Window>,
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
        self.state.space.map_element(win, geometry.loc, false);
        // TODO: anvil has a TODO here
    }

    // TODO: maximize request

    // TODO: unmaximize request

    // TODO: fullscreen request

    // TODO: unfullscreen request

    fn resize_request(
        &mut self,
        xwm: XwmId,
        window: smithay::xwayland::X11Surface,
        button: u32,
        resize_edge: smithay::xwayland::xwm::ResizeEdge,
    ) {
        let seat = &self.state.seat;
        let pointer = seat.get_pointer().expect("failed to get pointer");
        let start_data = pointer.grab_start_data().expect("no grab start data");

        let Some(win) = self
            .state
            .space
            .elements()
            .find(|elem| matches!(elem, WindowElement::X11(surface) if surface == &window))
        else {
            return;
        };

        let initial_window_location = self
            .state
            .space
            .element_location(win)
            .expect("failed to get x11 loc");
        let initial_window_size = win.geometry().size;

        if let Some(wl_surface) = win.wl_surface() {
            wl_surface.with_state(|state| {
                state.resize_state = ResizeSurfaceState::Resizing {
                    edges: resize_edge.into(),
                    initial_window_rect: Rectangle::from_loc_and_size(
                        initial_window_location,
                        initial_window_size,
                    ),
                };
            });

            let grab = ResizeSurfaceGrab::start(
                start_data,
                win.clone(),
                resize_edge.into(),
                Rectangle::from_loc_and_size(initial_window_location, initial_window_size),
                0x110, // BUTTON_LEFT
            );

            if let Some(grab) = grab {
                pointer.set_grab(
                    &mut self.state,
                    grab,
                    SERIAL_COUNTER.next_serial(),
                    Focus::Clear,
                );
            }
        }
    }

    fn move_request(&mut self, xwm: XwmId, window: smithay::xwayland::X11Surface, button: u32) {
        todo!()
    }

    // TODO: allow_selection_access

    // TODO: send_selection

    // TODO: new_selection

    // TODO: cleared_selection
}
