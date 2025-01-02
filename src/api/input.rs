use super::StateFnSender;

mod v1;

pub struct InputService {
    sender: StateFnSender,
}

impl InputService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}
