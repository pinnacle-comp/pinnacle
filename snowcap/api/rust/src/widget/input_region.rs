use snowcap_api_defs::snowcap::widget;

use crate::widget::{Length, Widget, WidgetDef};

#[derive(Debug, PartialEq, Clone)]
pub struct InputRegion<Msg> {
    pub add: bool,
    pub width: Option<Length>,
    pub height: Option<Length>,
    pub child: WidgetDef<Msg>,
}

impl<Msg> InputRegion<Msg> {
    pub fn new(add: bool, child: impl Into<WidgetDef<Msg>>) -> Self {
        Self {
            add,
            child: child.into(),
            width: None,
            height: None,
        }
    }

    pub fn width(self, width: Length) -> Self {
        Self {
            width: Some(width),
            ..self
        }
    }

    pub fn height(self, height: Length) -> Self {
        Self {
            height: Some(height),
            ..self
        }
    }
}

impl<Msg> From<InputRegion<Msg>> for widget::v1::InputRegion {
    fn from(value: InputRegion<Msg>) -> Self {
        Self {
            add: value.add,
            width: value.width.map(From::from),
            height: value.height.map(From::from),
            child: Some(Box::new(value.child.into())),
        }
    }
}

impl<Msg> From<InputRegion<Msg>> for Widget<Msg> {
    fn from(value: InputRegion<Msg>) -> Self {
        Self::InputRegion(Box::new(value))
    }
}
