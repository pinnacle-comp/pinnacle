// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use itertools::Itertools;
use smithay::{
    desktop::{space::SpaceElement, Space, Window},
    output::Output,
    utils::{Logical, Size},
    wayland::{compositor, shell::xdg::XdgToplevelSurfaceData},
};

use crate::window::window_state::{WindowResizeState, WindowState};

pub enum Direction {
    Left,
    Right,
    Top,
    Bottom,
}

pub struct MasterStack<'a, S: SpaceElement> {
    inner: Vec<&'a mut Vec<S>>,
}

pub trait Layout<'a, S: SpaceElement> {
    /// Add a [`SpaceElement`] to this layout and update positions.
    fn add(&mut self, space: &Space<S>, output: &Output, elem: S);
    /// Remove a [`SpaceElement`] from this layout and update positions.
    fn remove(&mut self, space: &Space<S>, output: &Output, elem: &S);

    // TODO: return result
    /// Swap two elements in this layout and update their positions.
    fn swap(&mut self, space: &Space<S>, elem1: &S, elem2: &S);

    /// Perform a full layout with all elements. Use this when you are switching from another layout.
    fn layout(&self, space: &Space<S>, output: &Output);

    fn chain_with(self, vec: &'a mut Vec<S>) -> Self;
}

impl MasterStack<'_, Window> {
    pub fn master(&self) -> Option<&Window> {
        self.inner.iter().flat_map(|vec| vec.iter()).next()
    }

    pub fn stack(&self) -> impl Iterator<Item = &Window> {
        self.inner
            .iter()
            .flat_map(|vec| vec.iter())
            .unique()
            .skip(1)
    }
}

impl MasterStack<'_, Window> {
    fn layout_stack(&self, space: &Space<Window>, output: &Output) {
        let stack_count = self.stack().count();

        let Some(output_geo) = space.output_geometry(output) else {
            tracing::error!("could not get output geometry");
            return;
        };

        let height = output_geo.size.h / stack_count as i32;

        for (i, win) in self.stack().enumerate() {
            win.toplevel().with_pending_state(|state| {
                state.size = Some((output_geo.size.w / 2, height).into());
            });

            WindowState::with(win, |state| {
                state.resize_state = WindowResizeState::WaitingForAck(
                    win.toplevel().send_configure(),
                    (output_geo.size.w / 2, i as i32 * height).into(),
                );
            });
        }
    }
}

pub fn swap_window_positions(space: &Space<Window>, win1: &Window, win2: &Window) {
    // FIXME: moving the mouse quickly will break swapping

    let win1_loc = space.element_location(win1).unwrap(); // TODO: handle unwraps
    let win2_loc = space.element_location(win2).unwrap();
    let win1_geo = win1.geometry();
    let win2_geo = win2.geometry();

    win1.toplevel().with_pending_state(|state| {
        state.size = Some(win2_geo.size);
    });
    win2.toplevel().with_pending_state(|state| {
        state.size = Some(win1_geo.size);
    });

    let serial = win1.toplevel().send_configure();
    WindowState::with(win1, |state| {
        state.resize_state = WindowResizeState::WaitingForAck(serial, win2_loc);
    });

    let serial = win2.toplevel().send_configure();
    WindowState::with(win2, |state| {
        state.resize_state = WindowResizeState::WaitingForAck(serial, win1_loc);
    });
}

