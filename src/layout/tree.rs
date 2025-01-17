use smithay::utils::{Logical, Rectangle};

use crate::util::treediff::EditAction;

#[derive(PartialEq, Clone)]
pub struct LayoutNode {
    pub label: Option<String>,
    pub traversal_index: u32,
    pub style: taffy::Style,
    pub children: Vec<LayoutNode>,
}

impl std::fmt::Debug for LayoutNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayoutNode")
            .field("label", &self.label)
            .field("traversal_index", &self.traversal_index)
            .field("style", &"...")
            .field("children", &self.children)
            .finish()
    }
}

#[derive(Debug)]
pub struct LayoutTree {
    pub(super) taffy_tree: taffy::TaffyTree<u32>,
    pub(super) root: LayoutNode,
    pub(super) taffy_root_id: taffy::NodeId,
}

impl LayoutTree {
    fn build_node(tree: &mut taffy::TaffyTree<u32>, node: LayoutNode) -> taffy::NodeId {
        let children = node
            .children
            .into_iter()
            .map(|child| Self::build_node(tree, child))
            .collect::<Vec<_>>();

        let root_id = tree.new_with_children(node.style, &children).unwrap();
        tree.set_node_context(root_id, Some(node.traversal_index))
            .unwrap();

        root_id
    }

    fn process_leaves(tree: &mut taffy::TaffyTree<u32>, node: taffy::NodeId) {
        let children = tree.children(node).unwrap();

        for child in children.iter() {
            Self::process_leaves(tree, *child);
        }

        let has_children = !children.is_empty();
        if !has_children {
            let mut new_node_style = tree.style(node).unwrap().clone();
            let prev_margin = new_node_style.margin;
            new_node_style.margin = taffy::Rect::length(0.0);
            tree.set_style(node, new_node_style).unwrap();

            let leaf_child = tree
                .new_leaf_with_context(
                    taffy::Style {
                        margin: prev_margin,
                        flex_basis: taffy::Dimension::Percent(1.0),
                        ..Default::default()
                    },
                    0,
                )
                .unwrap();
            tree.set_children(node, &[leaf_child]).unwrap();
        }
    }

    fn unprocess_leaves(tree: &mut taffy::TaffyTree<u32>, node: taffy::NodeId) {
        let children = tree.children(node).unwrap();

        for child in children.iter() {
            Self::unprocess_leaves(tree, *child);
        }

        let has_children = !children.is_empty();
        if !has_children {
            let parent = tree.parent(node).unwrap();

            debug_assert_eq!(tree.children(parent).unwrap().len(), 1);

            let margin = tree.style(node).unwrap().margin;

            let mut parent_style = tree.style(parent).unwrap().clone();
            parent_style.margin = margin;

            tree.set_style(parent, parent_style).unwrap();

            tree.remove(node).unwrap();
        }
    }

    pub fn new(root: LayoutNode) -> Self {
        let tree = taffy::TaffyTree::<u32>::new();

        Self::new_with_data(root, tree)
    }

    fn new_with_data(root: LayoutNode, mut tree: taffy::TaffyTree<u32>) -> Self {
        let fake_root = Self::build_node(&mut tree, root.clone());
        Self::process_leaves(&mut tree, fake_root);

        let actual_root = tree
            .new_with_children(
                taffy::Style {
                    size: taffy::Size {
                        width: taffy::Dimension::Percent(1.0),
                        height: taffy::Dimension::Percent(1.0),
                    },
                    ..Default::default()
                },
                &[fake_root],
            )
            .unwrap();

        Self {
            taffy_tree: tree,
            root,
            taffy_root_id: actual_root,
        }
    }

