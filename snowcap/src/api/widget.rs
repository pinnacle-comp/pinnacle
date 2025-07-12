pub mod v0alpha1;
pub mod v1;

use super::StateFnSender;

#[derive(Clone)]
pub struct WidgetService {
    sender: StateFnSender,
}

impl WidgetService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}
