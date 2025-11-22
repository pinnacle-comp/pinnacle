//! Update internal state for some widgets.
//!
//! [`Operations`] can be passed to [`LayerHandle::operate`] and [`DecorationHandle::operate`] to
//! act on their widgets states.
//!
//! # Example
//! Focus a given widget:
//! ```no_run
//! use snowcap_api::layer::LayerHandle;
//! use snowcap_api::widget::operation;
//!
//! #[derive(Clone, Default)]
//! pub struct ProgramMsg {
//!     // [...]
//! };
//!
//! fn focus_widget(handle: LayerHandle<ProgramMsg>, widget_id: impl Into<String>) {
//!     handle.operate(operation::focusable::focus(widget_id));
//! }
//! ```
//!
//! Focus a widget and move the cursor to the beginning of the field:
//! ```no_run
//! use snowcap_api::layer::LayerHandle;
//! use snowcap_api::widget::operation;
//!
//! #[derive(Clone, Default)]
//! pub struct ProgramMsg {
//!     // [...]
//! };
//!
//! fn prepend_to_widget(handle: LayerHandle<ProgramMsg>, widget_id: impl Into<String>) {
//!     let widget_id = widget_id.into();
//!
//!     handle.operate(operation::focusable::focus(&widget_id));
//!     handle.operate(operation::text_input::move_cursor_front(&widget_id));
//! }
//! ```
//!
//! [`Operations`]: Operation
//! [`LayerHandle::operate`]: crate::layer::LayerHandle::operate
//! [`DecorationHandle::operate`]: crate::decoration::DecorationHandle::operate

use snowcap_api_defs::snowcap::operation;

/// Update widgets' internal state.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Operation {
    Focusable(focusable::Focusable),
    TextInput(text_input::TextInput),
}

/// Create [`Operations`] acting on widget that can be focused.
///
/// [`Operations`]: Operation
pub mod focusable {
    use super::Operation;
    use snowcap_api_defs::snowcap::operation::v1;

    /// [`Operation`] acting on widget that can be focused.
    #[derive(Debug, Clone, PartialEq)]
    #[non_exhaustive]
    pub enum Focusable {
        Focus(String),
        Unfocus,
        FocusNext,
        FocusPrev,
    }

    /// Creates an [`Operation`] to focus a specific widget.
    pub fn focus(widget_id: impl Into<String>) -> Operation {
        Focusable::Focus(widget_id.into()).into()
    }

    /// Creates an [`Operation`] to remove focus from any widgets.
    pub fn unfocus() -> Operation {
        Focusable::Unfocus.into()
    }

    /// Creates an [`Operation`] to focus the next widget in the tree, or the first one.
    pub fn focus_next() -> Operation {
        Focusable::FocusNext.into()
    }

    /// Creates an [`Operation`] to focus the previous widget in the tree, or the last one.
    pub fn focus_previous() -> Operation {
        Focusable::FocusPrev.into()
    }

    impl From<Focusable> for Operation {
        fn from(value: Focusable) -> Self {
            Self::Focusable(value)
        }
    }

    impl From<Focusable> for v1::Focusable {
        fn from(value: Focusable) -> Self {
            Self {
                op: Some(value.into()),
            }
        }
    }

    impl From<Focusable> for v1::focusable::Op {
        fn from(value: Focusable) -> Self {
            use v1::focusable::{self, Op};

            match value {
                Focusable::Focus(id) => Op::Focus(focusable::Focus { id }),
                Focusable::Unfocus => Op::Unfocus(focusable::Unfocus {}),
                Focusable::FocusNext => Op::FocusNext(focusable::FocusNext {}),
                Focusable::FocusPrev => Op::FocusPrev(focusable::FocusPrev {}),
            }
        }
    }
}

/// [`Operation`] acting on widget that have a text input.
pub mod text_input {
    use snowcap_api_defs::snowcap::operation::v1;

    use super::Operation;

    /// [`Operation`] acting on widget that have a text input.
    #[derive(Debug, Clone, PartialEq)]
    #[non_exhaustive]
    pub enum TextInput {
        MoveCursor { id: String, position: usize },
        MoveCursorFront(String),
        MoveCursorEnd(String),
        SelectAll(String),
    }

    /// Creates an [`Operation`] that set the position of the widget's cursor.
    pub fn move_cursor(widget_id: impl Into<String>, position: usize) -> Operation {
        TextInput::MoveCursor {
            id: widget_id.into(),
            position,
        }
        .into()
    }

    /// Creates an [`Operation`] that sets the widget's cursor to the beginning of the field.
    pub fn move_cursor_front(widget_id: impl Into<String>) -> Operation {
        TextInput::MoveCursorFront(widget_id.into()).into()
    }

    /// Creates an [`Operation`] that sets the widget's cursor to the end of the field.
    pub fn move_cursor_end(widget_id: impl Into<String>) -> Operation {
        TextInput::MoveCursorEnd(widget_id.into()).into()
    }

    /// Creates an [`Operation`] that select the widget's content.
    pub fn select_all(widget_id: impl Into<String>) -> Operation {
        TextInput::SelectAll(widget_id.into()).into()
    }

    impl From<TextInput> for Operation {
        fn from(value: TextInput) -> Self {
            Operation::TextInput(value)
        }
    }

    impl From<TextInput> for v1::TextInput {
        fn from(value: TextInput) -> Self {
            Self {
                op: Some(value.into()),
            }
        }
    }

    impl From<TextInput> for v1::text_input::Op {
        fn from(value: TextInput) -> Self {
            use v1::text_input::{self, Op};

            match value {
                TextInput::MoveCursor { id, position } => Op::MoveCursor(text_input::MoveCursor {
                    id,
                    position: position as u64,
                }),
                TextInput::MoveCursorFront(id) => {
                    Op::MoveCursorFront(text_input::MoveCursorFront { id })
                }
                TextInput::MoveCursorEnd(id) => Op::MoveCursorEnd(text_input::MoveCursorEnd { id }),
                TextInput::SelectAll(id) => Op::SelectAll(text_input::SelectAll { id }),
            }
        }
    }
}

impl From<Operation> for operation::v1::Operation {
    fn from(value: Operation) -> Self {
        Self {
            target: Some(value.into()),
        }
    }
}

impl From<Operation> for operation::v1::operation::Target {
    fn from(value: Operation) -> Self {
        use operation::v1::operation::Target;

        match value {
            Operation::Focusable(f) => Target::Focusable(f.into()),
            Operation::TextInput(t) => Target::TextInput(t.into()),
        }
    }
}
