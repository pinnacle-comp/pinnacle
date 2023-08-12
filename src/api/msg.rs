// SPDX-License-Identifier: GPL-3.0-or-later

// The MessagePack format for these is a one-element map where the element's key is the enum name and its
// value is a map of the enum's values

use crate::{
    layout::Layout,
    output::OutputName,
    tag::TagId,
    window::window_state::{StatusName, WindowId},
};

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
        window_id: WindowId,
    },
    ToggleFloating {
        window_id: WindowId,
    },
    SetWindowSize {
        window_id: WindowId,
        #[serde(default)]
        width: Option<i32>,
        #[serde(default)]
        height: Option<i32>,
    },
    MoveWindowToTag {
        window_id: WindowId,
        tag_id: TagId,
    },
    ToggleTagOnWindow {
        window_id: WindowId,
        tag_id: TagId,
    },
    SetStatus {
        window_id: WindowId,
        status: StatusName,
    },

    // Tag management
    ToggleTag {
        tag_id: TagId,
    },
    SwitchToTag {
        tag_id: TagId,
    },
    AddTags {
        /// The name of the output you want these tags on.
        output_name: String,
        tag_names: Vec<String>,
    },
    RemoveTags {
        /// The name of the output you want these tags removed from.
        tag_ids: Vec<TagId>,
    },
    SetLayout {
        tag_id: TagId,
        layout: Layout,
    },

    // Output management
    ConnectForAllOutputs {
        callback_id: CallbackId,
    },
    SetOutputLocation {
        output_name: OutputName,
        #[serde(default)]
        x: Option<i32>,
        #[serde(default)]
        y: Option<i32>,
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

    Request {
        request_id: RequestId,
        request: Request,
    },
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RequestId(u32);

#[allow(clippy::enum_variant_names)]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
/// Messages that require a server response, usually to provide some data.
pub enum Request {
    // Windows
    GetWindows,
    GetWindowProps { window_id: WindowId },
    // Outputs
    GetOutputs,
    GetOutputProps { output_name: String },
    // Tags
    GetTags,
    GetTagProps { tag_id: TagId },
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub enum Modifier {
    Shift = 0b0000_0001,
    Ctrl = 0b0000_0010,
    Alt = 0b0000_0100,
    Super = 0b0000_1000,
}

/// A bitmask of [`Modifier`]s for the purpose of hashing.
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
    #[allow(dead_code)]
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
        request_id: RequestId,
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
    Window {
        window_id: Option<WindowId>,
    },
    Windows {
        window_ids: Vec<WindowId>,
    },
    WindowProps {
        size: Option<(i32, i32)>,
        loc: Option<(i32, i32)>,
        class: Option<String>,
        title: Option<String>,
        focused: Option<bool>,
        status: Option<StatusName>,
    },
    Output {
        output_name: Option<String>,
    },
    Outputs {
        output_names: Vec<String>,
    },
    OutputProps {
        /// The make of the output.
        make: Option<String>,
        /// The model of the output.
        model: Option<String>,
        /// The location of the output in the space.
        loc: Option<(i32, i32)>,
        /// The resolution of the output.
        res: Option<(i32, i32)>,
        /// The refresh rate of the output.
        refresh_rate: Option<i32>,
        /// The size of the output, in millimeters.
        physical_size: Option<(i32, i32)>,
        /// Whether the output is focused or not.
        focused: Option<bool>,
        tag_ids: Option<Vec<TagId>>,
    },
    Tags {
        tag_ids: Vec<TagId>,
    },
    TagProps {
        active: Option<bool>,
        name: Option<String>,
        output_name: Option<String>,
    },
}
