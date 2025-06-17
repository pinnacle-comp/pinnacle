//! A Rust port of the Zhang-Shasha diff algorithm, taken from
//! the [Gumtree implementation](https://github.com/GumTreeDiff/gumtree/blob/5b939f8c775a8cf22c3a27a3e68e4fa85b42ae02/core/src/main/java/com/github/gumtreediff/matchers/optimal/zs/ZsMatcher.java).

use std::collections::{HashMap, HashSet, VecDeque};

use slab_tree::NodeRef;

use super::MappingStore;

pub fn zhang_shasha<'a, T>(
    src: NodeRef<'a, T>,
    dst: NodeRef<'a, T>,
    mappings: MappingStore,
    cost_insert: fn(NodeRef<'_, T>) -> f64,
    cost_remove: fn(NodeRef<'_, T>) -> f64,
    cost_update: fn(NodeRef<'_, T>, NodeRef<'_, T>) -> f64,
    labels_same: fn(NodeRef<'_, T>, NodeRef<'_, T>) -> bool,
) -> MappingStore {
    let mut matcher = ZsMatcher {
        mappings,
        zs_src: ZsTree::new(src),
        zs_dst: ZsTree::new(dst),
        tree_dist: Vec::new(),
        forest_dist: Vec::new(),
        cost_insert,
        cost_remove,
        cost_update,
        labels_same,
    };

    matcher.r#match();

    matcher.mappings
}

struct ZsMatcher<'a, T> {
    mappings: MappingStore,
    zs_src: ZsTree<'a, T>,
    zs_dst: ZsTree<'a, T>,
    tree_dist: Vec<Vec<f64>>,
    forest_dist: Vec<Vec<f64>>,
    cost_insert: fn(NodeRef<'_, T>) -> f64,
    cost_remove: fn(NodeRef<'_, T>) -> f64,
    cost_update: fn(NodeRef<'_, T>, NodeRef<'_, T>) -> f64,
    labels_same: fn(NodeRef<'_, T>, NodeRef<'_, T>) -> bool,
}

impl<'a, T> ZsMatcher<'a, T> {
    fn r#match(&mut self) {
        self.compute_tree_dist();

        let mut root_node_pair = true;

        let mut tree_pairs = VecDeque::new();

        tree_pairs.push_front([self.zs_src.node_count, self.zs_dst.node_count]);

        while let Some([last_row, last_col]) = tree_pairs.pop_front() {
            if !root_node_pair {
                self.forest_dist(last_row, last_col);
            }

            root_node_pair = false;

            let first_row = self.zs_src.lld(last_row) - 1;
            let first_col = self.zs_dst.lld(last_col) - 1;

            let mut row = last_row;
            let mut col = last_col;

            while row > first_row || col > first_col {
                if (row > first_row)
                    && self.forest_dist[row - 1][col] + 1.0 == self.forest_dist[row][col]
                {
                    row -= 1;
                } else if (col > first_col)
                    && (self.forest_dist[row][col - 1] + 1.0 == self.forest_dist[row][col])
                {
                    col -= 1;
                } else if (self.zs_src.lld(row) - 1 == self.zs_src.lld(last_row) - 1)
                    && self.zs_dst.lld(col) - 1 == self.zs_dst.lld(last_col) - 1
                {
                    let t_src = self.zs_src.tree(row);
                    let t_dst = self.zs_dst.tree(col);
                    if (self.labels_same)(t_src, t_dst) {
                        self.mappings.add_mapping(t_src.node_id(), t_dst.node_id());
                    } else {
                        panic!("should not map incompatible nodes");
                    }
                    row -= 1;
                    col -= 1;
                } else {
                    tree_pairs.push_front([row, col]);

                    row = self.zs_src.lld(row) - 1;
                    col = self.zs_dst.lld(col) - 1;
                }
            }
        }
    }

    fn compute_tree_dist(&mut self) {
        self.tree_dist = vec![vec![0.0; self.zs_dst.node_count + 1]; self.zs_src.node_count + 1];
        self.forest_dist = vec![vec![0.0; self.zs_dst.node_count + 1]; self.zs_src.node_count + 1];

        for i in 1..self.zs_src.kr.len() {
            for j in 1..self.zs_dst.kr.len() {
                self.forest_dist(self.zs_src.kr[i], self.zs_dst.kr[j]);
            }
        }
    }

    fn forest_dist(&mut self, i: usize, j: usize) {
        self.forest_dist[self.zs_src.lld(i) - 1][self.zs_dst.lld(j) - 1] = 0.0;
        for di in self.zs_src.lld(i)..=i {
            let cost_del = (self.cost_remove)(self.zs_src.tree(di));
            self.forest_dist[di][self.zs_dst.lld(j) - 1] =
                self.forest_dist[di - 1][self.zs_dst.lld(j) - 1] + cost_del;

            for dj in self.zs_dst.lld(j)..=j {
                let cost_insert = (self.cost_insert)(self.zs_dst.tree(dj));
                self.forest_dist[self.zs_src.lld(i) - 1][dj] =
                    self.forest_dist[self.zs_src.lld(i) - 1][dj - 1] + cost_insert;

                if self.zs_src.lld(di) == self.zs_src.lld(i)
                    && self.zs_dst.lld(dj) == self.zs_dst.lld(j)
                {
                    let cost_update =
                        (self.cost_update)(self.zs_src.tree(di), self.zs_dst.tree(dj));

                    self.forest_dist[di][dj] = [
                        self.forest_dist[di - 1][dj] + cost_del,
                        self.forest_dist[di][dj - 1] + cost_insert,
                        self.forest_dist[di - 1][dj - 1] + cost_update,
                    ]
                    .into_iter()
                    .min_by(|a, b| a.total_cmp(b))
                    .unwrap();

                    self.tree_dist[di][dj] = self.forest_dist[di][dj];
                } else {
                    self.forest_dist[di][dj] = [
                        self.forest_dist[di - 1][dj] + cost_del,
                        self.forest_dist[di][dj - 1] + cost_insert,
                        self.forest_dist[self.zs_src.lld(di) - 1][self.zs_dst.lld(dj) - 1]
                            + self.tree_dist[di][dj],
                    ]
                    .into_iter()
                    .min_by(|a, b| a.total_cmp(b))
                    .unwrap();
                }
            }
        }
    }
}

struct ZsTree<'a, T> {
    node_count: usize,
    leaf_count: usize,
    llds: Vec<usize>,
    labels: Vec<Option<NodeRef<'a, T>>>,
    kr: Vec<usize>,
}

