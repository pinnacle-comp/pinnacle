use std::{fmt::Display, sync::atomic::AtomicU32};

use crate::signal::Signaler;

/// A building block providing common widget functionality.
///
/// This enables widgets to uniquely identify themselves and
/// connect and emit signals.
#[derive(Debug)]
pub struct WidgetBase {
    widget_type: String,
    id: u32,
    signaler: Signaler,
}

static COUNT: AtomicU32 = AtomicU32::new(0);

fn next_id() -> u32 {
    COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

impl WidgetBase {
    /// Creates a new widget base with a given type.
    pub fn new(widget_type: impl Into<String>) -> Self {
        Self {
            widget_type: widget_type.into(),
            id: next_id(),
            signaler: Signaler::default(),
        }
    }

    /// Returns this widget base's id.
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Returns a clone of this widget base's [`Signaler`].
    pub fn signaler(&self) -> Signaler {
        self.signaler.clone()
    }
}

impl Display for WidgetBase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{}#{}>", self.widget_type, self.id)
    }
}
