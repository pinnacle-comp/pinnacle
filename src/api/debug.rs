use super::StateFnSender;

mod v1;

pub struct DebugService {
    sender: StateFnSender,
}

impl DebugService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}
