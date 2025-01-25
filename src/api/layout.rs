use super::StateFnSender;

mod v1;

pub struct LayoutService {
    sender: StateFnSender,
}

impl LayoutService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}
