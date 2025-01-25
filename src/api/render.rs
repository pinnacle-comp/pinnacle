mod v1;

use super::StateFnSender;

pub struct RenderService {
    sender: StateFnSender,
}

impl RenderService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}
