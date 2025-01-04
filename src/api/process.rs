use super::StateFnSender;

mod v1;

pub struct ProcessService {
    sender: StateFnSender,
}

impl ProcessService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}
