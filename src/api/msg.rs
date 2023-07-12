// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

// The MessagePack format for these is a one-element map where the element's key is the enum name and its
// value is a map of the enum's values

use crate::window::{window_state::WindowId, WindowProperties};

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Copy)]
pub struct CallbackId(pub u32);

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Msg {
    // Input
    SetKeybind {
        key: u32,
        modifiers: Vec<Modifier>,
        callback_id: CallbackId,
    },
    SetMousebind {
        button: u8,
    },

    // Window management
    CloseWindow {
        #[serde(default)]
        client_id: Option<u32>,
    },
    ToggleFloating {
        #[serde(default)]
        client_id: Option<u32>,
    },
    SetWindowSize {
        window_id: WindowId,
        size: (i32, i32),
    },
    MoveWindowToTag {
        window_id: WindowId,
        tag_id: String,
    },
    ToggleTagOnWindow {
        window_id: WindowId,
        tag_id: String,
    },

    // Tag management
    ToggleTag {
        output_name: String,
        tag_name: String,
    },
    SwitchToTag {
        output_name: String,
        tag_name: String,
    },
    AddTags {
        /// The name of the output you want these tags on.
        output_name: String,
        tags: Vec<String>,
    },
    RemoveTags {
        /// The name of the output you want these tags removed from.
        output_name: String,
        tags: Vec<String>,
    },

    // Output management
    ConnectForAllOutputs {
        callback_id: CallbackId,
    },

    // Process management
    /// Spawn a program with an optional callback.
    Spawn {
        command: Vec<String>,
        #[serde(default)]
        callback_id: Option<CallbackId>,
    },

    // Pinnacle management
    /// Quit the compositor.
    Quit,

    Request(Request),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RequestId(pub u32);

#[allow(clippy::enum_variant_names)]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
/// Messages that require a server response, usually to provide some data.
pub enum Request {
    GetWindowByAppId { app_id: String },
    GetWindowByTitle { title: String },
    GetWindowByFocus,
    GetAllWindows,
    GetOutputByName { name: String },
    GetOutputsByModel { model: String },
    GetOutputsByRes { res: (u32, u32) },
    GetOutputByFocus,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub enum Modifier {
    Shift = 0b0000_0001,
    Ctrl = 0b0000_0010,
    Alt = 0b0000_0100,
    Super = 0b0000_1000,
}

/// A bitmask of [Modifiers] for the purpose of hashing.
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub struct ModifierMask(u8);

impl<T: IntoIterator<Item = Modifier>> From<T> for ModifierMask {
    fn from(value: T) -> Self {
        let value = value.into_iter();
        let mut mask: u8 = 0b0000_0000;
        for modifier in value {
            mask |= modifier as u8;
        }
        Self(mask)
    }
}

impl ModifierMask {
    pub fn values(self) -> Vec<Modifier> {
        let mut res = Vec::<Modifier>::new();
        if self.0 & Modifier::Shift as u8 == Modifier::Shift as u8 {
            res.push(Modifier::Shift);
        }
        if self.0 & Modifier::Ctrl as u8 == Modifier::Ctrl as u8 {
            res.push(Modifier::Ctrl);
        }
        if self.0 & Modifier::Alt as u8 == Modifier::Alt as u8 {
            res.push(Modifier::Alt);
        }
        if self.0 & Modifier::Super as u8 == Modifier::Super as u8 {
            res.push(Modifier::Super);
        }
        res
    }
}

/// Messages sent from the server to the client.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum OutgoingMsg {
    CallCallback {
        callback_id: CallbackId,
        #[serde(default)]
        args: Option<Args>,
    },
    RequestResponse {
        response: RequestResponse,
    },
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Args {
    /// Send a message with lines from the spawned process.
    Spawn {
        #[serde(default)]
        stdout: Option<String>,
        #[serde(default)]
        stderr: Option<String>,
        #[serde(default)]
        exit_code: Option<i32>,
        #[serde(default)]
        exit_msg: Option<String>,
    },
    ConnectForAllOutputs {
        output_name: String,
    },
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum RequestResponse {
    Window { window: WindowProperties },
    GetAllWindows { windows: Vec<WindowProperties> },
    Outputs { names: Vec<String> },
}