fn get_first_leaf<'a, T>(tree: NodeRef<'a, T>) -> NodeRef<'a, T> {
    tree.traverse_pre_order()
        .find(|node| node.children().next().is_none())
        .expect("there is always a leaf")
}

impl<'a, T> ZsTree<'a, T> {
    fn new(tree: NodeRef<'a, T>) -> Self {
        let node_count = tree.traverse_post_order().count();
        let mut this = ZsTree {
            node_count,
            leaf_count: 0,
            llds: vec![0; node_count],
            labels: vec![None; node_count],
            kr: Vec::new(),
        };

        let mut idx = 1;
        let mut tmp_data = HashMap::new();
        for n in tree.traverse_post_order() {
            tmp_data.insert(n, idx);
            this.set_i_tree(idx, n);
            this.set_lld(idx, *tmp_data.get(&get_first_leaf(n)).unwrap());
            if n.children().next().is_none() {
                this.leaf_count += 1;
            }
            idx += 1;
        }

        this.set_keyroots();

        this
    }

    fn set_i_tree(&mut self, i: usize, tree: NodeRef<'a, T>) {
        self.labels[i - 1] = Some(tree);
        if self.node_count < i {
            self.node_count = i;
        }
    }

    fn set_lld(&mut self, i: usize, lld: usize) {
        self.llds[i - 1] = lld - 1;
        if self.node_count < i {
            self.node_count = i;
        }
    }

    fn lld(&self, i: usize) -> usize {
        self.llds[i - 1] + 1
    }

    fn tree(&self, i: usize) -> NodeRef<'a, T> {
        self.labels[i - 1].unwrap()
    }

    fn set_keyroots(&mut self) {
        self.kr = vec![0; self.leaf_count + 1];
        let mut visited = HashSet::new();
        let mut k = self.kr.len() - 1;
        for i in (1..=self.node_count).rev() {
            if !visited.contains(&self.lld(i)) {
                self.kr[k] = i;
                visited.insert(self.lld(i));
                k -= 1;
            }
        }
    }
}
