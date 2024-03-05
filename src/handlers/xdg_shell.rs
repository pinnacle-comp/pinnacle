use smithay::{
    delegate_xdg_shell,
    desktop::{
        find_popup_root_surface, layer_map_for_output, PopupKeyboardGrab, PopupKind,
        PopupPointerGrab, PopupUngrabStrategy, Window, WindowSurfaceType,
    },
    input::{pointer::Focus, Seat},
    output::Output,
    reexports::{
        wayland_protocols::xdg::shell::server::{
            xdg_positioner::{Anchor, ConstraintAdjustment, Gravity},
            xdg_toplevel::{self, ResizeEdge},
        },
        wayland_server::{
            protocol::{wl_output::WlOutput, wl_seat::WlSeat},
            Resource,
        },
    },
    utils::{Logical, Point, Rectangle, Serial, SERIAL_COUNTER},
    wayland::{
        seat::WaylandFocus,
        shell::xdg::{
            PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
        },
    },
};

use crate::{
    focus::keyboard::KeyboardFocusTarget,
    state::{State, WithState},
    window::WindowElement,
};

impl XdgShellHandler for State {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::TiledTop);
            state.states.set(xdg_toplevel::State::TiledBottom);
            state.states.set(xdg_toplevel::State::TiledLeft);
            state.states.set(xdg_toplevel::State::TiledRight);
        });

        let window = WindowElement::new(Window::new_wayland_window(surface.clone()));
        self.new_windows.push(window);
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        tracing::debug!("toplevel destroyed");
        self.windows.retain(|window| {
            window
                .wl_surface()
                .is_some_and(|surf| &surf != surface.wl_surface())
        });

        self.z_index_stack.stack.retain(|window| {
            window
                .wl_surface()
                .is_some_and(|surf| &surf != surface.wl_surface())
        });

        for output in self.space.outputs() {
            output.with_state_mut(|state| {
                state.focus_stack.stack.retain(|window| {
                    window
                        .wl_surface()
                        .is_some_and(|surf| &surf != surface.wl_surface())
                })
            });
        }

        let Some(window) = self.window_for_surface(surface.wl_surface()) else {
            return;
        };

        if let Some(output) = window.output(self) {
            self.update_windows(&output);
            let focus = self
                .focused_window(&output)
                .map(KeyboardFocusTarget::Window);
            if let Some(KeyboardFocusTarget::Window(win)) = &focus {
                tracing::debug!("Focusing on prev win");
                // TODO:
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

    // this is 500 lines there has to be a shorter way to do this
    fn new_popup(&mut self, surface: PopupSurface, mut positioner: PositionerState) {
        tracing::debug!(?positioner.constraint_adjustment, ?positioner.gravity);
        let output_rect = self
            .focused_output()
            .and_then(|op| self.space.output_geometry(op));

        /// Horizontal direction
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum DirH {
            Left,
            Right,
        }

        /// Vertical direction
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum DirV {
            Top,
            Bottom,
        }

        let mut is_subpopup = false;

        // We have to go through this really verbose way of getting the location of the popup in
        // the global space.
        //
        // The location from PopupSurface.with_pending_state's state.geometry.loc is relative to
        // its parent. When its parent is a window, we can simply add the window's location to the
        // popup's to get its global location.
        //
        // However, if its parent is another popup, we need to step upwards and grab the location
        // from each popup, aggregating them into one global space location.
        let global_loc = {
            let mut surf = surface.get_parent_surface();
            let mut loc = surface.with_pending_state(|state| state.geometry.loc);
            tracing::debug!(?loc);

            while let Some(s) = &surf {
                if let Some(popup) = self.popup_manager.find_popup(s) {
                    // popup.geometry() doesn't return the right location, so we dive into the
                    // PopupSurface's state to grab it.
                    let PopupKind::Xdg(popup_surf) = &popup else { return }; // TODO:
                    let l = popup_surf.with_pending_state(|state| state.geometry.loc);
                    tracing::debug!(loc = ?l, "parent is popup");
                    loc += l;
                    is_subpopup = true;
                    surf = popup_surf.get_parent_surface();
                } else if let Some(win) = self.window_for_surface(s) {
                    // Once we reach a window, we can stop as windows are already in the global space.
                    tracing::debug!("parent is window");
                    loc += self
                        .space
                        .element_location(&win)
                        .unwrap_or_else(|| (0, 0).into());
                    tracing::debug!(?loc);
                    break;
                }
            }

            loc
        };

        let mut popup_rect_global = {
            let mut rect = positioner.get_geometry();
            rect.loc = global_loc;
            rect
        };

        tracing::debug!(?global_loc);

        // The final rectangle that the popup is set to needs to be relative to its parent,
        // so we store its old location here and subtract it at the end.
        let loc_diff = global_loc - surface.with_pending_state(|state| state.geometry.loc);

        if let Some(output_rect) = output_rect {
            // Check if the rect is constrained horizontally, and if so, which side.
            let constrained_x = |rect: Rectangle<i32, Logical>| -> Option<DirH> {
                tracing::debug!(?rect, ?output_rect);

                if rect.loc.x < output_rect.loc.x {
                    Some(DirH::Left)
                } else if rect.loc.x + rect.size.w > output_rect.loc.x + output_rect.size.w {
                    Some(DirH::Right)
                } else {
                    None
                }
            };

            // Check if the rect is constrained vertically, and if so, which side.
            let constrained_y = |rect: Rectangle<i32, Logical>| -> Option<DirV> {
                tracing::debug!(?rect, ?output_rect);

                if rect.loc.y < output_rect.loc.y {
                    Some(DirV::Top)
                } else if rect.loc.y + rect.size.h > output_rect.loc.y + output_rect.size.h {
                    Some(DirV::Bottom)
                } else {
                    None
                }
            };

            // We're going to need to position popups such that they stay fully onscreen.
            // We can use the provided `positioner.constraint_adjustment` to get hints on how
            // the popups want to be relocated.

            let output_left_x = output_rect.loc.x;
            let output_right_x = output_rect.loc.x + output_rect.size.w;
            let output_top_y = output_rect.loc.y;
            let output_bottom_y = output_rect.loc.y + output_rect.size.h;

            // The popup is flowing offscreen in the horizontal direction.
            if let Some(constrain_dir) = constrained_x(popup_rect_global) {
                tracing::debug!("Popup was constrained on the x axis, repositioning");
                let gravity = match positioner.gravity {
                    Gravity::Left | Gravity::TopLeft | Gravity::BottomLeft => DirH::Left,
                    _ => DirH::Right,
                };

                tracing::debug!(?gravity);
                tracing::debug!(?constrain_dir);

                'block: {
                    // If the constraint_adjustment has SlideX, we attempt to slide the popup
                    // towards the direction specified by positioner.gravity until  TODO:
                    if positioner
                        .constraint_adjustment
                        .contains(ConstraintAdjustment::SlideX)
                        && !is_subpopup
                    // If it's a subpopup, flip instead of slide. This makes
                    // stuff like Firefox nested dropdowns more intuitive.
                    {
                        tracing::debug!("Attempting to slide popup X");
                        // Slide towards the gravity until the opposite edge is unconstrained or the
                        // same edge is constrained
                        match (&constrain_dir, &gravity) {
                            (DirH::Left, DirH::Right) => {
                                let len_until_same_edge_constrained = output_right_x
                                    - (popup_rect_global.loc.x + popup_rect_global.size.w);
                                let len_until_opp_edge_unconstrained =
                                    output_left_x - popup_rect_global.loc.x;

                                popup_rect_global.loc.x += i32::min(
                                    len_until_same_edge_constrained,
                                    len_until_opp_edge_unconstrained,
                                )
                                .max(0);
                                tracing::debug!(
                                    ?popup_rect_global,
                                    "Constrained SlideX left right"
                                );
                            }
                            (DirH::Right, DirH::Left) => {
                                let len_until_same_edge_constrained =
                                    popup_rect_global.loc.x - output_left_x;
                                let len_until_opp_edge_unconstrained = (popup_rect_global.loc.x
                                    + popup_rect_global.size.w)
                                    - output_right_x;

                                popup_rect_global.loc.x -= i32::min(
                                    len_until_opp_edge_unconstrained,
                                    len_until_same_edge_constrained,
                                )
                                .max(0);
                                tracing::debug!(
                                    ?popup_rect_global,
                                    "Constrained SlideX right left"
                                );
                            }
                            _ => (),
                        };

                        if constrained_x(popup_rect_global).is_none() {
                            break 'block;
                        }

                        // Do the same but in the other direction
                        match (constrain_dir, gravity) {
                            (DirH::Right, DirH::Right) => {
                                let len_until_same_edge_unconstrained =
                                    popup_rect_global.loc.x - output_left_x;
                                let len_until_opp_edge_constrained = (popup_rect_global.loc.x
                                    + popup_rect_global.size.w)
                                    - output_right_x;

                                popup_rect_global.loc.x -= i32::min(
                                    len_until_opp_edge_constrained,
                                    len_until_same_edge_unconstrained,
                                )
                                .max(0);
                                tracing::debug!(
                                    ?popup_rect_global,
                                    "Constrained SlideX right right"
                                );
                            }
                            (DirH::Left, DirH::Left) => {
                                let len_until_same_edge_unconstrained = output_right_x
                                    - (popup_rect_global.loc.x + popup_rect_global.size.w);
                                let len_until_opp_edge_constrained =
                                    output_left_x - popup_rect_global.loc.x;

                                popup_rect_global.loc.x += i32::min(
                                    len_until_same_edge_unconstrained,
                                    len_until_opp_edge_constrained,
                                )
                                .max(0);
                                tracing::debug!(?popup_rect_global, "Constrained SlideX left left");
                            }
                            _ => (),
                        };

                        if constrained_x(popup_rect_global).is_none() {
                            break 'block;
                        }
                    }

                    // If the above didn't bring the popup onscreen or if it's a nested popup, flip it.
                    if positioner
                        .constraint_adjustment
                        .contains(ConstraintAdjustment::FlipX)
                    {
                        tracing::debug!("Attempting to flip popup X");
                        let old_gravity = positioner.gravity;
                        positioner.gravity = match positioner.gravity {
                            Gravity::Left => Gravity::Right,
                            Gravity::Right => Gravity::Left,
                            Gravity::TopLeft => Gravity::TopRight,
                            Gravity::BottomLeft => Gravity::BottomRight,
                            Gravity::TopRight => Gravity::TopLeft,
                            Gravity::BottomRight => Gravity::BottomLeft,
                            rest => rest,
                        };

                        let old_anchor = positioner.anchor_edges;
                        positioner.anchor_edges = match positioner.anchor_edges {
                            Anchor::Left => Anchor::Right,
                            Anchor::Right => Anchor::Left,
                            Anchor::TopLeft => Anchor::TopRight,
                            Anchor::BottomLeft => Anchor::BottomRight,
                            Anchor::TopRight => Anchor::TopLeft,
                            Anchor::BottomRight => Anchor::BottomLeft,
                            rest => rest,
                        };

                        let mut relative_geo = positioner.get_geometry();
                        relative_geo.loc += loc_diff;
                        tracing::debug!(?relative_geo, "FlipX");
                        if constrained_x(relative_geo).is_none() {
                            popup_rect_global = relative_geo;
                            break 'block;
                        }

                        // The protocol states that if flipping it didn't bring it onscreen,
                        // then it should just stay at its unflipped state.
                        positioner.gravity = old_gravity;
                        positioner.anchor_edges = old_anchor;
                    }

                    // Finally, if flipping it failed, resize it to fit.
                    if positioner
                        .constraint_adjustment
                        .contains(ConstraintAdjustment::ResizeX)
                    {
                        tracing::debug!("Resizing popup X");
                        // Slice off the left side
                        if popup_rect_global.loc.x < output_left_x {
                            let len_to_slice = output_left_x - popup_rect_global.loc.x;
                            let new_top_left: Point<i32, Logical> = (
                                popup_rect_global.loc.x + len_to_slice,
                                popup_rect_global.loc.y,
                            )
                                .into();
                            let bottom_right: Point<i32, Logical> = (
                                popup_rect_global.loc.x + popup_rect_global.size.w,
                                popup_rect_global.loc.y + popup_rect_global.size.h,
                            )
                                .into();
                            popup_rect_global =
                                Rectangle::from_extemities(new_top_left, bottom_right);
                        }

                        // Slice off the right side
                        if popup_rect_global.loc.x + popup_rect_global.size.w > output_right_x {
                            let len_to_slice = (popup_rect_global.loc.x + popup_rect_global.size.w)
                                - output_right_x;
                            let top_left = popup_rect_global.loc;
                            let new_bottom_right: Point<i32, Logical> = (
                                popup_rect_global.loc.x + popup_rect_global.size.w - len_to_slice,
                                popup_rect_global.loc.y + popup_rect_global.size.h,
                            )
                                .into();
                            popup_rect_global =
                                Rectangle::from_extemities(top_left, new_bottom_right);
                        }

                        if constrained_x(popup_rect_global).is_none() {
                            break 'block;
                        }
                    }
                }
            }

            // The popup is flowing offscreen in the vertical direction.
            if let Some(constrain_dir) = constrained_y(popup_rect_global) {
                tracing::debug!("Popup was constrained on the y axis, repositioning");
                let gravity = match positioner.gravity {
                    Gravity::Top | Gravity::TopLeft | Gravity::TopRight => DirV::Top,
                    _ => DirV::Bottom,
                };

                tracing::debug!(?gravity);
                tracing::debug!(?constrain_dir);

                'block: {
                    // If the constraint_adjustment has SlideY, we attempt to slide the popup
                    // towards the direction specified by positioner.gravity until  TODO:
                    if positioner
                        .constraint_adjustment
                        .contains(ConstraintAdjustment::SlideY)
                        && !is_subpopup
                    // If it's a subpopup, flip instead of slide. This makes
                    // stuff like Firefox nested dropdowns more intuitive.
                    {
                        // Slide towards the gravity until the opposite edge is unconstrained or the
                        // same edge is constrained
                        match (&constrain_dir, &gravity) {
                            (DirV::Top, DirV::Bottom) => {
                                let len_until_same_edge_constrained = output_bottom_y
                                    - (popup_rect_global.loc.y + popup_rect_global.size.h);
                                let len_until_opp_edge_unconstrained =
                                    output_top_y - popup_rect_global.loc.y;

                                popup_rect_global.loc.y += i32::min(
                                    len_until_same_edge_constrained,
                                    len_until_opp_edge_unconstrained,
                                )
                                .max(0);
                            }
                            (DirV::Bottom, DirV::Top) => {
                                let len_until_same_edge_constrained =
                                    popup_rect_global.loc.y - output_top_y;
                                let len_until_opp_edge_unconstrained = (popup_rect_global.loc.y
                                    + popup_rect_global.size.h)
                                    - output_bottom_y;

                                popup_rect_global.loc.y -= i32::min(
                                    len_until_opp_edge_unconstrained,
                                    len_until_same_edge_constrained,
                                )
                                .max(0);
                            }
                            _ => (),
                        };

                        if constrained_y(popup_rect_global).is_none() {
                            break 'block;
                        }

                        // Do the same but in the other direction
                        match (constrain_dir, gravity) {
                            (DirV::Bottom, DirV::Bottom) => {
                                let len_until_same_edge_unconstrained =
                                    popup_rect_global.loc.y - output_top_y;
                                let len_until_opp_edge_constrained = (popup_rect_global.loc.y
                                    + popup_rect_global.size.h)
                                    - output_bottom_y;

                                popup_rect_global.loc.y -= i32::min(
                                    len_until_opp_edge_constrained,
                                    len_until_same_edge_unconstrained,
                                )
                                .max(0);
                            }
                            (DirV::Top, DirV::Top) => {
                                let len_until_same_edge_unconstrained = output_bottom_y
                                    - (popup_rect_global.loc.y + popup_rect_global.size.h);
                                let len_until_opp_edge_constrained =
                                    output_top_y - popup_rect_global.loc.y;

                                popup_rect_global.loc.y += i32::min(
                                    len_until_same_edge_unconstrained,
                                    len_until_opp_edge_constrained,
                                )
                                .max(0);
                            }
                            _ => (),
                        };

                        if constrained_y(popup_rect_global).is_none() {
                            break 'block;
                        }
                    }

                    // If the above didn't bring the popup onscreen or if it's a nested popup, flip it.
                    if positioner
                        .constraint_adjustment
                        .contains(ConstraintAdjustment::FlipY)
                    {
                        let old_gravity = positioner.gravity;
                        positioner.gravity = match positioner.gravity {
                            Gravity::Top => Gravity::Bottom,
                            Gravity::Bottom => Gravity::Top,
                            Gravity::TopLeft => Gravity::BottomLeft,
                            Gravity::BottomLeft => Gravity::TopLeft,
                            Gravity::TopRight => Gravity::BottomRight,
                            Gravity::BottomRight => Gravity::TopRight,
                            rest => rest,
                        };

                        let old_anchor = positioner.anchor_edges;
                        positioner.anchor_edges = match positioner.anchor_edges {
                            Anchor::Top => Anchor::Bottom,
                            Anchor::Bottom => Anchor::Top,
                            Anchor::TopLeft => Anchor::BottomLeft,
                            Anchor::BottomLeft => Anchor::TopLeft,
                            Anchor::TopRight => Anchor::BottomRight,
                            Anchor::BottomRight => Anchor::TopRight,
                            rest => rest,
                        };

                        let mut geo = positioner.get_geometry();
                        tracing::debug!(?geo, "Flipped Y geo");
                        geo.loc += loc_diff;
                        geo.loc.x = popup_rect_global.loc.x;
                        tracing::debug!(?geo, "Flipped Y geo global");
                        if constrained_y(geo).is_none() {
                            popup_rect_global = geo;
                            break 'block;
                        }

                        // The protocol states that if flipping it didn't bring it onscreen,
                        // then it should just stay at its unflipped state.
                        positioner.gravity = old_gravity;
                        positioner.anchor_edges = old_anchor;
                    }

                    // Finally, if flipping it failed, resize it to fit.
                    if positioner
                        .constraint_adjustment
                        .contains(ConstraintAdjustment::ResizeY)
                    {
                        // Slice off the top side
                        if popup_rect_global.loc.y < output_top_y {
                            let len_to_slice = output_top_y - popup_rect_global.loc.y;
                            let new_top_left: Point<i32, Logical> = (
                                popup_rect_global.loc.x,
                                popup_rect_global.loc.y + len_to_slice,
                            )
                                .into();
                            let bottom_right: Point<i32, Logical> = (
                                popup_rect_global.loc.x + popup_rect_global.size.w,
                                popup_rect_global.loc.y + popup_rect_global.size.h,
                            )
                                .into();
                            popup_rect_global =
                                Rectangle::from_extemities(new_top_left, bottom_right);
                        }

                        // Slice off the right side
                        if popup_rect_global.loc.y + popup_rect_global.size.h > output_bottom_y {
                            let len_to_slice = (popup_rect_global.loc.y + popup_rect_global.size.h)
                                - output_bottom_y;
                            let top_left = popup_rect_global.loc;
                            let new_bottom_right: Point<i32, Logical> = (
                                popup_rect_global.loc.x + popup_rect_global.size.w,
                                popup_rect_global.loc.y + popup_rect_global.size.h - len_to_slice,
                            )
                                .into();
                            popup_rect_global =
                                Rectangle::from_extemities(top_left, new_bottom_right);
                        }

                        if constrained_y(popup_rect_global).is_none() {
                            break 'block;
                        }
                    }
                }
            }
        }

        tracing::debug!(?popup_rect_global, "New popup");
        popup_rect_global.loc -= loc_diff;

        surface.with_pending_state(|state| state.geometry = popup_rect_global);

        if let Err(err) = self.popup_manager.track_popup(PopupKind::from(surface)) {
            tracing::warn!("failed to track popup: {}", err);
        }
    }

    fn move_request(&mut self, surface: ToplevelSurface, seat: WlSeat, serial: Serial) {
        tracing::debug!("move_request_client");
        const BUTTON_LEFT: u32 = 0x110; // We assume the left mouse button is used
        crate::grab::move_grab::move_request_client(
            self,
            surface.wl_surface(),
            &Seat::from_resource(&seat).expect("couldn't get seat from WlSeat"),
            serial,
            BUTTON_LEFT,
        );
    }

    fn resize_request(
        &mut self,
        surface: ToplevelSurface,
        seat: WlSeat,
        serial: Serial,
        edges: ResizeEdge,
    ) {
        const BUTTON_LEFT: u32 = 0x110;
        crate::grab::resize_grab::resize_request_client(
            self,
            surface.wl_surface(),
            &Seat::from_resource(&seat).expect("couldn't get seat from WlSeat"),
            serial,
            edges.into(),
            BUTTON_LEFT,
        );
    }

    fn reposition_request(
        &mut self,
        surface: PopupSurface,
        positioner: PositionerState,
        token: u32,
    ) {
        // TODO: reposition logic

        surface.with_pending_state(|state| {
            state.geometry = positioner.get_geometry();
            state.positioner = positioner;
        });
        surface.send_repositioned(token);
    }

    fn grab(&mut self, surface: PopupSurface, seat: WlSeat, serial: Serial) {
        let seat: Seat<Self> = Seat::from_resource(&seat).expect("couldn't get seat from WlSeat");
        let popup_kind = PopupKind::Xdg(surface);
        if let Some(root) = find_popup_root_surface(&popup_kind).ok().and_then(|root| {
            self.window_for_surface(&root)
                .map(KeyboardFocusTarget::Window)
                .or_else(|| {
                    self.space.outputs().find_map(|op| {
                        layer_map_for_output(op)
                            .layer_for_surface(&root, WindowSurfaceType::TOPLEVEL)
                            .cloned()
                            .map(KeyboardFocusTarget::LayerSurface)
                    })
                })
        }) {
            if let Ok(mut grab) = self
                .popup_manager
                .grab_popup(root, popup_kind, &seat, serial)
            {
                if let Some(keyboard) = seat.get_keyboard() {
                    if keyboard.is_grabbed()
                        && !(keyboard.has_grab(serial)
                            || keyboard.has_grab(grab.previous_serial().unwrap_or(serial)))
                    {
                        grab.ungrab(PopupUngrabStrategy::All);
                        return;
                    }

                    keyboard.set_focus(self, grab.current_grab(), serial);
                    keyboard.set_grab(PopupKeyboardGrab::new(&grab), serial);
                }
                if let Some(pointer) = seat.get_pointer() {
                    if pointer.is_grabbed()
                        && !(pointer.has_grab(serial)
                            || pointer
                                .has_grab(grab.previous_serial().unwrap_or_else(|| grab.serial())))
                    {
                        grab.ungrab(PopupUngrabStrategy::All);
                        return;
                    }
                    pointer.set_grab(self, PopupPointerGrab::new(&grab), serial, Focus::Keep);
                }
            }
        }
    }

    fn fullscreen_request(&mut self, surface: ToplevelSurface, mut wl_output: Option<WlOutput>) {
        if !surface
            .current_state()
            .capabilities
            .contains(xdg_toplevel::WmCapabilities::Fullscreen)
        {
            return;
        }

        let wl_surface = surface.wl_surface();
        let output = wl_output
            .as_ref()
            .and_then(Output::from_resource)
            .or_else(|| {
                self.window_for_surface(wl_surface)
                    .and_then(|window| self.space.outputs_for_element(&window).first().cloned())
            });

        if let Some(output) = output {
            let Some(geometry) = self.space.output_geometry(&output) else {
                surface.send_configure();
                return;
            };

            let client = self
                .display_handle
                .get_client(wl_surface.id())
                .expect("wl_surface had no client");
            for output in output.client_outputs(&client) {
                wl_output = Some(output);
            }

            surface.with_pending_state(|state| {
                state.states.set(xdg_toplevel::State::Fullscreen);
                state.size = Some(geometry.size);
                state.fullscreen_output = wl_output;
            });

            let Some(window) = self.window_for_surface(wl_surface) else {
                tracing::error!("wl_surface had no window");
                return;
            };

            if !window.with_state(|state| state.fullscreen_or_maximized.is_fullscreen()) {
                window.toggle_fullscreen();
            }
        }

        surface.send_configure();
    }

    fn unfullscreen_request(&mut self, surface: ToplevelSurface) {
        if !surface
            .current_state()
            .states
            .contains(xdg_toplevel::State::Fullscreen)
        {
            return;
        }

        surface.with_pending_state(|state| {
            state.states.unset(xdg_toplevel::State::Fullscreen);
            state.size = None;
            state.fullscreen_output.take();
        });

        surface.send_pending_configure();

        let Some(window) = self.window_for_surface(surface.wl_surface()) else {
            tracing::error!("wl_surface had no window");
            return;
        };

        if window.with_state(|state| state.fullscreen_or_maximized.is_fullscreen()) {
            window.toggle_fullscreen();
        }
    }

    fn maximize_request(&mut self, surface: ToplevelSurface) {
        let Some(window) = self.window_for_surface(surface.wl_surface()) else {
            return;
        };

        if !window.with_state(|state| state.fullscreen_or_maximized.is_maximized()) {
            window.toggle_maximized();
        }

        let Some(output) = window.output(self) else { return };
        self.update_windows(&output);
    }

    fn unmaximize_request(&mut self, surface: ToplevelSurface) {
        let Some(window) = self.window_for_surface(surface.wl_surface()) else {
            return;
        };

        if window.with_state(|state| state.fullscreen_or_maximized.is_maximized()) {
            window.toggle_maximized();
        }

        let Some(output) = window.output(self) else { return };
        self.update_windows(&output);
    }

    fn minimize_request(&mut self, _surface: ToplevelSurface) {
        // TODO:
        // if let Some(window) = self.window_for_surface(surface.wl_surface()) {
        //     self.space.unmap_elem(&window);
        // }
    }

    // TODO: impl the rest of the fns in XdgShellHandler
}
delegate_xdg_shell!(State);
