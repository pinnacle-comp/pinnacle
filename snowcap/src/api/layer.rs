mod v0alpha1;
mod v1;

use super::StateFnSender;

#[derive(Clone)]
pub struct LayerService {
    sender: StateFnSender,
}

impl LayerService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}
