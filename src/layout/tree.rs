#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use smithay::utils::{Logical, Rectangle, Size};
use tracing::trace;

use crate::util::treediff::{
    EditAction,
    diffable::{Diffable, StyleDiff},
};

pub const MIN_TILE_SIZE: f32 = 50.0;

#[derive(PartialEq, Clone)]
pub struct LayoutNode {
    pub label: Option<String>,
    pub traversal_index: u32,
    pub traversal_overrides: HashMap<u32, Vec<u32>>,
    pub style: taffy::Style,
    pub children: Vec<LayoutNode>,
}

impl std::fmt::Debug for LayoutNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayoutNode")
            .field("label", &self.label)
            .field("traversal_index", &self.traversal_index)
            .field("traversal_overrides", &self.traversal_overrides)
            .field("style.flex_basis", &self.style.flex_basis)
            .field("children", &self.children)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct LayoutTree {
    taffy_tree: taffy::TaffyTree<NodeContext>,
    root: LayoutNode,
    taffy_root_id: taffy::NodeId,
    neighbor_info: HashMap<taffy::NodeId, NeighborInfo>,
}

impl PartialEq for LayoutTree {
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root
            && self.taffy_root_id == other.taffy_root_id
            && self.neighbor_info == other.neighbor_info
            && taffy_node_partial_eq(
                &self.taffy_tree,
                &other.taffy_tree,
                self.taffy_root_id,
                other.taffy_root_id,
            )
    }
}

fn taffy_node_partial_eq(
    this: &taffy::TaffyTree<NodeContext>,
    other: &taffy::TaffyTree<NodeContext>,
    this_node: taffy::NodeId,
    other_node: taffy::NodeId,
) -> bool {
    if this.style(this_node) != other.style(other_node) {
        return false;
    }

    if this.get_node_context(this_node) != other.get_node_context(other_node) {
        return false;
    }

    let this_children = this.children(this_node).unwrap();
    let other_children = other.children(other_node).unwrap();

    if this_children.len() != other_children.len() {
        return false;
    }

    this_children
        .into_iter()
        .zip(other_children)
        .all(|(this_node, other_node)| taffy_node_partial_eq(this, other, this_node, other_node))
}

#[derive(Debug, Clone, Default, PartialEq)]
struct NodeContext {
    traversal_index: u32,
    traversal_overrides: HashMap<u32, Vec<u32>>,
    original_flex_basis: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct NeighborInfo {
    has_immediate_row_neighbor_ahead: bool,
    has_immediate_row_neighbor_behind: bool,
    has_immediate_col_neighbor_ahead: bool,
    has_immediate_col_neighbor_behind: bool,
    row_neighbors_ahead: u32,
    row_neighbors_behind: u32,
    col_neighbors_ahead: u32,
    col_neighbors_behind: u32,
}

impl NeighborInfo {
    fn has_immediate_neighbor(&self, layout_dir: LayoutDir, resize_dir: ResizeDir) -> bool {
        match (layout_dir, resize_dir) {
            (LayoutDir::Row, ResizeDir::Ahead) => self.has_immediate_row_neighbor_ahead,
            (LayoutDir::Row, ResizeDir::Behind) => self.has_immediate_row_neighbor_behind,
            (LayoutDir::Col, ResizeDir::Ahead) => self.has_immediate_col_neighbor_ahead,
            (LayoutDir::Col, ResizeDir::Behind) => self.has_immediate_col_neighbor_behind,
        }
    }

