mod v1;

use super::StateFnSender;

#[derive(Clone)]
pub struct PopupService {
    sender: StateFnSender,
}

impl PopupService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}
