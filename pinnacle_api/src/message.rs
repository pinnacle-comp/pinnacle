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
    // Action(Action),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Action {
    CloseWindow { client_id: Option<u32> },
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Modifiers {
    Shift,
    Ctrl,
    Alt,
    Super,
}