    fn neighbors(&self, layout_dir: LayoutDir, resize_dir: ResizeDir) -> u32 {
        match (layout_dir, resize_dir) {
            (LayoutDir::Row, ResizeDir::Ahead) => self.row_neighbors_ahead,
            (LayoutDir::Row, ResizeDir::Behind) => self.row_neighbors_behind,
            (LayoutDir::Col, ResizeDir::Ahead) => self.col_neighbors_ahead,
            (LayoutDir::Col, ResizeDir::Behind) => self.col_neighbors_behind,
        }
    }
}

impl LayoutTree {
    fn build_node(tree: &mut taffy::TaffyTree<NodeContext>, node: LayoutNode) -> taffy::NodeId {
        let children = node
            .children
            .into_iter()
            .map(|child| Self::build_node(tree, child))
            .collect::<Vec<_>>();

        let original_flex_basis = node.style.flex_basis.value();
        let root_id = tree.new_with_children(node.style, &children).unwrap();
        tree.set_node_context(
            root_id,
            Some(NodeContext {
                traversal_index: node.traversal_index,
                traversal_overrides: node.traversal_overrides,
                original_flex_basis,
            }),
        )
        .unwrap();

        root_id
    }

    fn process_leaves(tree: &mut taffy::TaffyTree<NodeContext>, node: taffy::NodeId) {
        let children = tree.children(node).unwrap();

        for child in children.iter() {
            Self::process_leaves(tree, *child);
        }

        if children.is_empty() {
            let mut new_node_style = tree.style(node).unwrap().clone();
            let prev_margin = new_node_style.margin;
            new_node_style.margin = taffy::Rect::length(0.0);
            tree.set_style(node, new_node_style).unwrap();

            let leaf_child = tree
                .new_leaf_with_context(
                    taffy::Style {
                        margin: prev_margin,
                        flex_basis: taffy::Dimension::percent(1.0),
                        ..Default::default()
                    },
                    NodeContext::default(),
                )
                .unwrap();
            tree.set_children(node, &[leaf_child]).unwrap();
        }
    }

