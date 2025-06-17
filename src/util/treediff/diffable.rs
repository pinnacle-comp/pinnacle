/// Diffable structs.
pub trait Diffable {
    /// The output of diffing two structs.
    type Output;

    /// Diffs `self` against `newer`.
    fn diff(&self, newer: &Self) -> Self::Output;
}

pub struct StyleDiff {
    pub flex_direction: Option<taffy::FlexDirection>,
    pub flex_basis: Option<taffy::Dimension>,
    pub margin: Option<taffy::Rect<taffy::LengthPercentageAuto>>,
}

impl Diffable for taffy::Style {
    type Output = StyleDiff;

    fn diff(&self, newer: &Self) -> Self::Output {
        StyleDiff {
            flex_direction: (self.flex_direction != newer.flex_direction)
                .then_some(newer.flex_direction),
            flex_basis: (self.flex_basis != newer.flex_basis).then_some(newer.flex_basis),
            margin: (self.margin != newer.margin).then_some(newer.margin),
        }
    }
}
