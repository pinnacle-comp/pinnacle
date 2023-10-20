//! Functions for compositor control, like `setup` and `quit`.

use crate::{msg::Msg, send_msg};

/// Quit Pinnacle.
pub fn quit() {
    send_msg(Msg::Quit).unwrap();
}
