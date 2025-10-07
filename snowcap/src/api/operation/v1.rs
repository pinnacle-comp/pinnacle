//! Widget operations

use anyhow::Context;
use iced_runtime::core::widget;
use snowcap_api_defs::snowcap::operation;

use crate::util::convert::{FromApi, TryFromApi};

impl TryFromApi<operation::v1::Operation> for Box<dyn widget::Operation + 'static> {
    type Error = anyhow::Error;

    fn try_from_api(api_type: operation::v1::Operation) -> Result<Self, Self::Error> {
        const MESSAGE: &str = "snowcap.operation.v1.Operation";
        const FIELD: &str = "target";

        let Some(target) = api_type.target else {
            anyhow::bail!("While converting {MESSAGE}: missing field '{FIELD}")
        };

        TryFromApi::try_from_api(target)
            .with_context(|| format!("While converting {MESSAGE}.{FIELD}"))
    }
}

impl TryFromApi<operation::v1::operation::Target> for Box<dyn widget::Operation + 'static> {
    type Error = anyhow::Error;

    fn try_from_api(api_type: operation::v1::operation::Target) -> Result<Self, Self::Error> {
        use operation::v1::operation::Target;

        match api_type {
            Target::Focusable(focusable) => TryFromApi::try_from_api(focusable),
            Target::TextInput(text_input) => TryFromApi::try_from_api(text_input),
        }
    }
}

impl TryFromApi<operation::v1::Focusable> for Box<dyn widget::Operation + 'static> {
    type Error = anyhow::Error;

    fn try_from_api(api_type: operation::v1::Focusable) -> Result<Self, Self::Error> {
        const MESSAGE: &str = "snowcap.operation.v1.Focusable";

        let Some(op) = api_type.op else {
            anyhow::bail!("While converting {MESSAGE}: missing field 'op'")
        };

        Ok(FromApi::from_api(op))
    }
}

impl FromApi<operation::v1::focusable::Op> for Box<dyn widget::Operation + 'static> {
    fn from_api(api_type: operation::v1::focusable::Op) -> Self {
        use operation::v1::focusable::{self, Op};

        match api_type {
            Op::Focus(focusable::Focus { id }) => {
                Box::new(widget::operation::focusable::focus(id.into()))
            }
            Op::Unfocus(_) => Box::new(widget::operation::focusable::unfocus()),
            Op::FocusNext(_) => Box::new(widget::operation::focusable::focus_next()),
            Op::FocusPrev(_) => Box::new(widget::operation::focusable::focus_previous()),
        }
    }
}

impl TryFromApi<operation::v1::TextInput> for Box<dyn widget::Operation + 'static> {
    type Error = anyhow::Error;

    fn try_from_api(api_type: operation::v1::TextInput) -> Result<Self, Self::Error> {
        const MESSAGE: &str = "snowcap.operation.v1.TextInput";

        let Some(op) = api_type.op else {
            anyhow::bail!("While converting {MESSAGE}: missing field 'op");
        };

        Ok(FromApi::from_api(op))
    }
}

impl FromApi<operation::v1::text_input::Op> for Box<dyn widget::Operation + 'static> {
    fn from_api(api_type: operation::v1::text_input::Op) -> Self {
        use operation::v1::text_input::{self, Op};

        match api_type {
            Op::MoveCursor(text_input::MoveCursor { id, position }) => Box::new(
                widget::operation::text_input::move_cursor_to(id.into(), position as usize),
            ),
            Op::MoveCursorFront(text_input::MoveCursorFront { id }) => Box::new(
                widget::operation::text_input::move_cursor_to_front(id.into()),
            ),
            Op::MoveCursorEnd(text_input::MoveCursorEnd { id }) => {
                Box::new(widget::operation::text_input::move_cursor_to_end(id.into()))
            }
            Op::SelectAll(text_input::SelectAll { id }) => {
                Box::new(widget::operation::text_input::select_all(id.into()))
            }
        }
    }
}
