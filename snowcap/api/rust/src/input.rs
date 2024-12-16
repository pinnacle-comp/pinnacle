//! Input types.

use snowcap_api_defs::snowcap::input;

/// Keyboard modifiers.
#[allow(missing_docs)]
#[derive(Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub logo: bool,
}

impl From<input::v0alpha1::Modifiers> for Modifiers {
    fn from(value: input::v0alpha1::Modifiers) -> Self {
        Self {
            shift: value.shift(),
            ctrl: value.ctrl(),
            alt: value.alt(),
            logo: value.super_(),
        }
    }
}
