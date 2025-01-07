use std::collections::{hash_map::Entry, BTreeMap, HashMap};

use smithay::utils::{Logical, Rectangle};

#[derive(PartialEq, Clone, Debug)]
pub struct LayoutNode {
    pub style: taffy::Style,
    pub children: indexmap::IndexMap<u32, LayoutNode>,
}

impl treediff::Value for LayoutNode {
    type Key = u32;

    type Item = Self;

    fn items<'a>(&'a self) -> Option<Box<dyn Iterator<Item = (Self::Key, &'a Self::Item)> + 'a>> {
        if self.children.is_empty() {
            return None;
        }
        Some(Box::new(self.children.iter().map(|(id, node)| (*id, node))) as _)
    }
}

#[derive(Debug)]
pub struct LayoutTree {
    pub(super) tree: taffy::TaffyTree<u32>,
    pub(super) id_map: HashMap<u32, taffy::NodeId>,
    pub(super) root_id: u32,
    pub(super) root: LayoutNode,
    pub(super) taffy_root_id: taffy::NodeId,
    pub(super) inner_gaps: f32,
    pub(super) outer_gaps: f32,
}

impl LayoutTree {
    fn build_node(
        tree: &mut taffy::TaffyTree<u32>,
        node: LayoutNode,
        node_id: u32,
        id_map: &mut HashMap<u32, taffy::NodeId>,
        inner_gaps: f32,
    ) -> taffy::NodeId {
        let children = node
            .children
            .into_iter()
            .map(|(child_id, child)| Self::build_node(tree, child, child_id, id_map, inner_gaps))
            .collect::<Vec<_>>();

        let root_id = match id_map.entry(node_id) {
            Entry::Occupied(occupied_entry) => {
                let id = *occupied_entry.get();
                tree.set_style(id, node.style).unwrap();
                tree.set_children(id, &children).unwrap();
                tree.set_node_context(id, Some(node_id)).unwrap();
                id
            }
            Entry::Vacant(vacant_entry) => {
                let id = tree.new_with_children(node.style, &children).unwrap();
                tree.set_node_context(id, Some(node_id)).unwrap();
                vacant_entry.insert(id);
                id
            }
        };

        let has_children = !children.is_empty();
        if !has_children {
            let leaf_child = tree
                .new_leaf_with_context(
                    taffy::Style {
                        margin: taffy::Rect::length(inner_gaps),
                        flex_basis: taffy::Dimension::Percent(1.0),
                        ..Default::default()
                    },
                    node_id,
                )
                .unwrap();
            tree.set_children(root_id, &[leaf_child]).unwrap();
        }

        root_id
    }

    pub fn new(root: LayoutNode, root_id: u32, inner_gaps: f32, outer_gaps: f32) -> Self {
        let tree = taffy::TaffyTree::<u32>::new();
        let id_map = HashMap::new();

        Self::new_with_data(root, root_id, tree, id_map, inner_gaps, outer_gaps)
    }

    fn new_with_data(
        root: LayoutNode,
        root_id: u32,
        mut tree: taffy::TaffyTree<u32>,
        mut id_map: HashMap<u32, taffy::NodeId>,
        inner_gaps: f32,
        outer_gaps: f32,
    ) -> Self {
        let taffy_root_id =
            Self::build_node(&mut tree, root.clone(), root_id, &mut id_map, inner_gaps);
        let mut root_style = tree.style(taffy_root_id).unwrap().clone();
        root_style.size = taffy::Size {
            width: taffy::Dimension::Percent(1.0),
            height: taffy::Dimension::Percent(1.0),
        };
        root_style.padding = taffy::Rect::length(outer_gaps);
        tree.set_style(taffy_root_id, root_style).unwrap();

        Self {
            tree,
            id_map,
            root_id,
            root,
            taffy_root_id,
            inner_gaps,
            outer_gaps,
        }
    }