    fn unprocess_leaves(tree: &mut taffy::TaffyTree<NodeContext>, node: taffy::NodeId) {
        let children = tree.children(node).unwrap();

        for child in children.iter() {
            Self::unprocess_leaves(tree, *child);
        }

        if children.is_empty() {
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
        let mut tree = taffy::TaffyTree::new();
        let fake_root = Self::build_node(&mut tree, root.clone());
        Self::process_leaves(&mut tree, fake_root);

        let actual_root = tree
            .new_with_children(
                taffy::Style {
                    size: taffy::Size {
                        width: taffy::Dimension::percent(1.0),
                        height: taffy::Dimension::percent(1.0),
                    },
                    ..Default::default()
                },
                &[fake_root],
            )
            .unwrap();

        let mut layout_tree = Self {
            taffy_tree: tree,
            root,
            taffy_root_id: actual_root,
            neighbor_info: HashMap::new(),
        };

        layout_tree.update_neighbor_info();

        layout_tree
    }

    pub fn compute_geos(
        &mut self,
        width: u32,
        height: u32,
    ) -> Vec<(Rectangle<i32, Logical>, taffy::NodeId)> {
        fn compute_geos_rec(
            geos: &mut Vec<(Rectangle<i32, Logical>, taffy::NodeId)>,
            tree: &taffy::TaffyTree<NodeContext>,
            node: taffy::NodeId,
            offset_x: f64,
            offset_y: f64,
            traversal_overrides: &[u32],
            node_assigned: &mut HashSet<taffy::NodeId>,
            counters: &mut HashMap<taffy::NodeId, u32>,
        ) -> bool {
            let geo = tree.layout(node).unwrap();
            let mut loc = geo.location.map(|loc| loc as f64);
            loc.x += offset_x;
            loc.y += offset_y;

            let mut children = tree.children(node).unwrap();

            if children.is_empty() {
                if node_assigned.contains(&node) {
                    return false;
                }
                node_assigned.insert(node);

                let size = geo.size.map(|size| size as f64);

                let rect: Rectangle<i32, Logical> = Rectangle {
                    loc: smithay::utils::Point::from((loc.x, loc.y)),
                    size: smithay::utils::Size::from((size.width, size.height)),
                }
                .to_i32_round();
                geos.push((rect, tree.parent(node).unwrap()));

                *counters.entry(node).or_default() += 1;

                return true;
            }

            children.sort_by(|a, b| {
                let traversal_index_a = tree.get_node_context(*a).unwrap().traversal_index;
                let traversal_index_b = tree.get_node_context(*b).unwrap().traversal_index;
                traversal_index_a.cmp(&traversal_index_b)
            });

            let traversal_overrides = tree
                .get_node_context(node)
                .and_then(|context| {
                    context
                        .traversal_overrides
                        .get(counters.entry(node).or_default())
                })
                .filter(|overrides| !overrides.is_empty())
                .map_or(traversal_overrides, |v| v);

            let (traversal_index, traversal_split) = match traversal_overrides.split_first() {
                Some((idx, rest)) => (Some(*idx), Some(rest)),
                None => (None, None),
            };

            if let Some(override_index) = traversal_index {
                if children.get(override_index as usize).is_some() {
                    let child = children.remove(override_index as usize);
                    children.insert(0, child);
                }
            }

            for child in children.into_iter() {
                let assigned = compute_geos_rec(
                    geos,
                    tree,
                    child,
                    loc.x,
                    loc.y,
                    traversal_split.unwrap_or_default(),
                    node_assigned,
                    counters,
                );
                if assigned {
                    *counters.entry(node).or_default() += 1;
                    return true;
                }
            }

            false
        }

        if self.taffy_tree.dirty(self.taffy_root_id).unwrap() {
            self.taffy_tree
                .compute_layout(
                    self.taffy_root_id,
                    taffy::Size {
                        width: taffy::AvailableSpace::Definite(width as f32),
                        height: taffy::AvailableSpace::Definite(height as f32),
                    },
                )
                .unwrap();
        }

        let mut geos = Vec::new();

        let mut node_assigned = HashSet::<taffy::NodeId>::new();
        let mut counters = HashMap::<taffy::NodeId, u32>::new();
        loop {
            let mut traversal_overrides = vec![0u32];
            if let Some(overrides) = self
                .root
                .traversal_overrides
                .get(counters.entry(self.taffy_root_id).or_default())
            {
                traversal_overrides.extend(overrides);
            }

            if !compute_geos_rec(
                &mut geos,
                &self.taffy_tree,
                self.taffy_root_id,
                0.0,
                0.0,
                &traversal_overrides,
                &mut node_assigned,
                &mut counters,
            ) {
                break;
            }

            *counters.get_mut(&self.taffy_root_id).unwrap() += 1;
        }

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

                if a.label == b.label { 1.0 } else { f64::MAX }
            },
            |a, b| a.data().label == b.data().label,
        );

        let taffy_root_id = self.taffy_root_id;
        let node_id_for_path = |tree: &taffy::TaffyTree<NodeContext>, path: &[usize]| {
            let mut current_node = taffy_root_id;
            for index in path.iter().copied() {
                current_node = tree.children(current_node).unwrap()[index];
            }

            current_node
        };

