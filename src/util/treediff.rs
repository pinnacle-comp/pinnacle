// Feast your eyes on the absolute worst pile of steaming poop that is this file.
// What you're looking at is the culmination of the two weeks I spent in the rabbit
// hole of tree diffing algorithms.

use bimap::BiHashMap;
use itertools::{EitherOrBoth, Itertools};
use slab_tree::{NodeId, NodeMut, NodeRef, RemoveBehavior, Tree};

use std::collections::{BTreeMap, HashMap, HashSet};

pub fn diff<T: Clone + Default + PartialEq + std::fmt::Debug>(
    src_tree: &Tree<T>,
    dst_tree: &Tree<T>,
    cost_insert: fn(NodeRef<'_, T>) -> f64,
    cost_remove: fn(NodeRef<'_, T>) -> f64,
    cost_update: fn(NodeRef<'_, T>, NodeRef<'_, T>) -> f64,
    labels_same: fn(NodeRef<'_, T>, NodeRef<'_, T>) -> bool,
) -> Vec<EditAction<T>> {
    let mappings = gumtree(
        src_tree,
        dst_tree,
        cost_insert,
        cost_remove,
        cost_update,
        labels_same,
    );
    let edit_script = chawathe_edit_script(src_tree, dst_tree, mappings);
    edit_script
}

//////////////////////////////////////////////////

#[derive(Default, Debug)]
struct MappingStore {
    pub mappings: BiHashMap<NodeId, NodeId>,
}

impl MappingStore {
    fn add_mapping(&mut self, src: NodeId, dst: NodeId) {
        match self.mappings.insert(src, dst) {
            bimap::Overwritten::Neither | bimap::Overwritten::Pair(_, _) => (),
            bimap::Overwritten::Left(_, _)
            | bimap::Overwritten::Right(_, _)
            | bimap::Overwritten::Both(_, _) => panic!("tried to overwrite a mapping"),
        }
    }

    fn src_for_dst(&self, dst: NodeId) -> Option<NodeId> {
        self.mappings.get_by_right(&dst).copied()
    }

    fn dst_for_src(&self, src: NodeId) -> Option<NodeId> {
        self.mappings.get_by_left(&src).copied()
    }

    fn is_src_mapped(&self, src: NodeId) -> bool {
        self.mappings.contains_left(&src)
    }

    fn is_dst_mapped(&self, dst: NodeId) -> bool {
        self.mappings.contains_right(&dst)
    }

    fn update_src_mapping(&mut self, old_src: NodeId, new_src: NodeId) {
        let Some((_, dst)) = self.mappings.remove_by_left(&old_src) else {
            return;
        };
        self.mappings.insert(new_src, dst);
    }

    fn contains(&self, src: NodeId, dst: NodeId) -> bool {
        self.mappings
            .get_by_left(&src)
            .is_some_and(|right| *right == dst)
    }

    fn into_iter(self) -> bimap::hash::IntoIter<NodeId, NodeId> {
        self.mappings.into_iter()
    }
}

impl Extend<(NodeId, NodeId)> for MappingStore {
    fn extend<T: IntoIterator<Item = (NodeId, NodeId)>>(&mut self, iter: T) {
        self.mappings.extend(iter);
    }
}

////////////////////////////////////////////

fn zhang_shasha<T, FC, FR, FU>(
    t1: &NodeRef<'_, T>,
    t2: &NodeRef<'_, T>,

    src_tree: &Tree<T>,
    dst_tree: &Tree<T>,
    cost_insert: FC,
    cost_remove: FR,
    cost_update: FU,
) -> (f64, Vec<Op>)
where
    FC: Fn(NodeRef<'_, T>) -> f64,
    FR: Fn(NodeRef<'_, T>) -> f64,
    FU: Fn(NodeRef<'_, T>, NodeRef<'_, T>) -> f64,
{
    let (id_to_node_i, node_to_id_i) = tree_id_map(t1);
    let (id_to_node_j, node_to_id_j) = tree_id_map(t2);

    let lr_kr_1 = lr_keyroots(t1);
    let lr_kr_2 = lr_keyroots(t2);

    let l_map_i = llds_map(t1, &node_to_id_i);
    let l_map_j = llds_map(t2, &node_to_id_j);

    let size_a = t1.traverse_post_order().count();
    let size_b = t2.traverse_post_order().count();
    let mut treedist = vec![vec![0.0f64; size_b]; size_a];
    let mut ops = vec![vec![vec![]; size_b]; size_a];

    for i in lr_kr_1 {
        for j in lr_kr_2.iter().copied() {
            tree_dist(
                i,
                j,
                &id_to_node_i,
                &id_to_node_j,
                &l_map_i,
                &l_map_j,
                &mut treedist,
                &mut ops,
                src_tree,
                dst_tree,
                &cost_insert,
                &cost_remove,
                &cost_update,
            );
        }
    }

    (
        *treedist.last().unwrap().last().unwrap(),
        ops.last().unwrap().last().unwrap().clone(),
    )
}

fn llds_map<T>(root: &NodeRef<'_, T>, node_to_id: &HashMap<NodeId, u32>) -> HashMap<u32, u32> {
    let mut map = HashMap::new();
    for node in root.traverse_post_order() {
        let this_id = node_to_id[&node.node_id()];
        let l_node = node
            .traverse_pre_order()
            .find(|node| node.children().next().is_none())
            .unwrap();
        let l_id = node_to_id[&l_node.node_id()];
        map.insert(this_id, l_id);
    }
    map
}

fn tree_id_map<T>(root: &NodeRef<'_, T>) -> (HashMap<u32, NodeId>, HashMap<NodeId, u32>) {
    let mut id_to_node = HashMap::new();
    let mut node_to_id = HashMap::new();
    for (idx, node) in root.traverse_post_order().enumerate() {
        let idx = idx as u32;
        id_to_node.insert(idx, node.node_id());
        node_to_id.insert(node.node_id(), idx);
    }
    (id_to_node, node_to_id)
}

fn lr_keyroots<T>(tree: &NodeRef<'_, T>) -> Vec<u32> {
    tree.traverse_post_order()
        .enumerate()
        .filter_map(
            |(i, node)| match node.parent().is_none() || node.prev_sibling().is_some() {
                true => Some(i as u32),
                false => None,
            },
        )
        .collect()
}

fn tree_dist<T, FC, FR, FU>(
    i: u32,
    j: u32,
    src_id_map: &HashMap<u32, NodeId>,
    dst_id_map: &HashMap<u32, NodeId>,
    src_llds_map: &HashMap<u32, u32>,
    dst_llds_map: &HashMap<u32, u32>,
    tree_dist: &mut [Vec<f64>],
    operations: &mut [Vec<Vec<Op>>],
    src_tree: &Tree<T>,
    dst_tree: &Tree<T>,
    cost_insert: FC,
    cost_remove: FR,
    cost_update: FU,
) where
    FC: Fn(NodeRef<'_, T>) -> f64,
    FR: Fn(NodeRef<'_, T>) -> f64,
    FU: Fn(NodeRef<'_, T>, NodeRef<'_, T>) -> f64,
{
    let m = i - src_llds_map[&i] + 2;
    let n = j - dst_llds_map[&j] + 2;
    let mut forest_dist = vec![vec![0.0f64; n as usize]; m as usize];
    let mut partial_ops = vec![vec![vec![]; n as usize]; m as usize];

    let ioff: i32 = src_llds_map[&i] as i32 - 1;
    let joff: i32 = dst_llds_map[&j] as i32 - 1;

    for x in 1..m {
        let x = x as usize;
        let node_id = src_id_map[&((x as i32 + ioff) as u32)];
        forest_dist[x][0] = forest_dist[x - 1][0] + cost_remove(src_tree.get(node_id).unwrap());
        partial_ops[x][0] = partial_ops[x - 1][0]
            .clone()
            .into_iter()
            .chain([Op::Remove])
            .collect();
    }
    for y in 1..n {
        let y = y as usize;
        let node_id = dst_id_map[&((y as i32 + joff) as u32)];
        forest_dist[0][y] = forest_dist[0][y - 1] + cost_insert(dst_tree.get(node_id).unwrap());
        partial_ops[0][y] = partial_ops[0][y - 1]
            .clone()
            .into_iter()
            .chain([Op::Add])
            .collect();
    }

    for x in 1..m {
        for y in 1..n {
            if src_llds_map[&i] == src_llds_map[&((x as i32 + ioff) as u32)]
                && dst_llds_map[&j] == dst_llds_map[&((y as i32 + joff) as u32)]
            {
                let x = x as usize;
                let y = y as usize;

                let src_node_id = src_id_map[&((x as i32 + ioff) as u32)];
                let dst_node_id = dst_id_map[&((y as i32 + joff) as u32)];

                let costs = [
                    forest_dist[x - 1][y] + cost_remove(src_tree.get(src_node_id).unwrap()),
                    forest_dist[x][y - 1] + cost_insert(dst_tree.get(dst_node_id).unwrap()),
                    forest_dist[x - 1][y - 1]
                        + cost_update(
                            src_tree.get(src_node_id).unwrap(),
                            dst_tree.get(dst_node_id).unwrap(),
                        ),
                ];
                forest_dist[x][y] = costs.into_iter().min_by(|a, b| a.total_cmp(b)).unwrap();

                let min_index = costs
                    .into_iter()
                    .position(|it| it == forest_dist[x][y])
                    .unwrap();

                match min_index {
                    0 => {
                        partial_ops[x][y] = partial_ops[x - 1][y]
                            .clone()
                            .into_iter()
                            .chain([Op::Remove])
                            .collect();
                    }
                    1 => {
                        partial_ops[x][y] = partial_ops[x][y - 1]
                            .clone()
                            .into_iter()
                            .chain([Op::Add])
                            .collect();
                    }
                    2 => {
                        partial_ops[x][y] = partial_ops[x - 1][y - 1]
                            .clone()
                            .into_iter()
                            .chain([Op::Update(src_node_id, dst_node_id)])
                            .collect();
                    }
                    _ => unreachable!(),
                }

                tree_dist[(x as i32 + ioff) as usize][(y as i32 + joff) as usize] =
                    forest_dist[x][y];

                operations[(x as i32 + ioff) as usize][(y as i32 + joff) as usize] =
                    partial_ops[x][y].clone();
            } else {
                let p = src_llds_map[&((x as i32 + ioff) as u32)] as i32 - 1 - ioff;
                let q = dst_llds_map[&((y as i32 + joff) as u32)] as i32 - 1 - joff;
                let x = x as usize;
                let y = y as usize;

                let src_node_id = src_id_map[&((x as i32 + ioff) as u32)];
                let dst_node_id = dst_id_map[&((y as i32 + joff) as u32)];

                let costs = [
                    forest_dist[x - 1][y] + cost_remove(src_tree.get(src_node_id).unwrap()),
                    forest_dist[x][y - 1] + cost_insert(dst_tree.get(dst_node_id).unwrap()),
                    forest_dist[p as usize][q as usize]
                        + tree_dist[(x as i32 + ioff) as usize][(y as i32 + joff) as usize],
                ];
                forest_dist[x][y] = costs.into_iter().min_by(|a, b| a.total_cmp(b)).unwrap();

                let min_index = costs
                    .into_iter()
                    .position(|it| it == forest_dist[x][y])
                    .unwrap();

                match min_index {
                    0 => {
                        partial_ops[x][y] = partial_ops[x - 1][y]
                            .clone()
                            .into_iter()
                            .chain([Op::Remove])
                            .collect();
                    }
                    1 => {
                        partial_ops[x][y] = partial_ops[x][y - 1]
                            .clone()
                            .into_iter()
                            .chain([Op::Add])
                            .collect();
                    }
                    2 => {
                        partial_ops[x][y] = partial_ops[p as usize][q as usize]
                            .clone()
                            .into_iter()
                            .chain(
                                operations[(x as i32 + ioff) as usize][(y as i32 + joff) as usize]
                                    .clone(),
                            )
                            .collect();
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
enum Op {
    Add,
    Remove,
    Update(NodeId, NodeId),
}

fn gumtree<T: PartialEq + std::fmt::Debug>(
    tree1: &Tree<T>,
    tree2: &Tree<T>,
    cost_insert: fn(NodeRef<'_, T>) -> f64,
    cost_remove: fn(NodeRef<'_, T>) -> f64,
    cost_update: fn(NodeRef<'_, T>, NodeRef<'_, T>) -> f64,
    labels_same: fn(NodeRef<'_, T>, NodeRef<'_, T>) -> bool,
) -> MappingStore {
    let mappings = gumtree_top_down(tree1, tree2);

    gumtree_bottom_up(
        tree1,
        tree2,
        mappings,
        0.5,
        100,
        cost_insert,
        cost_remove,
        cost_update,
        labels_same,
    )
}

fn gumtree_top_down<T: PartialEq + std::fmt::Debug>(
    tree1: &Tree<T>,
    tree2: &Tree<T>,
) -> MappingStore {
    let mut l1 = BTreeMap::<usize, Vec<slab_tree::NodeId>>::new();
    let mut l2 = BTreeMap::<usize, Vec<slab_tree::NodeId>>::new();

    let root1 = tree1.root_id().unwrap();
    let root2 = tree2.root_id().unwrap();

    l1.entry(height(&tree1.get(root1).unwrap()))
        .or_default()
        .push(root1);
    l2.entry(height(&tree2.get(root2).unwrap()))
        .or_default()
        .push(root2);

    let mut a = Vec::<(NodeId, NodeId)>::new();
    let mut m = MappingStore::default();

    while let (
        Some((l1_max_height, l1_max_height_nodes)),
        Some((l2_max_height, l2_max_height_nodes)),
    ) = (l1.pop_last(), l2.pop_last())
    {
        if std::cmp::min(l1_max_height, l2_max_height) < 2 {
            break;
        }

        // match is one more level of indentation
        #[allow(clippy::comparison_chain)]
        if l1_max_height > l2_max_height {
            debug_assert!(l2.insert(l2_max_height, l2_max_height_nodes).is_none());
            for node in l1_max_height_nodes {
                for child in tree1.get(node).unwrap().children() {
                    l1.entry(height(&child)).or_default().push(child.node_id());
                }
            }
        } else if l1_max_height < l2_max_height {
            debug_assert!(l1.insert(l1_max_height, l1_max_height_nodes).is_none());
            for node in l2_max_height_nodes {
                for child in tree2.get(node).unwrap().children() {
                    l2.entry(height(&child)).or_default().push(child.node_id());
                }
            }
        } else {
            let h1 = l1_max_height_nodes;
            let h2 = l2_max_height_nodes;
            for (t1, t2) in h1.iter().copied().cartesian_product(h2.iter().copied()) {
                let t1_ref = tree1.get(t1).unwrap();
                let t2_ref = tree2.get(t2).unwrap();
                if isomorphic(&t1_ref, &t2_ref) {
                    let t1_has_another_iso = t2_ref
                        .traverse_post_order()
                        .any(|node| node.node_id() != t2 && isomorphic(&t1_ref, &node));

                    let t2_has_another_iso = t1_ref
                        .traverse_post_order()
                        .any(|node| node.node_id() != t1 && isomorphic(&node, &t2_ref));

                    if t1_has_another_iso || t2_has_another_iso {
                        a.push((t1, t2));
                    } else {
                        let t1_traverse = t1_ref.traverse_pre_order().map(|node| node.node_id());
                        let t2_traverse = t2_ref.traverse_pre_order().map(|node| node.node_id());
                        m.extend(t1_traverse.zip(t2_traverse));
                    }
                }
            }

            for t1 in h1 {
                if !a.iter().any(|(node, _)| *node == t1) && !m.is_src_mapped(t1) {
                    for child in tree1.get(t1).unwrap().children() {
                        l1.entry(height(&child)).or_default().push(child.node_id());
                    }
                }
            }

            for t2 in h2 {
                if !a.iter().any(|(_, node)| *node == t2) && !m.is_dst_mapped(t2) {
                    for child in tree2.get(t2).unwrap().children() {
                        l2.entry(height(&child)).or_default().push(child.node_id());
                    }
                }
            }
        }
    }

    a.sort_by(|(t1, t2), (t11, t21)| {
        dice(
            &tree1.get(*t1).unwrap().parent().unwrap(),
            &tree2.get(*t2).unwrap().parent().unwrap(),
            &m,
        )
        .total_cmp(&dice(
            &tree1.get(*t11).unwrap().parent().unwrap(),
            &tree2.get(*t21).unwrap().parent().unwrap(),
            &m,
        ))
        .reverse()
    });

    while !a.is_empty() {
        let (t1, t2) = a.remove(0);

        let t1_ref = tree1.get(t1).unwrap();
        let t2_ref = tree2.get(t2).unwrap();
        let t1_traverse = t1_ref.traverse_pre_order().map(|node| node.node_id());
        let t2_traverse = t2_ref.traverse_pre_order().map(|node| node.node_id());
        m.extend(t1_traverse.zip(t2_traverse));

        a.retain(|(l, r)| *l == t1 || *r == t2);
    }

    m
}

fn gumtree_bottom_up<T>(
    t1: &Tree<T>,
    t2: &Tree<T>,
    mut mappings: MappingStore,
    min_dice: f32,
    max_size: usize,

    cost_insert: fn(NodeRef<'_, T>) -> f64,
    cost_remove: fn(NodeRef<'_, T>) -> f64,
    cost_update: fn(NodeRef<'_, T>, NodeRef<'_, T>) -> f64,
    labels_same: fn(NodeRef<'_, T>, NodeRef<'_, T>) -> bool,
) -> MappingStore {
    let last_chance_match = |src_node: &NodeRef<'_, T>,
                             dst_node: &NodeRef<'_, T>,
                             mappings: &mut MappingStore,
                             max_size: usize| {
        let t1_size = src_node.traverse_post_order().count();
        let t2_size = dst_node.traverse_post_order().count();

        if t1_size >= max_size || t2_size >= max_size {
            return;
        }

        let (_, ops) = zhang_shasha(
            src_node,
            dst_node,
            t1,
            t2,
            cost_insert,
            cost_remove,
            cost_update,
        );
        let zs_mappings = ops
            .into_iter()
            .filter_map(|op| match op {
                Op::Add => None,
                Op::Remove => None,
                Op::Update(node_id, node_id1) => Some((node_id, node_id1)),
            })
            .collect::<Vec<_>>();

        for (src, dst) in zs_mappings {
            let ta_mapped = mappings.is_src_mapped(src);
            let tb_mapped = mappings.is_dst_mapped(dst);
            let labels_same = labels_same(t1.get(src).unwrap(), t2.get(dst).unwrap());
            if !ta_mapped && !tb_mapped && labels_same {
                mappings.add_mapping(src, dst);
            }
        }
    };

    for t in t1.root().unwrap().traverse_post_order() {
        if t.parent().is_none() {
            mappings.add_mapping(t.node_id(), t2.root_id().unwrap());
            last_chance_match(&t, &t2.root().unwrap(), &mut mappings, max_size);
            break;
        } else if !mappings.is_src_mapped(t.node_id()) && t.children().next().is_some() {
            let candidates = candidates(&t, t2, &mappings);

            let mut best = None::<NodeId>;
            let mut max = -1.0;

            for cand in candidates {
                let cand_ref = t2.get(cand).unwrap();
                let dice = dice(&t, &cand_ref, &mappings);
                if dice > max && dice >= min_dice {
                    max = dice;
                    best = Some(cand);
                }
            }

            if let Some(best) = best {
                let best_ref = t2.get(best).unwrap();
                last_chance_match(&t, &best_ref, &mut mappings, max_size);
                mappings.add_mapping(t.node_id(), best);
            }
        }
    }

    mappings
}

fn candidates<T>(src: &NodeRef<'_, T>, dst_tree: &Tree<T>, mappings: &MappingStore) -> Vec<NodeId> {
    let mut seeds = Vec::new();
    for des in src.traverse_pre_order().skip(1) {
        if let Some(dst) = mappings.dst_for_src(des.node_id()) {
            seeds.push(dst);
        }
    }

    let mut candidates = Vec::new();
    let mut visited = HashSet::new();
    for mut seed in seeds {
        while let Some(parent) = dst_tree.get(seed).unwrap().parent() {
            if visited.contains(&parent.node_id()) {
                break;
            }
            visited.insert(parent.node_id());
            let parent_mapped = mappings.is_dst_mapped(parent.node_id());
            if
            /* types same && */
            !parent_mapped && parent.parent().is_some() {
                candidates.push(parent.node_id());
            }
            seed = parent.node_id();
        }
    }

    candidates
}

fn height<T>(node: &NodeRef<'_, T>) -> usize {
    let mut children = node.children().peekable();

    if children.peek().is_none() {
        return 1;
    }

    children.map(|child| height(&child)).max().unwrap() + 1
}

fn isomorphic<T: PartialEq + std::fmt::Debug>(
    node1: &NodeRef<'_, T>,
    node2: &NodeRef<'_, T>,
) -> bool {
    if node1.data() != node2.data() {
        return false;
    }

    let n1_children = node1.children().collect::<Vec<_>>();
    let n2_children = node2.children().collect::<Vec<_>>();
    if n1_children.len() != n2_children.len() {
        return false;
    }

    for children in n1_children.into_iter().zip_longest(n2_children) {
        let EitherOrBoth::Both(n1_child, n2_child) = children else {
            unreachable!()
        };
        if !isomorphic(&n1_child, &n2_child) {
            return false;
        }
    }

    true
}

fn dice<T>(t1: &NodeRef<'_, T>, t2: &NodeRef<'_, T>, mappings: &MappingStore) -> f32 {
    let descendants_of_t1_in_m = t1
        .traverse_pre_order()
        .skip(1)
        .filter(|des| {
            mappings.is_src_mapped(des.node_id())
                && t2
                    .traverse_pre_order()
                    .skip(1)
                    .any(|t2des| mappings.dst_for_src(des.node_id()) == Some(t2des.node_id()))
        })
        .count();

    let ret = (2.0 * descendants_of_t1_in_m as f32)
        / (t1.traverse_pre_order().skip(1).count() + t2.traverse_pre_order().skip(1).count())
            as f32;

    assert!(ret >= 0.0);
    assert!(ret <= 1.0);

    ret
}

#[derive(Debug, Clone)]
pub enum EditAction<T> {
    Insert {
        val: T,
        dst: Vec<usize>,
        idx: usize,
    },
    Delete(Vec<usize>),
    Update(Vec<usize>, T),
    Move {
        src: Vec<usize>,
        dst: Vec<usize>,
        idx: usize,
    },
}

fn clone_tree<T: Clone>(
    orig: &NodeRef<'_, T>,
    mut clone: NodeMut<'_, T>,
    orig_to_clone: &mut BiHashMap<NodeId, NodeId>,
) {
    orig_to_clone.insert(orig.node_id(), clone.node_id());
    for child in orig.children() {
        let new_child = clone.append(child.data().clone());
        clone_tree(&child, new_child, orig_to_clone);
    }
}

fn chawathe_edit_script<T: Clone + Default + PartialEq + std::fmt::Debug>(
    t1: &Tree<T>,
    t2: &Tree<T>,
    mappings: MappingStore,
) -> Vec<EditAction<T>> {
    // initWith
    let (mut t1, t1_orig_to_clone) = {
        let mut clone = Tree::new();

        let t1_root = t1.root().unwrap();
        clone.set_root(t1_root.data().clone());
        let clone_root = clone.root_mut().unwrap();

        let mut map = BiHashMap::new();
        clone_tree(&t1_root, clone_root, &mut map);

        (clone, map)
    };

    let (mut t2, t2_orig_to_clone) = {
        let mut clone = Tree::new();

        let t2_root = t2.root().unwrap();
        clone.set_root(t2_root.data().clone());
        let clone_root = clone.root_mut().unwrap();

        let mut map = BiHashMap::new();
        clone_tree(&t2_root, clone_root, &mut map);

        (clone, map)
    };

    let mut mappings = MappingStore {
        mappings: mappings
            .into_iter()
            .map(|(src, dst)| {
                (
                    *t1_orig_to_clone.get_by_left(&src).unwrap(),
                    *t2_orig_to_clone.get_by_left(&dst).unwrap(),
                )
            })
            .collect(),
    };

    let mut edits = Vec::new();
    let mut dst_in_order = HashSet::new();
    let mut src_in_order = HashSet::new();

    let src_fake_root = t1.set_root(T::default());

    let prev_root = t2.root_id().unwrap();
    let dst_fake_root = t2.set_root(T::default());

    mappings.add_mapping(src_fake_root, dst_fake_root);

    for x in t2.get(prev_root).unwrap().traverse_level_order() {
        let mut w;
        let y = x.parent().unwrap();
        let z = mappings.src_for_dst(y.node_id()).unwrap();

        let dst_x_mapped = mappings.is_dst_mapped(x.node_id());
        if !dst_x_mapped {
            let k = find_pos(&x, &mappings, &t1, &t2, &dst_in_order, None);

            let mut z_mut = t1.get_mut(z).unwrap();
            let w_mut = z_mut.prepend(x.data().clone());
            w = w_mut.node_id();

            mappings.add_mapping(w, x.node_id());

            let mut w_mut = t1.get_mut(w).unwrap();
            for _ in 0..k {
                w_mut.swap_next_sibling();
            }

            edits.push(EditAction::Insert {
                val: x.data().clone(),
                dst: index_path_to_node(&t1, z),
                idx: k,
            });
        } else {
            w = mappings.src_for_dst(x.node_id()).unwrap();
            if x.node_id() != prev_root {
                let v = {
                    let mut w_mut = t1.get_mut(w).unwrap();
                    let v = w_mut.parent().unwrap().node_id();

                    if w_mut.data() != x.data() {
                        *w_mut.data() = x.data().clone();
                        edits.push(EditAction::Update(
                            index_path_to_node(&t1, w),
                            x.data().clone(),
                        ));
                    }

                    v
                };

                if z != v {
                    let k = find_pos(&x, &mappings, &t1, &t2, &dst_in_order, None);

                    edits.push(EditAction::Move {
                        src: index_path_to_node(&t1, w),
                        dst: index_path_to_node(&t1, z),
                        idx: k,
                    });

                    w = move_subtree(&mut t1, w, z, k, &mut mappings);
                }
            }
        }

        src_in_order.insert(w);
        dst_in_order.insert(x.node_id());
        align_children(
            w,
            x.node_id(),
            &mut t1,
            &t2,
            &mappings,
            &mut src_in_order,
            &mut dst_in_order,
            &mut edits,
        );
    }

    // PERF: This repeatedly loops the traversal, restarting whenever a node is removed to
    // get around the borrow checker.
    'outer: loop {
        for w in t1.root().unwrap().traverse_post_order() {
            if !mappings.is_src_mapped(w.node_id()) {
                edits.push(EditAction::Delete(index_path_to_node(&t1, w.node_id())));

                t1.remove(w.node_id(), RemoveBehavior::DropChildren);
                continue 'outer;
            }
        }

        break;
    }

    for edit in edits.iter_mut() {
        match edit {
            EditAction::Insert { dst, .. } => {
                dst.remove(0);
            }
            EditAction::Delete(vec) => {
                vec.remove(0);
            }
            EditAction::Update(vec, _) => {
                vec.remove(0);
            }
            EditAction::Move { src, dst, .. } => {
                src.remove(0);
                dst.remove(0);
            }
        }
    }

    debug_assert!(isomorphic(&t1.root().unwrap(), &t2.root().unwrap()));

    edits
}

/// Returns the new src_root
fn move_subtree<T: Clone>(
    tree: &mut Tree<T>,
    src_root: NodeId,
    dst_parent: NodeId,
    k: usize,
    mappings: &mut MappingStore,
) -> NodeId {
    let (subtree_clone, subtree_orig_to_clone) = {
        let mut clone = Tree::new();

        let subtree_root = tree.get(src_root).unwrap();
        clone.set_root(subtree_root.data().clone());
        let clone_root = clone.root_mut().unwrap();

        let mut map = BiHashMap::new();
        clone_tree(&subtree_root, clone_root, &mut map);

        (clone, map)
    };

    tree.remove(src_root, RemoveBehavior::DropChildren);

    let mut dst_parent = tree.get_mut(dst_parent).unwrap();
    let mut clone_to_dst = BiHashMap::new();
    let subtree_root = subtree_clone.root().unwrap();
    let mut dst_root = dst_parent.prepend(subtree_root.data().clone());
    for _ in 0..k {
        dst_root.swap_next_sibling();
    }

    let dst_root_id = dst_root.node_id();

    clone_tree(&subtree_root, dst_root, &mut clone_to_dst);
    for (subtree_clone_node, dst_node) in clone_to_dst {
        mappings.update_src_mapping(
            *subtree_orig_to_clone
                .get_by_right(&subtree_clone_node)
                .unwrap(),
            dst_node,
        );
    }

    dst_root_id
}

fn align_children<T>(
    w: NodeId,
    x: NodeId,
    src_tree_clone: &mut Tree<T>,
    dst_tree: &Tree<T>,
    mappings: &MappingStore,
    src_in_order: &mut HashSet<NodeId>,
    dst_in_order: &mut HashSet<NodeId>,
    edit_script: &mut Vec<EditAction<T>>,
) {
    let w_ref = src_tree_clone.get(w).unwrap();
    let x_ref = dst_tree.get(x).unwrap();

    for child in w_ref.children() {
        src_in_order.remove(&child.node_id());
    }
    for child in x_ref.children() {
        dst_in_order.remove(&child.node_id());
    }

    let s1 = w_ref
        .children()
        .filter(|child| {
            mappings.is_src_mapped(child.node_id())
                && x_ref
                    .children()
                    .any(|ch| ch.node_id() == mappings.dst_for_src(child.node_id()).unwrap())
        })
        .map(|node| node.node_id())
        .collect_vec();

    let s2 = x_ref
        .children()
        .filter(|child| {
            mappings.is_dst_mapped(child.node_id())
                && w_ref
                    .children()
                    .any(|ch| ch.node_id() == mappings.src_for_dst(child.node_id()).unwrap())
        })
        .map(|node| node.node_id())
        .collect_vec();

    let s = lcs(&s1, &s2, |a, b| mappings.contains(*a, *b));
    let s = s.into_iter().map(|(a, b)| (*a, *b)).collect_vec();

    // 5.
    for (a, b) in s.iter().copied() {
        src_in_order.insert(a);
        dst_in_order.insert(b);
    }

    // 6.
    // iterate through s2 first, to ensure left-to-right insertions
    for (b, a) in s2.iter().copied().cartesian_product(s1.iter().copied()) {
        if mappings.contains(a, b) && !s.contains(&(a, b)) {
            let k = find_pos(
                &dst_tree.get(b).unwrap(),
                mappings,
                src_tree_clone,
                dst_tree,
                dst_in_order,
                Some(a),
            );

            edit_script.push(EditAction::Move {
                src: index_path_to_node(src_tree_clone, a),
                dst: index_path_to_node(src_tree_clone, w),
                idx: k,
            });

            let mut a_mut = src_tree_clone.get_mut(a).unwrap();
            for _ in 0..k {
                a_mut.swap_next_sibling();
            }

            src_in_order.insert(a);
            dst_in_order.insert(b);
        }
    }
}

fn find_pos<T>(
    x: &NodeRef<'_, T>,
    mappings: &MappingStore,
    src_tree_clone: &Tree<T>,
    dst_tree: &Tree<T>,
    dst_in_order: &HashSet<NodeId>,
    ignore_src: Option<NodeId>,
) -> usize {
    let y = x.parent().unwrap();

    let siblings = y.children().collect_vec();

    for c in siblings.iter() {
        if dst_in_order.contains(&c.node_id()) {
            if c.node_id() == x.node_id() {
                return 0;
            } else {
                break;
            }
        }
    }

    let mut v = None;
    let mut current = x.node_id();
    while let Some(prev_sib) = dst_tree.get(current).unwrap().prev_sibling() {
        if dst_in_order.contains(&prev_sib.node_id()) {
            v = Some(prev_sib.node_id());
            break;
        }

        current = prev_sib.node_id();
    }

    let Some(v) = v else {
        return 0;
    };

    let u = mappings.src_for_dst(v).unwrap();

    let index = src_tree_clone
        .get(u)
        .unwrap()
        .parent()
        .unwrap()
        .children()
        .filter(|child| Some(child.node_id()) != ignore_src)
        .position(|sib| sib.node_id() == u)
        .unwrap();

    index + 1
}

fn index_path_to_node<T>(tree: &Tree<T>, node_id: NodeId) -> Vec<usize> {
    let mut indices = Vec::new();

    let index = |tree: &Tree<T>, mut node_id: NodeId| {
        let mut index = 0usize;
        while let Some(prev_sib) = tree.get(node_id).unwrap().prev_sibling() {
            index += 1;
            node_id = prev_sib.node_id();
        }
        index
    };

    indices.push(index(tree, node_id));

    for anc in tree.get(node_id).unwrap().ancestors() {
        indices.push(index(tree, anc.node_id()));
    }

    indices.into_iter().rev().collect()
}

// https://rustp.org/dynamic-programming/longest-common-subsequence/
// This website mixes up the m and n
// no I don't remember a whole lot from my algorithms course
pub fn lcs<'a, T, F>(string1: &'a [T], string2: &'a [T], mut equals: F) -> Vec<(&'a T, &'a T)>
where
    F: FnMut(&T, &T) -> bool,
{
    let m = string1.len();
    let n = string2.len();

    let mut dp = vec![vec![vec![]; n + 1]; m + 1];

    for i in 1..m + 1 {
        for j in 1..n + 1 {
            if equals(&string1[i - 1], &string2[j - 1]) {
                let mut subseq = dp[i - 1][j - 1].clone();
                subseq.push((&string1[i - 1], &string2[j - 1]));
                dp[i][j] = subseq;
            } else {
                dp[i][j] =
                    std::cmp::max_by_key(dp[i - 1][j].clone(), dp[i][j - 1].clone(), |subseq| {
                        subseq.len()
                    });
            }
        }
    }
    std::mem::take(&mut dp[m][n])
}
