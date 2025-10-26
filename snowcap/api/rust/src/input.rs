//! Input types.

use snowcap_api_defs::snowcap::input;
use xkbcommon::xkb::Keysym;

/// Keyboard modifiers.
#[allow(missing_docs)]
#[derive(Default, Debug, Clone, Copy)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub logo: bool,
}

impl From<input::v1::Modifiers> for Modifiers {
    fn from(value: input::v1::Modifiers) -> Self {
        Self {
            shift: value.shift,
            ctrl: value.ctrl,
            alt: value.alt,
            logo: value.super_,
        }
    }
}

/// A Key event.
#[derive(Debug, Clone)]
pub struct KeyEvent {
    /// Key Symbol.
    pub key: Keysym,
    /// Currently active modifiers.
    pub mods: Modifiers,
    /// True if the key is currently pressed, false on release.
    pub pressed: bool,
    /// True if the event was flagged as Captured by a widget.
    pub captured: bool,
    /// Text produced by the event, if any.
    pub text: Option<String>,
}

impl From<input::v1::KeyboardKeyResponse> for KeyEvent {
    fn from(value: input::v1::KeyboardKeyResponse) -> Self {
        Self {
            key: Keysym::new(value.key),
            mods: Modifiers::from(value.modifiers.unwrap_or_default()),
            pressed: value.pressed,
            captured: value.captured,
            text: value.text,
        }
    }
}
