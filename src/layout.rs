// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use smithay::{
    desktop::{space::SpaceElement, Space, Window},
    output::Output,
    utils::{Logical, Size},
};

use crate::{
    backend::Backend,
    state::{State, WithState},
    tag::TagId,
    window::window_state::WindowResizeState,
};

pub enum Direction {
    Left,
    Right,
    Top,
    Bottom,
}

pub trait Layout<S: SpaceElement> {
    fn layout(&self, space: &Space<S>, output: &Output);
}

pub struct MasterStack<S: SpaceElement> {
    inner: Vec<S>,
}

impl MasterStack<Window> {
    pub fn master(&self) -> Option<&Window> {
        self.inner.first()
    }

    pub fn stack(&self) -> impl Iterator<Item = &Window> {
        self.inner.iter().skip(1)
    }

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

            win.with_state(|state| {
                state.resize_state = WindowResizeState::WaitingForAck(
                    win.toplevel().send_configure(),
                    (output_geo.size.w / 2, i as i32 * height).into(),
                );
            });
        }
    }
}

impl Layout<Window> for MasterStack<Window> {
    fn layout(&self, space: &Space<Window>, output: &Output) {
        let Some(master) = self.master() else {
            return;
        };

        let Some(output_geo) = space.output_geometry(output) else {
            tracing::error!("could not get output geometry");
            return;
        };

        if self.stack().count() == 0 {
            // one window
            master.toplevel().with_pending_state(|state| {
                state.size = Some(output_geo.size);
            });

            master.with_state(|state| {
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
            master.with_state(|state| {
                state.resize_state = WindowResizeState::WaitingForAck(
                    master.toplevel().send_configure(),
                    (0, 0).into(),
                );
            });

            self.layout_stack(space, output);
        }
    }
}

pub struct Dwindle<S: SpaceElement> {
    inner: Vec<S>,
}

impl Layout<Window> for Dwindle<Window> {
    fn layout(&self, space: &Space<Window>, output: &Output) {
        todo!()
    }
}

pub trait LayoutVec<S: SpaceElement> {
    /// Interpret this vec as a master-stack layout.
    fn to_master_stack(&self, tags: Vec<TagId>) -> MasterStack<S>;
    fn to_dwindle(&self, tags: Vec<TagId>) -> Dwindle<S>;
}

impl LayoutVec<Window> for Vec<Window> {
    fn to_master_stack(&self, tags: Vec<TagId>) -> MasterStack<Window> {
        MasterStack {
            inner: filter_windows(self, tags),
        }
    }

    fn to_dwindle(&self, tags: Vec<TagId>) -> Dwindle<Window> {
        Dwindle {
            inner: filter_windows(self, tags),
        }
    }
}

fn filter_windows(windows: &[Window], tags: Vec<TagId>) -> Vec<Window> {
    windows
        .iter()
        .filter(|window| {
            window.with_state(|state| {
                state.floating.is_tiled() && {
                    for tag_id in state.tags.iter() {
                        if tags.iter().any(|tag| tag == tag_id) {
                            return true;
                        }
                    }
                    false
                }
            })
        })
        .cloned()
        .collect()
}

impl<B: Backend> State<B> {
    pub fn swap_window_positions(&mut self, win1: &Window, win2: &Window) {
        // FIXME: moving the mouse quickly will break swapping

        let win1_loc = self.space.element_location(win1).unwrap(); // TODO: handle unwraps
        let win2_loc = self.space.element_location(win2).unwrap();
        let win1_geo = win1.geometry();
        let win2_geo = win2.geometry();

        win1.toplevel().with_pending_state(|state| {
            state.size = Some(win2_geo.size);
        });
        win2.toplevel().with_pending_state(|state| {
            state.size = Some(win1_geo.size);
        });

        let serial = win1.toplevel().send_configure();
        win1.with_state(|state| {
            state.resize_state = WindowResizeState::WaitingForAck(serial, win2_loc);
        });

        let serial = win2.toplevel().send_configure();
        win2.with_state(|state| {
            state.resize_state = WindowResizeState::WaitingForAck(serial, win1_loc);
        });

        let mut elems = self
            .windows
            .iter_mut()
            .filter(|win| *win == win1 || *win == win2);

        let (first, second) = (elems.next(), elems.next());

        if let Some(first) = first {
            if let Some(second) = second {
                std::mem::swap(first, second);
            }
        }
    }
}
