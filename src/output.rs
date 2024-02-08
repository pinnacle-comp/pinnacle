// SPDX-License-Identifier: GPL-3.0-or-later

use std::{cell::RefCell, marker::PhantomData};

use smithay::output::Output;

use crate::{
    state::{State, WithState},
    tag::Tag,
};

/// A unique identifier for an output.
///
/// An empty string represents an invalid output.
// TODO: maybe encode that in the type
#[derive(Debug, Hash, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct OutputName(pub String);

impl OutputName {
    /// Get the output with this name.
    pub fn output(&self, state: &State) -> Option<Output> {
        state
            .space
            .outputs()
            .find(|output| output.name() == self.0)
            .cloned()
    }
}

/// The state of an output
#[derive(Default, Debug)]
pub struct OutputState {
    pub tags: Vec<Tag>,
}

impl WithState for Output {
    type State = OutputState;

    fn with_state<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&mut Self::State) -> T,
    {
        let state = self
            .user_data()
            .get_or_insert(RefCell::<Self::State>::default);

        func(&mut state.borrow_mut())
    }
}

impl OutputState {
    pub fn focused_tags(&self) -> impl Iterator<Item = &Tag> {
        self.tags.iter().filter(|tag| tag.active())
    }
}

trait OutputSignal {
    type Callback;
}

///////////////////////////////////////////

#[derive(Copy, Clone)]
struct OutputSignalConnect;

impl OutputSignal for OutputSignalConnect {
    type Callback = Box<dyn FnMut(Output)>;
}

#[derive(Copy, Clone)]
struct OutputSignalDisconnect;

impl OutputSignal for OutputSignalDisconnect {
    type Callback = Box<dyn FnMut(Output, i32)>;
}

// impl OutputSignalConnect {
//     fn do_something(&self) -> <OutputSignalConnect as OutputSignal>::Callback {
//         todo!()
//     }
// }

fn connect_signal<S: OutputSignal>(signal: S, callback: <S as OutputSignal>::Callback) {}

fn do_things() {
    connect_signal(OutputSignalConnect, Box::new(|output| {}));
    connect_signal(OutputSignalDisconnect, Box::new(|output, num| {}));
}

///////// ENUM

enum OpSignal {
    Connect(Box<dyn FnMut(Output)>),
    Disconnect(Box<dyn FnMut(Output, i32)>),
}

fn connect_signal_enum(signal: OpSignal) {}

fn do_things_enum() {
    connect_signal_enum(OpSignal::Connect(Box::new(|output| {})));
}
