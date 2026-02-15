use crate::signal::Signal;

/// Notifies that a redraw is needed.
#[derive(Signal, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RedrawNeeded;

/// Request the surface to close itself.
#[derive(Signal, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestClose;

/// Emits a message that will update widgets.
#[derive(Signal, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Message<Msg>(pub Msg);

impl<Msg> Message<Msg> {
    /// Creates a new [`Message`].
    pub fn new(msg: Msg) -> Self {
        Self(msg)
    }

    /// Unwraps this message.
    pub fn into_inner(self) -> Msg {
        self.0
    }
}

impl<Msg> From<Msg> for Message<Msg> {
    fn from(value: Msg) -> Self {
        Self::new(value)
    }
}

/// Notifies that a widget closed.
#[derive(Signal, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Closed;