        for action in edit_script {
            match action {
                EditAction::Insert { val, dst, idx } => {
                    trace!(?dst, ?idx, "Layout insert");
                    let parent = node_id_for_path(&self.taffy_tree, &dst);
                    let original_flex_basis = val.style.flex_basis.value();
                    let child = self
                        .taffy_tree
                        .new_leaf_with_context(
                            val.style,
                            NodeContext {
                                traversal_index: val.traversal_index,
                                traversal_overrides: val.traversal_overrides,
                                original_flex_basis,
                            },
                        )
                        .unwrap();
                    self.taffy_tree
                        .insert_child_at_index(parent, idx, child)
                        .unwrap();
                }
                EditAction::Delete(path) => {
                    trace!(?path, "Layout delete");
                    let to_remove = node_id_for_path(&self.taffy_tree, &path);
                    let parent = self.taffy_tree.parent(to_remove).unwrap();
                    let original_flex_basis = self
                        .taffy_tree
                        .get_node_context(to_remove)
                        .unwrap()
                        .original_flex_basis;
                    let target_sum = self
                        .taffy_tree
                        .children(parent)
                        .unwrap()
                        .into_iter()
                        .map(|child| self.taffy_tree.style(child).unwrap().flex_basis.value())
                        .sum::<f32>()
                        - original_flex_basis;

                    self.taffy_tree.remove(to_remove).unwrap();

                    let children = self.taffy_tree.children(parent).unwrap();

                    let old_basises = children
                        .iter()
                        .map(|child| self.taffy_tree.style(*child).unwrap().flex_basis.value())
                        .collect::<Vec<_>>();

                    let new_basises = rescale_flex_basises(&old_basises, target_sum);

                    for (child, basis) in children.into_iter().zip(new_basises) {
                        let mut style = self.taffy_tree.style(child).unwrap().clone();
                        style.flex_basis = taffy::Dimension::percent(basis);
                        self.taffy_tree.set_style(child, style).unwrap();
                    }
                }
                EditAction::Update(path, val) => {
                    trace!(?path, "Layout update");
                    let to_update = node_id_for_path(&self.taffy_tree, &path);

                    let LayoutNodeDataDiff {
                        traversal_index,
                        traversal_overrides,
                        style:
                            StyleDiff {
                                flex_direction,
                                flex_basis,
                                margin,
                            },
                    } = val;

                    let mut style = self.taffy_tree.style(to_update).unwrap().clone();

                    if let Some(flex_direction) = flex_direction {
                        style.flex_direction = flex_direction;
                    }
                    if let Some(flex_basis) = flex_basis {
                        style.flex_basis = flex_basis;
                    }
                    if let Some(margin) = margin {
                        style.margin = margin;
                    }
                    self.taffy_tree.set_style(to_update, style).unwrap();

                    let node_context = self.taffy_tree.get_node_context_mut(to_update).unwrap();
                    if let Some(traversal_index) = traversal_index {
                        node_context.traversal_index = traversal_index;
                    }
                    if let Some(traversal_overrides) = traversal_overrides {
                        node_context.traversal_overrides = traversal_overrides;
                    }
                }
                EditAction::Move { src, dst, idx } => {
                    trace!(?src, ?dst, ?idx, "Layout move");
                    let to_move = node_id_for_path(&self.taffy_tree, &src);
                    let dst_parent_node = node_id_for_path(&self.taffy_tree, &dst);

                    let parent_of_to_move = self.taffy_tree.parent(to_move).unwrap();
                    let old_idx = self
                        .taffy_tree
                        .children(parent_of_to_move)
                        .unwrap()
                        .iter()
                        .position(|sib| *sib == to_move)
                        .unwrap();

                    self.taffy_tree
                        .remove_child_at_index(parent_of_to_move, old_idx)
                        .unwrap();

                    self.taffy_tree
                        .insert_child_at_index(dst_parent_node, idx, to_move)
                        .unwrap();
                }
            }
        }

        Self::process_leaves(&mut self.taffy_tree, self.taffy_root_id);

        self.root = new_root;

        self.update_neighbor_info();
    }

