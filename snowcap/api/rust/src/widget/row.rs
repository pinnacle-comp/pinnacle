use snowcap_api_defs::snowcap::widget;

use super::{Alignment, Length, Padding, WidgetDef};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Row {
    pub spacing: Option<f32>,
    pub padding: Option<Padding>,
    pub item_alignment: Option<Alignment>,
    pub width: Option<Length>,
    pub height: Option<Length>,
    pub clip: Option<bool>,
    pub children: Vec<WidgetDef>,
}

impl Row {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_children(children: impl IntoIterator<Item = WidgetDef>) -> Self {
        Self {
            children: children.into_iter().collect(),
            ..Default::default()
        }
    }

    pub fn spacing(self, spacing: f32) -> Self {
        Self {
            spacing: Some(spacing),
            ..self
        }
    }

    pub fn item_alignment(self, item_alignment: Alignment) -> Self {
        Self {
            item_alignment: Some(item_alignment),
            ..self
        }
    }

    pub fn padding(self, padding: Padding) -> Self {
        Self {
            padding: Some(padding),
            ..self
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

    pub fn clip(self, clip: bool) -> Self {
        Self {
            clip: Some(clip),
            ..self
        }
    }

    pub fn push(mut self, child: impl Into<WidgetDef>) -> Self {
        self.children.push(child.into());
        self
    }
}

impl From<Row> for widget::v1::Row {
    fn from(value: Row) -> Self {
        widget::v1::Row {
            spacing: value.spacing,
            padding: value.padding.map(From::from),
            item_alignment: value
                .item_alignment
                .map(|it| widget::v1::Alignment::from(it) as i32),
            width: value.width.map(From::from),
            height: value.height.map(From::from),
            clip: value.clip,
            children: value.children.into_iter().map(From::from).collect(),
        }
    }
}