    pub fn compute_geos(
        &mut self,
        width: u32,
        height: u32,
    ) -> BTreeMap<u32, Rectangle<i32, Logical>> {
        self.tree
            .compute_layout(
                self.taffy_root_id,
                taffy::Size {
                    width: taffy::AvailableSpace::Definite(width as f32),
                    height: taffy::AvailableSpace::Definite(height as f32),
                },
            )
            .unwrap();

        self.tree.print_tree(self.taffy_root_id);

        let mut geos = BTreeMap::<u32, Rectangle<i32, Logical>>::new();

        fn compute_geos_rec(
            geos: &mut BTreeMap<u32, Rectangle<i32, Logical>>,
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

            let index = *tree.get_node_context(node).unwrap();

            let children = tree.children(node).unwrap();

            if children.is_empty() {
                let rect: Rectangle<i32, Logical> = Rectangle {
                    loc: smithay::utils::Point::from((loc.x, loc.y)),
                    size: smithay::utils::Size::from((size.width, size.height)),
                }
                .to_i32_round();
                geos.insert(index, rect);
                return;
            }

            for child in children.into_iter() {
                compute_geos_rec(geos, tree, child, loc.x, loc.y);
            }
        }

        compute_geos_rec(&mut geos, &self.tree, self.taffy_root_id, 0.0, 0.0);

        geos
    }

    pub fn diff(&mut self, root: LayoutNode, root_id: u32) {
        for node in self.id_map.values().copied() {
            self.tree.set_children(node, &[]).unwrap();
        }
        let tree = std::mem::replace(&mut self.tree, taffy::TaffyTree::new());
        let id_map = std::mem::take(&mut self.id_map);
        *self = Self::new_with_data(
            root,
            root_id,
            tree,
            id_map,
            self.inner_gaps,
            self.outer_gaps,
        );

        // if root_id != self.root_id {
        //     // If the root node's id has changed we're just gonna make a whole new tree
        //     *self = Self::new(root, root_id);
        //     return;
        // }
        //
        // if self.root.style != root.style {
        //     let mut new_root_style = root.style.clone();
        //     new_root_style.size = taffy::Size {
        //         width: taffy::Dimension::Percent(1.0),
        //         height: taffy::Dimension::Percent(1.0),
        //     };
        //     self.tree
        //         .set_style(self.taffy_root_id, new_root_style)
        //         .unwrap();
        // }
        //
        // let mut d = treediff::tools::Recorder::default();
        // treediff::diff(&self.root, &root, &mut d);
        //
        // dbg!(&d.calls);
        //
        // for call in d.calls {
        //     match call {
        //         treediff::tools::ChangeType::Removed(vec, _node) => {
        //             let last_node = *self.id_map.get(vec.last().unwrap()).unwrap();
        //             let parent = vec
        //                 .iter()
        //                 .nth_back(1)
        //                 .map(|id| *self.id_map.get(id).unwrap())
        //                 .unwrap_or(self.taffy_root_id);
        //             self.tree.remove_child(parent, last_node).unwrap();
        //         }
        //         treediff::tools::ChangeType::Added(mut vec, node) => {
        //             let mut parent_node = &root;
        //
        //             let new_node_id = vec.pop().unwrap();
        //
        //             for id in vec.iter() {
        //                 parent_node = parent_node.children.get(id).unwrap();
        //             }
        //
        //             let parent = vec
        //                 .last()
        //                 .map(|id| *self.id_map.get(id).unwrap())
        //                 .unwrap_or(self.taffy_root_id);
        //             let child = Self::build_node(
        //                 &mut self.tree,
        //                 node.clone(),
        //                 new_node_id,
        //                 &mut self.id_map,
        //             );
        //
        //             let insert_index = parent_node.children.get_index_of(&new_node_id).unwrap();
        //
        //             self.tree
        //                 .insert_child_at_index(parent, insert_index, child)
        //                 .unwrap();
        //         }
        //         treediff::tools::ChangeType::Unchanged(..) => (),
        //         treediff::tools::ChangeType::Modified(vec, _old_node, new_node) => {
        //             match vec.last().copied() {
        //                 Some(last) => {
        //                     Self::build_node(
        //                         &mut self.tree,
        //                         new_node.clone(),
        //                         last,
        //                         &mut self.id_map,
        //                     );
        //                 }
        //                 None => {
        //                     Self::build_node(
        //                         &mut self.tree,
        //                         new_node.clone(),
        //                         self.root_id,
        //                         &mut self.id_map,
        //                     );
        //
        //                     let mut root_style =
        //                         self.tree.style(self.taffy_root_id).unwrap().clone();
        //                     root_style.size = taffy::Size {
        //                         width: taffy::Dimension::Percent(1.0),
        //                         height: taffy::Dimension::Percent(1.0),
        //                     };
        //                     self.tree.set_style(self.taffy_root_id, root_style).unwrap();
        //                 }
        //             }
        //         }
        //     }
        // }
        //
        // self.root = root;
    }
}
