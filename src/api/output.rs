use super::StateFnSender;

mod v1;

pub struct OutputService {
    sender: StateFnSender,
}

impl OutputService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}
