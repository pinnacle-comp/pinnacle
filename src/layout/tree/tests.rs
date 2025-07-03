use std::collections::HashMap;

use proptest::{
    prelude::{Strategy, any},
    prop_compose, proptest,
};
use rand::seq::IndexedRandom;
use smithay::utils::{Logical, Size};

use crate::layout::tree::{LayoutTree, ResizeDir};

use super::LayoutNode;

prop_compose! {
    fn arbitrary_style()(
        margin_left in -50.0f32..50.0,
        margin_right in -50.0f32..=50.0,
        margin_top in -50.0f32..=50.0,
        margin_bottom in -50.0f32..=50.0,
        flex_basis in 0.0f32..10.0,
        row in any::<bool>(),
    ) -> taffy::Style {
        taffy::Style {
            margin: taffy::Rect {
                left: taffy::LengthPercentageAuto::length(margin_left),
                right: taffy::LengthPercentageAuto::length(margin_right),
                top: taffy::LengthPercentageAuto::length(margin_top),
                bottom: taffy::LengthPercentageAuto::length(margin_bottom)
            },
            flex_grow: 1.0,
            flex_basis: taffy::Dimension::percent(flex_basis),
            flex_direction: match row {
                true => taffy::FlexDirection::Row,
                false => taffy::FlexDirection::Column,
            },
            ..Default::default()
        }
    }
}

fn arbitrary_traversal_overrides() -> impl Strategy<Value = HashMap<u32, Vec<u32>>> {
    proptest::collection::hash_map(
        0u32..=20,
        proptest::collection::vec(0u32..=20, 0..=20),
        0..=20,
    )
}

prop_compose! {
    fn arbitrary_single_layout_node(children: Vec<LayoutNode>)(
        label in proptest::option::of(proptest::string::string_regex("[0-9]").unwrap()),
        traversal_index in any::<u32>(),
        traversal_overrides in arbitrary_traversal_overrides(),
        style in arbitrary_style(),
    ) -> LayoutNode {
        LayoutNode {
            label,
            traversal_index,
            traversal_overrides,
            style,
            children: children.clone(),
        }
    }
}

fn arbitrary_layout_node() -> impl Strategy<Value = LayoutNode> {
    let leaf = arbitrary_single_layout_node(Vec::new());
    leaf.prop_recursive(8, 128, 8, |inner| {
        proptest::collection::vec(inner, 0..=8).prop_flat_map(arbitrary_single_layout_node)
    })
}

fn arbitrary_size() -> impl Strategy<Value = Size<i32, Logical>> {
    (1i32..10000, 1i32..10000).prop_map(|(w, h)| Size::from((w, h)))
}

proptest! {
    #[test]
    fn resize_tile_does_not_panic(
        root_node in arbitrary_layout_node(),
        w in 1u32..=100000,
        h in 1u32..=100000,
        resize_x_dir: ResizeDir,
        resize_y_dir: ResizeDir,
        new_size in arbitrary_size(),
    ) {
        let mut tree = LayoutTree::new(root_node);
        let geos_and_nodes = tree.compute_geos(w, h);

        let &(_, resize_node) = geos_and_nodes.choose(&mut rand::rng()).unwrap();

        tree.resize_tile(resize_node, new_size, resize_x_dir, resize_y_dir);

        let _ = tree.compute_geos(w, h);
    }
}
