// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// The MessagePack format for these is a one-element map where the element's key is the enum name and its
// value is a map of the enum's values

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Copy)]
pub struct CallbackId(pub u32);

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Msg {
    // Input
    SetKeybind {
        key: u32,
        modifiers: Vec<Modifiers>,
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

    // Process management
    /// Spawn a program with an optional callback.
    Spawn {
        command: Vec<String>,
        #[serde(default)]
        callback_id: Option<CallbackId>,
    },

    /// Run a command using the optionally specified shell and callback.
    SpawnShell {
        shell: Option<String>,
        command: Vec<String>,
        #[serde(default)]
        callback_id: Option<CallbackId>,
    },

    // Pinnacle management
    /// Quit the compositor.
    Quit,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub enum Modifiers {
    Shift = 0b0000_0001,
    Ctrl = 0b0000_0010,
    Alt = 0b0000_0100,
    Super = 0b0000_1000,
}

/// A bitmask of [Modifiers] for the purpose of hashing.
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub struct ModifierMask(u8);

impl<T: IntoIterator<Item = Modifiers>> From<T> for ModifierMask {
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
    pub fn values(self) -> Vec<Modifiers> {
        let mut res = Vec::<Modifiers>::new();
        if self.0 & Modifiers::Shift as u8 == Modifiers::Shift as u8 {
            res.push(Modifiers::Shift);
        }
        if self.0 & Modifiers::Ctrl as u8 == Modifiers::Ctrl as u8 {
            res.push(Modifiers::Ctrl);
        }
        if self.0 & Modifiers::Alt as u8 == Modifiers::Alt as u8 {
            res.push(Modifiers::Alt);
        }
        if self.0 & Modifiers::Super as u8 == Modifiers::Super as u8 {
            res.push(Modifiers::Super);
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
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Args {
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
}
