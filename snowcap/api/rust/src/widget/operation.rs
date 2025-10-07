//! Widget operation
//!
//! Update internal state for some widgets.

use snowcap_api_defs::snowcap::operation;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Operation {
    Focusable(focusable::Focusable),
    TextInput(text_input::TextInput),
}

pub mod focusable {
    use super::Operation;
    use snowcap_api_defs::snowcap::operation::v1;

    #[derive(Debug, Clone, PartialEq)]
    #[non_exhaustive]
    pub enum Focusable {
        Focus(String),
        Unfocus,
        FocusNext,
        FocusPrev,
    }

    pub fn focus(widget_id: impl Into<String>) -> Operation {
        Focusable::Focus(widget_id.into()).into()
    }

    pub fn unfocus() -> Operation {
        Focusable::Unfocus.into()
    }

    pub fn focus_next() -> Operation {
        Focusable::FocusNext.into()
    }

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

pub mod text_input {
    use snowcap_api_defs::snowcap::operation::v1;

    use super::Operation;

    #[derive(Debug, Clone, PartialEq)]
    #[non_exhaustive]
    pub enum TextInput {
        MoveCursor { id: String, position: usize },
        MoveCursorFront(String),
        MoveCursorEnd(String),
        SelectAll(String),
    }

    pub fn move_cursor(widget_id: impl Into<String>, position: usize) -> Operation {
        TextInput::MoveCursor {
            id: widget_id.into(),
            position,
        }
        .into()
    }

    pub fn move_cursor_front(widget_id: impl Into<String>) -> Operation {
        TextInput::MoveCursorFront(widget_id.into()).into()
    }

    pub fn move_cursor_end(widget_id: impl Into<String>) -> Operation {
        TextInput::MoveCursorEnd(widget_id.into()).into()
    }

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
