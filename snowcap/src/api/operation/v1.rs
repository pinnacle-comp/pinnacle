//! Widget operations

use iced_runtime::core::widget;
use snowcap_api_defs::snowcap::operation;

use crate::util::convert::FromApi;

impl FromApi<operation::v1::Operation> for Box<dyn widget::Operation + 'static> {
    fn from_api(api_type: operation::v1::Operation) -> Self {
        // FIXME remove expect
        api_type
            .target
            .map(FromApi::from_api)
            .expect("Operations should have a target")
    }
}

impl FromApi<operation::v1::operation::Target> for Box<dyn widget::Operation + 'static> {
    fn from_api(api_type: operation::v1::operation::Target) -> Self {
        use operation::v1::operation::Target;

        match api_type {
            Target::Focusable(focusable) => FromApi::from_api(focusable),
            Target::TextInput(text_input) => FromApi::from_api(text_input),
        }
    }
}

impl FromApi<operation::v1::Focusable> for Box<dyn widget::Operation + 'static> {
    fn from_api(api_type: operation::v1::Focusable) -> Self {
        // FIXME remove expect
        api_type
            .op
            .map(FromApi::from_api)
            .expect("Focusable should have an operation")
    }
}

impl FromApi<operation::v1::focusable::Op> for Box<dyn widget::Operation + 'static> {
    fn from_api(api_type: operation::v1::focusable::Op) -> Self {
        use operation::v1::focusable::{self, Op};

        match api_type {
            Op::Focus(focusable::Focus { id }) => {
                Box::new(widget::operation::focusable::focus(widget::Id::new(id)))
            }
            Op::Unfocus(_) => Box::new(widget::operation::focusable::unfocus()),
            Op::FocusNext(_) => Box::new(widget::operation::focusable::focus_next()),
            Op::FocusPrev(_) => Box::new(widget::operation::focusable::focus_previous()),
        }
    }
}

impl FromApi<operation::v1::TextInput> for Box<dyn widget::Operation + 'static> {
    fn from_api(api_type: operation::v1::TextInput) -> Self {
        // FIXME remove expect
        api_type
            .op
            .map(FromApi::from_api)
            .expect("TextInput should have an operation")
    }
}

impl FromApi<operation::v1::text_input::Op> for Box<dyn widget::Operation + 'static> {
    fn from_api(api_type: operation::v1::text_input::Op) -> Self {
        use operation::v1::text_input::{self, Op};

        match api_type {
            Op::MoveCursor(text_input::MoveCursor { id, position }) => {
                Box::new(widget::operation::text_input::move_cursor_to(
                    widget::Id::new(id),
                    position as usize,
                ))
            }
            Op::MoveCursorFront(text_input::MoveCursorFront { id }) => Box::new(
                widget::operation::text_input::move_cursor_to_front(widget::Id::new(id)),
            ),
            Op::MoveCursorEnd(text_input::MoveCursorEnd { id }) => Box::new(
                widget::operation::text_input::move_cursor_to_end(widget::Id::new(id)),
            ),
            Op::SelectAll(text_input::SelectAll { id }) => Box::new(
                widget::operation::text_input::select_all(widget::Id::new(id)),
            ),
        }
    }
}