    fn update_neighbor_info(&mut self) {
        /// Returns (row_nodes_under_node, col_nodes_under_node).
        fn update_rec(
            tree: &taffy::TaffyTree<NodeContext>,
            node: taffy::NodeId,
            ahead: bool,
            neighbors_row: u32,
            neighbors_col: u32,
            has_immediate_row_neighbor: bool,
            has_immediate_col_neighbor: bool,
            ret: &mut HashMap<taffy::NodeId, NeighborInfo>,
        ) -> (u32, u32) {
            let entry = ret.entry(node).or_default();
            if ahead {
                entry.row_neighbors_ahead = neighbors_row;
                entry.col_neighbors_ahead = neighbors_col;
                entry.has_immediate_row_neighbor_ahead = has_immediate_row_neighbor;
                entry.has_immediate_col_neighbor_ahead = has_immediate_col_neighbor;
            } else {
                entry.row_neighbors_behind = neighbors_row;
                entry.col_neighbors_behind = neighbors_col;
                entry.has_immediate_row_neighbor_behind = has_immediate_row_neighbor;
                entry.has_immediate_col_neighbor_behind = has_immediate_col_neighbor;
            }

            let children = tree.children(node).unwrap().into_iter();
            let flex_direction = tree.style(node).unwrap().flex_direction;
            let is_row = match flex_direction {
                taffy::FlexDirection::Row | taffy::FlexDirection::RowReverse => true,
                taffy::FlexDirection::Column | taffy::FlexDirection::ColumnReverse => false,
            };
            let children = match ahead {
                true => itertools::Either::Left(children.rev().with_position()),
                false => itertools::Either::Right(children.with_position()),
            };

            let mut row_nodes_under_this_node = neighbors_row;
            let mut col_nodes_under_this_node = neighbors_col;

            for (pos, child) in children {
                let has_immediate_neighbor = match pos {
                    itertools::Position::First | itertools::Position::Only => false,
                    itertools::Position::Middle | itertools::Position::Last => true,
                };
                let has_immediate_row_neighbor = is_row && has_immediate_neighbor;
                let has_immediate_col_neighbor = !is_row && has_immediate_neighbor;
                let (row_nodes_under_child, col_nodes_under_child) = update_rec(
                    tree,
                    child,
                    ahead,
                    if is_row {
                        row_nodes_under_this_node
                    } else {
                        neighbors_row
                    },
                    if !is_row {
                        col_nodes_under_this_node
                    } else {
                        neighbors_col
                    },
                    has_immediate_row_neighbor,
                    has_immediate_col_neighbor,
                    ret,
                );
                row_nodes_under_this_node = u32::max(
                    row_nodes_under_this_node + (is_row && pos != itertools::Position::Only) as u32,
                    row_nodes_under_child,
                );
                col_nodes_under_this_node = u32::max(
                    col_nodes_under_this_node
                        + (!is_row && pos != itertools::Position::Only) as u32,
                    col_nodes_under_child,
                );
            }

            (row_nodes_under_this_node, col_nodes_under_this_node)
        }

        let mut ret = HashMap::new();

        update_rec(
            &self.taffy_tree,
            self.taffy_root_id,
            true,
            0,
            0,
            false,
            false,
            &mut ret,
        );

        update_rec(
            &self.taffy_tree,
            self.taffy_root_id,
            false,
            0,
            0,
            false,
            false,
            &mut ret,
        );

        self.neighbor_info = ret;
    }

    /// Returns the amount of available space in the direction for a node with no
    /// immediate siblings but that *does* have a neighboring node.
    fn available_space_in_direction(
        &self,
        node: taffy::NodeId,
        layout_dir: LayoutDir,
        resize_dir: ResizeDir,
    ) -> f32 {
        let mut current = node;
        while let Some(parent) = self.taffy_tree.parent(current) {
            let sibling_direction = self.taffy_tree.style(parent).unwrap().flex_direction;
            let in_same_layout_dir = match sibling_direction {
                taffy::FlexDirection::Row | taffy::FlexDirection::RowReverse => {
                    layout_dir == LayoutDir::Row
                }
                taffy::FlexDirection::Column | taffy::FlexDirection::ColumnReverse => {
                    layout_dir == LayoutDir::Col
                }
            };
            if !in_same_layout_dir {
                current = parent;
            } else {
                let current_neighbor_info = self.neighbor_info.get(&current).unwrap();

                if !current_neighbor_info.has_immediate_neighbor(layout_dir, resize_dir) {
                    current = parent;
                } else {
                    break;
                }
            }
        }

        let parent = self.taffy_tree.parent(current).expect("ALALAL");

        let siblings = self.taffy_tree.children(parent).unwrap();
        let node_idx = siblings.iter().position(|n| *n == current).unwrap();

        let to_resize = if resize_dir == ResizeDir::Ahead {
            siblings.get(node_idx + 1..).unwrap_or_default()
        } else {
            &siblings[..node_idx]
        };

        let total_size = to_resize
            .iter()
            .map(|node| self.taffy_tree.unrounded_layout(*node).size)
            .map(|size| match layout_dir {
                LayoutDir::Row => size.width,
                LayoutDir::Col => size.height,
            })
            .sum::<f32>()
            .round();
        let len = to_resize.len() as u32;

        (total_size - MIN_TILE_SIZE * len as f32).max(0.0)
    }