    pub fn compute_geos(&mut self, width: u32, height: u32) -> Vec<Rectangle<i32, Logical>> {
        self.taffy_tree
            .compute_layout(
                self.taffy_root_id,
                taffy::Size {
                    width: taffy::AvailableSpace::Definite(width as f32),
                    height: taffy::AvailableSpace::Definite(height as f32),
                },
            )
            .unwrap();

        let mut geos = Vec::<Rectangle<i32, Logical>>::new();

        fn compute_geos_rec(
            geos: &mut Vec<Rectangle<i32, Logical>>,
            tree: &taffy::TaffyTree<u32>,
            node: taffy::NodeId,
            offset_x: f64,
            offset_y: f64,
        ) {
            let geo = tree.layout(node).unwrap();
            let mut loc = geo.location.map(|loc| loc as f64);
            loc.x += offset_x;
            loc.y += offset_y;
            let size = geo.size.map(|size| size as f64);

            let mut children = tree.children(node).unwrap();

            if children.is_empty() {
                let rect: Rectangle<i32, Logical> = Rectangle {
                    loc: smithay::utils::Point::from((loc.x, loc.y)),
                    size: smithay::utils::Size::from((size.width, size.height)),
                }
                .to_i32_round();
                geos.push(rect);
                return;
            }

            children.sort_by(|a, b| {
                let traversal_index_a = *tree.get_node_context(*a).unwrap();
                let traversal_index_b = *tree.get_node_context(*b).unwrap();
                traversal_index_a.cmp(&traversal_index_b)
            });

            for child in children.into_iter() {
                compute_geos_rec(geos, tree, child, loc.x, loc.y);
            }
        }

        compute_geos_rec(&mut geos, &self.taffy_tree, self.taffy_root_id, 0.0, 0.0);

        geos
    }

    pub fn diff(&mut self, new_root: LayoutNode) {
        Self::unprocess_leaves(&mut self.taffy_tree, self.taffy_root_id);

        let src_tree = self.root.to_slab_tree();
        let dst_tree = new_root.to_slab_tree();

        let edit_script = crate::util::treediff::diff(
            &src_tree,
            &dst_tree,
            |_| 1.0,
            |_| 1.0,
            |a, b| {
                let a = a.data();
                let b = b.data();

                if a != b {
                    1.0
                } else {
                    0.0
                }
            },
            |a, b| a.data().label == b.data().label,
        );

        let taffy_root_id = self.taffy_root_id;
        let node_id_for_path = |tree: &taffy::TaffyTree<u32>, path: &[usize]| {
            let mut current_node = taffy_root_id;
            for index in path.iter().copied() {
                current_node = tree.children(current_node).unwrap()[index];
            }

            current_node
        };

        for action in edit_script {
            match action {
                EditAction::Insert { val, dst, idx } => {
                    let parent = node_id_for_path(&self.taffy_tree, &dst);
                    let child = self
                        .taffy_tree
                        .new_leaf_with_context(val.style, val.traversal_index)
                        .unwrap();
                    self.taffy_tree
                        .insert_child_at_index(parent, idx, child)
                        .unwrap();
                }
                EditAction::Delete(path) => {
                    let to_remove = node_id_for_path(&self.taffy_tree, &path);
                    self.taffy_tree.remove(to_remove).unwrap();
                }
                EditAction::Update(path, val) => {
                    let to_update = node_id_for_path(&self.taffy_tree, &path);
                    self.taffy_tree.set_style(to_update, val.style).unwrap();
                    self.taffy_tree
                        .set_node_context(to_update, Some(val.traversal_index))
                        .unwrap();
                }
                EditAction::Move { src, dst, idx } => {
                    let to_move = node_id_for_path(&self.taffy_tree, &src);
                    let dst_parent_node = node_id_for_path(&self.taffy_tree, &dst);

                    self.taffy_tree
                        .insert_child_at_index(dst_parent_node, idx, to_move)
                        .unwrap();
                }
            }
        }

        Self::process_leaves(&mut self.taffy_tree, self.taffy_root_id);

        self.root = new_root;
    }
}

#[derive(Default, Clone, PartialEq)]
struct LayoutNodeData {
    label: Option<String>,
    traversal_index: u32,
    style: taffy::Style,
}

impl std::fmt::Debug for LayoutNodeData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayoutNodeData")
            .field("label", &self.label)
            .field("traversal_index", &self.traversal_index)
            .field("style", &"...")
            .finish()
    }
}

impl LayoutNode {
    fn to_slab_tree(&self) -> slab_tree::Tree<LayoutNodeData> {
        let mut tree = slab_tree::Tree::new();

        tree.set_root(LayoutNodeData::default());
        let root = tree.root_mut().unwrap();

        self.process_node(root);

        tree
    }

    fn process_node(&self, mut slab_node: slab_tree::NodeMut<'_, LayoutNodeData>) {
        let data = LayoutNodeData {
            label: self.label.clone(),
            traversal_index: self.traversal_index,
            style: self.style.clone(),
        };

        *slab_node.data() = data;

        for child in self.children.iter() {
            child.process_node(slab_node.append(LayoutNodeData::default()));
        }
    }
}