impl<'a> Layout<'a, Window> for MasterStack<'a, Window> {
    fn add(&mut self, space: &Space<Window>, output: &Output, elem: Window) {
        for vec in self.inner.iter_mut() {
            vec.push(elem.clone());
        }

        if self.stack().count() == 0 {
            let Some(master) = self.master() else { unreachable!() };
            let Some(output_geo) = space.output_geometry(output) else {
                tracing::error!("could not get output geometry");
                return;
            };
            master.toplevel().with_pending_state(|state| {
                state.size = Some(output_geo.size);
            });

            WindowState::with(master, |state| {
                state.resize_state = WindowResizeState::WaitingForAck(
                    master.toplevel().send_configure(),
                    (0, 0).into(),
                );
            });
        } else if self.stack().count() == 1 {
            let Some(master) = self.master() else { unreachable!() };
            let Some(output_geo) = space.output_geometry(output) else {
                tracing::error!("could not get output geometry");
                return;
            };
            master.toplevel().with_pending_state(|state| {
                state.size = Some((output_geo.size.w / 2, output_geo.size.h).into());
            });

            WindowState::with(master, |state| {
                state.resize_state = WindowResizeState::WaitingForAck(
                    master.toplevel().send_configure(),
                    (0, 0).into(),
                );
            });
            self.layout_stack(space, output);
        } else {
            self.layout_stack(space, output);
        }
    }

    fn remove(&mut self, space: &Space<Window>, output: &Output, elem: &Window) {
        for vec in self.inner.iter_mut() {
            vec.retain(|el| el != elem);
        }

        let Some(master) = self.master() else { return };

        let Some(output_geo) = space.output_geometry(output) else {
            tracing::error!("could not get output geometry");
            return;
        };

        if self.stack().count() == 0 {
            master.toplevel().with_pending_state(|state| {
                state.size = Some(output_geo.size);
            });

            WindowState::with(master, |state| {
                state.resize_state = WindowResizeState::WaitingForAck(
                    master.toplevel().send_configure(),
                    (0, 0).into(),
                );
            });
        } else {
            self.layout_stack(space, output);
        }
    }

    fn swap(&mut self, space: &Space<Window>, elem1: &Window, elem2: &Window) {
        tracing::debug!("top of swap");

        let mut elems = self
            .inner
            .iter_mut()
            .flat_map(|vec| vec.iter_mut())
            .filter(|elem| *elem == elem1 || *elem == elem2)
            .unique_by(|win| WindowState::with(win, |state| state.id));

        let (first, second) = (elems.next(), elems.next());

        if let Some(first) = first {
            if let Some(second) = second {
                std::mem::swap(first, second);
            }
        }

        let wins = self
            .inner
            .iter()
            .map(|vec| {
                vec.iter()
                    .enumerate()
                    .map(|(i, win)| {
                        compositor::with_states(win.toplevel().wl_surface(), |states| {
                            let lock = states
                                .data_map
                                .get::<XdgToplevelSurfaceData>()
                                .expect("XdgToplevelSurfaceData doesn't exist")
                                .lock()
                                .expect("Couldn't lock XdgToplevelSurfaceData");
                            (i, lock.app_id.clone().unwrap_or("".to_string()))
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        tracing::debug!("windows are: {wins:?}");

        swap_window_positions(space, elem1, elem2);
    }

    fn layout(&self, space: &Space<Window>, output: &Output) {
        let Some(master) = self.master() else {
            return;
        };

        let Some(output_geo) = space.output_geometry(output) else {
            tracing::error!("could not get output geometry");
            return;
        };
        let wins = self
            .inner
            .iter()
            .map(|vec| {
                vec.iter()
                    .enumerate()
                    .map(|(i, win)| {
                        compositor::with_states(win.toplevel().wl_surface(), |states| {
                            let lock = states
                                .data_map
                                .get::<XdgToplevelSurfaceData>()
                                .expect("XdgToplevelSurfaceData doesn't exist")
                                .lock()
                                .expect("Couldn't lock XdgToplevelSurfaceData");
                            (i, lock.app_id.clone().unwrap_or("".to_string()))
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        tracing::debug!("windows are: {wins:?}");

        if self.stack().count() == 0 {
            // one window
            master.toplevel().with_pending_state(|state| {
                state.size = Some(output_geo.size);
            });

            WindowState::with(master, |state| {
                state.resize_state = WindowResizeState::WaitingForAck(
                    master.toplevel().send_configure(),
                    (0, 0).into(),
                );
            });
        } else {
            let new_master_size: Size<i32, Logical> =
                (output_geo.size.w / 2, output_geo.size.h).into();
            master.toplevel().with_pending_state(|state| {
                state.size = Some(new_master_size);
            });
            WindowState::with(master, |state| {
                state.resize_state = WindowResizeState::WaitingForAck(
                    master.toplevel().send_configure(),
                    (0, 0).into(),
                );
            });

            self.layout_stack(space, output);
        }
    }

    /// Chain another tag's windows to this one to be layed out.
    fn chain_with(mut self, vec: &'a mut Vec<Window>) -> Self {
        self.inner.push(vec);
        self
    }
}

pub trait LayoutVec<S: SpaceElement> {
    /// Interpret this vec as a master-stack layout.
    fn as_master_stack(&mut self) -> MasterStack<S>;
    // fn as_binary_tree(&mut self); TODO:
}

impl<S: SpaceElement> LayoutVec<S> for Vec<S> {
    fn as_master_stack(&mut self) -> MasterStack<S> {
        MasterStack { inner: vec![self] }
    }
}