    /// Walks the layout tree upward to resize a tile in a given direction.
    fn resize_tile_in_direction(
        &mut self,
        node: taffy::NodeId,
        old_size: f32,
        mut new_size: i32,
        layout_dir: LayoutDir,
        resize_dir: ResizeDir,
    ) {
        new_size = new_size.max(MIN_TILE_SIZE as i32);
        let mut delta = new_size as f32 - old_size;

        let mut current = node;
        while let Some(parent) = self.taffy_tree.parent(current) {
            let sibling_direction = self.taffy_tree.style(parent).unwrap().flex_direction;
            let in_same_layout_dir = match sibling_direction {
                taffy::FlexDirection::Row | taffy::FlexDirection::RowReverse => {
                    layout_dir == LayoutDir::Row
                }
                taffy::FlexDirection::Column | taffy::FlexDirection::ColumnReverse => {
                    layout_dir == LayoutDir::Col
                }
            };
            if !in_same_layout_dir {
                // Walk the tree upward to try to find neighboring nodes to resize
                current = parent;
            } else {
                let current_neighbor_info = self.neighbor_info.get(&current).unwrap();

                let siblings = self.taffy_tree.children(parent).unwrap();
                let node_idx = siblings.iter().position(|n| *n == current).unwrap();
                if !current_neighbor_info.has_immediate_neighbor(layout_dir, resize_dir) {
                    // If there's no immediate neighbor (i.e. siblings with the same parent in the
                    // given direction), we need to resize the layout nodes under the current
                    // parent in the opposite direction to keep them in the same spot when
                    // resizing ancestor nodes.
                    //
                    // However, we don't need to perform that resizing if there are no
                    // neighbors *at all* (i.e. the tile is at the edge of the screen).
                    if current_neighbor_info.neighbors(layout_dir, resize_dir) > 0 {
                        let to_resize_opposing = if resize_dir == ResizeDir::Ahead {
                            &siblings[..=node_idx]
                        } else {
                            &siblings[node_idx..]
                        };

                        let mut geos = to_resize_opposing
                            .iter()
                            .map(|node| self.taffy_tree.layout(*node).unwrap().size)
                            .map(|size| match layout_dir {
                                LayoutDir::Row => size.width,
                                LayoutDir::Col => size.height,
                            })
                            .collect::<Vec<_>>();

                        let mut total_size: f32 = geos.iter().sum();

                        // There are possibly non-sibling nodes neighboring this node in the resize
                        // direction. We need to know how much space we have to resize them or else
                        // this algorithm causes nodes in the *opposite* direction to shrink if we
                        // resize "too far" (i.e. the nodes in the direction can't shrink anymore).
                        //
                        // Yes, this is not very performant as it walks up the tree again, but
                        // we need that size information *now*.
                        let available_space =
                            self.available_space_in_direction(current, layout_dir, resize_dir);

                        // Clamp the new size if we "over-resize"
                        if delta > available_space {
                            new_size -= (delta - available_space) as i32;
                            delta = available_space;
                        }

                        // Calculate the new total size of the parent node
                        // after resizing for use in ancestor nodes
                        total_size += new_size as f32
                            - match resize_dir {
                                ResizeDir::Ahead => geos[node_idx],
                                ResizeDir::Behind => geos[0],
                            };

                        // Update the size of the current node.
                        // Later on we'll shrink neighboring nodes in the resize direction
                        // to keep this node's opposite edge in the same spot.
                        if resize_dir == ResizeDir::Ahead {
                            geos[node_idx] = new_size as f32;
                        } else {
                            geos[0] = new_size as f32;
                        }

                        new_size = total_size.round() as i32;

                        let basises_sum = to_resize_opposing
                            .iter()
                            .map(|node| self.taffy_tree.style(*node).unwrap().flex_basis.value())
                            .sum();

                        let new_basises = calculate_flex_basises(&geos, basises_sum);

                        for (&node, new_basis) in to_resize_opposing.iter().zip(new_basises) {
                            let mut style = self.taffy_tree.style(node).unwrap().clone();
                            style.flex_basis = taffy::Dimension::percent(new_basis);
                            self.taffy_tree.set_style(node, style).unwrap();
                        }
                    }

                    current = parent;
                } else {
                    break;
                }
            }
        }

        // See below docs
        self.resize_final(current, new_size, delta, layout_dir, resize_dir);
    }

