mod v1;

use super::StateFnSender;

#[derive(Clone)]
pub struct DecorationService {
    sender: StateFnSender,
}

impl DecorationService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}
