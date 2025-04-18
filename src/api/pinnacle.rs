mod v1;

use super::StateFnSender;

pub struct PinnacleService {
    sender: StateFnSender,
}

impl PinnacleService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}