    /// Performs a final resize on the top-most node that needs resizing
    /// in the given direction.
    ///
    /// This is a modification of the above resizing logic that additionally
    /// subtracts the `delta_from_original_size` from all nodes in the
    /// opposite direction to keep the other edge of the original node
    /// in the same spot.
    fn resize_final(
        &mut self,
        node: taffy::NodeId,
        new_size: i32,
        delta_from_original_size: f32,
        layout_dir: LayoutDir,
        resize_dir: ResizeDir,
    ) {
        let Some(parent) = self.taffy_tree.parent(node) else {
            return;
        };

        let siblings = self.taffy_tree.children(parent).unwrap();
        let node_idx = siblings.iter().position(|n| *n == node).unwrap();

        let to_resize = if resize_dir == ResizeDir::Ahead {
            &siblings[node_idx..]
        } else {
            &siblings[..=node_idx]
        };

        let mut geos = to_resize
            .iter()
            .map(|node| self.taffy_tree.unrounded_layout(*node).size)
            .map(|size| match layout_dir {
                LayoutDir::Row => size.width,
                LayoutDir::Col => size.height,
            })
            .collect::<Vec<_>>();

        let num_to_shrink = geos.len() - 1;

        if resize_dir == ResizeDir::Ahead {
            // Shrink nodes in the same direction to keep the edge opposing
            // the resize in the same spot, making sure no tile shrinks
            // below MIN_TILE_SIZE.

            let mut shrink = |delta: f32| -> f32 {
                let mut could_not_shrink_by = 0.0;
                for geo in &mut geos[1..] {
                    *geo -= delta / num_to_shrink as f32;
                    if *geo < MIN_TILE_SIZE {
                        could_not_shrink_by += MIN_TILE_SIZE - *geo;
                        *geo = MIN_TILE_SIZE;
                    }
                }
                could_not_shrink_by
            };

            let mut left_to_shrink = shrink(delta_from_original_size);

            while left_to_shrink > 0.0 {
                let old = left_to_shrink;
                left_to_shrink = shrink(left_to_shrink);
                if (old - left_to_shrink).abs() <= 0.001 {
                    break;
                }
            }

            geos[0] = new_size as f32 - left_to_shrink;
        } else {
            // Same as above but in the other direction

            let mut shrink = |delta: f32| -> f32 {
                let mut could_not_shrink_by = 0.0;
                for geo in &mut geos[..node_idx] {
                    *geo -= delta / num_to_shrink as f32;
                    if *geo < MIN_TILE_SIZE {
                        could_not_shrink_by += MIN_TILE_SIZE - *geo;
                        *geo = MIN_TILE_SIZE;
                    }
                }
                could_not_shrink_by
            };

            let mut left_to_shrink = shrink(delta_from_original_size);

            while left_to_shrink > 0.0 {
                let old = left_to_shrink;
                left_to_shrink = shrink(left_to_shrink);
                if (old - left_to_shrink).abs() <= 0.001 {
                    break;
                }
            }

            geos[node_idx] = new_size as f32 - left_to_shrink;
        }

        let basises_sum = to_resize
            .iter()
            .map(|node| self.taffy_tree.style(*node).unwrap().flex_basis.value())
            .sum();

        let new_basises = calculate_flex_basises(&geos, basises_sum);

        for (&node, new_basis) in to_resize.iter().zip(new_basises) {
            let mut style = self.taffy_tree.style(node).unwrap().clone();
            style.flex_basis = taffy::Dimension::percent(new_basis);
            self.taffy_tree.set_style(node, style).unwrap();
        }
    }

