// The MessagePack format for these is a one-element map where the element's key is the enum name and its
// value is a map of the enum's values

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Msg {
    SetKeybind {
        key: u32,
        modifiers: Vec<Modifiers>,
        callback_id: u32,
    },
    SetMousebind {
        button: u8,
    },
    CloseWindow {
        client_id: Option<u32>,
    },
    ToggleFloating {
        client_id: Option<u32>,
    },
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Modifiers {
    Shift = 0b0000_0001,
    Ctrl = 0b0000_0010,
    Alt = 0b0000_0100,
    Super = 0b0000_1000,
}

/// A bitmask of [Modifiers] for the purpose of hashing.
#[derive(PartialEq, Eq, Hash)]
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

/// Messages sent from the server to the client.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum OutgoingMsg {
    CallCallback(u32),
}