    pub fn resize_tile(
        &mut self,
        node: taffy::NodeId,
        mut new_size: Size<i32, Logical>,
        resize_x_dir: ResizeDir,
        resize_y_dir: ResizeDir,
    ) {
        // Add the margins of the tile the window is in to get the
        // actual new size
        let child = self.taffy_tree.children(node).unwrap().remove(0);
        let margin = self.taffy_tree.style(child).unwrap().margin;
        let horiz_margin =
            margin.left.into_raw().value() as i32 + margin.right.into_raw().value() as i32;
        let vert_margin =
            margin.top.into_raw().value() as i32 + margin.bottom.into_raw().value() as i32;
        new_size.w += horiz_margin;
        new_size.h += vert_margin;

        let old_width = self.taffy_tree.layout(node).unwrap().size.width;
        let old_height = self.taffy_tree.layout(node).unwrap().size.height;

        self.resize_tile_in_direction(node, old_width, new_size.w, LayoutDir::Row, resize_x_dir);

        self.resize_tile_in_direction(node, old_height, new_size.h, LayoutDir::Col, resize_y_dir);
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum LayoutDir {
    Row,
    Col,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum ResizeDir {
    Ahead,
    Behind,
}

/// Calculates new flex basises for the given lengths.
///
/// The sum of the new basises will equal `basises_sum`.
fn calculate_flex_basises(new_lengths: &[f32], basises_sum: f32) -> Vec<f32> {
    let new_sum = new_lengths.iter().sum::<f32>();

    let new_proportions = new_lengths
        .iter()
        .map(|len| *len / new_sum)
        .collect::<Vec<_>>();

    let new_props_sum = new_proportions.iter().sum::<f32>();

    let scale_amt = basises_sum / new_props_sum;

    new_proportions
        .into_iter()
        .map(|prop| prop * scale_amt)
        .collect()
}

/// Rescales flex basises so that they sum up to `target_sum`.
fn rescale_flex_basises(basises: &[f32], target_sum: f32) -> Vec<f32> {
    let basises_sum = basises.iter().sum::<f32>();

    let scale_by = target_sum / basises_sum;

    basises.iter().map(|basis| *basis * scale_by).collect()
}

#[derive(Default, Clone, PartialEq)]
struct LayoutNodeData {
    label: Option<String>,
    traversal_index: u32,
    traversal_overrides: HashMap<u32, Vec<u32>>,
    style: taffy::Style,
}

struct LayoutNodeDataDiff {
    traversal_index: Option<u32>,
    traversal_overrides: Option<HashMap<u32, Vec<u32>>>,
    style: <taffy::Style as Diffable>::Output,
}

impl Diffable for LayoutNodeData {
    type Output = LayoutNodeDataDiff;

    fn diff(&self, newer: &Self) -> Self::Output {
        let style_diff = self.style.diff(&newer.style);

        LayoutNodeDataDiff {
            traversal_index: (self.traversal_index != newer.traversal_index)
                .then_some(newer.traversal_index),
            traversal_overrides: (self.traversal_overrides != newer.traversal_overrides)
                .then_some(newer.traversal_overrides.clone()),
            style: style_diff,
        }
    }
}

impl std::fmt::Debug for LayoutNodeData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayoutNodeData")
            .field("label", &self.label)
            .field("traversal_index", &self.traversal_index)
            .field("traversal_overrides", &self.traversal_overrides)
            .field("style", &"...")
            .field("style.margin", &self.style.margin)
            .field("style.flex_direction", &self.style.flex_direction)
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
            traversal_overrides: self.traversal_overrides.clone(),
            style: self.style.clone(),
        };

        *slab_node.data() = data;

        for child in self.children.iter() {
            child.process_node(slab_node.append(LayoutNodeData::default()));
        }
    }
}
