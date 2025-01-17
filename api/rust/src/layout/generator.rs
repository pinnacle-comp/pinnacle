use super::{GapsAll, LayoutArgs, LayoutDir, LayoutGenerator, LayoutNode};

#[derive(Debug, Clone)]
pub struct Line {
    pub outer_gaps: GapsAll,
    pub inner_gaps: GapsAll,
    pub direction: LayoutDir,
    pub reversed: bool,
}

impl LayoutGenerator for Line {
    fn layout(&self, args: LayoutArgs) -> LayoutNode {
        let window_count = args.window_count;

        let root = LayoutNode::new_with_label("builtin.line");
        root.set_gaps(self.outer_gaps);
        root.set_dir(self.direction);

        if window_count == 0 {
            return root;
        }

        let children = match self.reversed {
            false => (0..window_count)
                .map(|idx| {
                    let node = LayoutNode::new_with_traversal_index(idx);
                    node.set_gaps(self.inner_gaps);
                    node
                })
                .collect::<Vec<_>>(),
            true => (0..window_count)
                .rev()
                .map(|idx| {
                    let node = LayoutNode::new_with_traversal_index(idx);
                    node.set_gaps(self.inner_gaps);
                    node
                })
                .collect(),
        };

        root.set_children(children);

        root
    }
}

/// Which side the master area will be.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MasterSide {
    /// The master area will be on the left.
    Left,
    /// The master area will be on the right.
    Right,
    /// The master area will be at the top.
    Top,
    /// The master area will be at the bottom.
    Bottom,
}

/// A [`LayoutGenerator`] that has one master area to one side and a stack of windows
/// next to it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MasterStackLayout {
    /// Gaps between windows.
    pub inner_gaps: GapsAll,
    pub outer_gaps: GapsAll,
    /// The proportion of the output the master area will take up.
    ///
    /// This will be clamped between 0.1 and 0.9.
    ///
    /// Defaults to 0.5
    pub master_factor: f32,
    /// Which side the master area will be.
    ///
    /// Defaults to [`MasterSide::Left`].
    pub master_side: MasterSide,
    /// How many windows will be in the master area.
    ///
    /// Defaults to 1.
    pub master_count: u32,
    /// Reverses the direction of window insertion i.e. new windows
    /// are inserted at the top of the master stack instead of at the
    /// bottom of the side stack.
    pub reversed: bool,
}

impl Default for MasterStackLayout {
    fn default() -> Self {
        Self {
            outer_gaps: GapsAll::from(4.0),
            inner_gaps: GapsAll::from(4.0),
            master_factor: 0.5,
            master_side: MasterSide::Left,
            master_count: 1,
            reversed: false,
        }
    }
}

impl LayoutGenerator for MasterStackLayout {
    fn layout(&self, args: LayoutArgs) -> LayoutNode {
        let win_count = args.window_count;

        let root = LayoutNode::new_with_label("builtin.master_stack");
        root.set_gaps(self.outer_gaps);
        root.set_dir(match self.master_side {
            MasterSide::Left | MasterSide::Right => LayoutDir::Row,
            MasterSide::Top | MasterSide::Bottom => LayoutDir::Column,
        });

        if win_count == 0 {
            return root;
        }

        let master_factor = self.master_factor.clamp(0.1, 0.9);

        let (master_tv_idx, stack_tv_idx) = match self.reversed {
            true => (1, 0),
            false => (0, 1),
        };

        let mut master_side_args = args.clone();
        master_side_args.window_count = u32::min(self.master_count, win_count);

        let line = Line {
            outer_gaps: 0.0.into(),
            inner_gaps: self.inner_gaps,
            direction: match self.master_side {
                MasterSide::Left | MasterSide::Right => LayoutDir::Column,
                MasterSide::Top | MasterSide::Bottom => LayoutDir::Row,
            },
            reversed: self.reversed,
        };

        let master_side = line.layout(master_side_args);
        master_side.set_traversal_index(master_tv_idx);
        master_side.set_size_proportion(master_factor * 10.0);

        if win_count <= self.master_count {
            root.add_child(master_side);
            return root;
        }

        let mut stack_side_args = args.clone();
        stack_side_args.window_count = win_count - u32::min(self.master_count, win_count);
        let stack_side = line.layout(stack_side_args);
        stack_side.set_traversal_index(stack_tv_idx);
        stack_side.set_size_proportion((1.0 - master_factor) * 10.0);

        match self.master_side {
            MasterSide::Left | MasterSide::Top => {
                root.set_children([master_side, stack_side]);
            }
            MasterSide::Right | MasterSide::Bottom => {
                root.set_children([stack_side, master_side]);
            }
        }

        root
    }
}

/// A [`LayoutGenerator`] that lays out windows in a shrinking fashion
/// towards the bottom right corner.
#[derive(Clone, Debug, PartialEq)]
pub struct DwindleLayout {
    /// Gaps between windows.
    pub inner_gaps: GapsAll,
    pub outer_gaps: GapsAll,
}

impl Default for DwindleLayout {
    fn default() -> Self {
        Self {
            inner_gaps: 4.0.into(),
            outer_gaps: 4.0.into(),
        }
    }
}

impl LayoutGenerator for DwindleLayout {
    fn layout(&self, args: LayoutArgs) -> LayoutNode {
        let win_count = args.window_count;

        let root = LayoutNode::new_with_label("builtin.dwindle");
        root.set_gaps(self.outer_gaps);

        if win_count == 0 {
            return root;
        }

        if win_count == 1 {
            let child = LayoutNode::new();
            child.set_gaps(self.inner_gaps);
            root.add_child(child);
            return root;
        }

        let mut current_node = root.clone();

        for i in 0..win_count - 1 {
            if current_node != root {
                current_node.set_label(Some("builtin.dwindle.split"));
                current_node.set_gaps(0.0);
            }

            let child1 = LayoutNode::new_with_traversal_index(0);
            child1.set_dir(match i % 2 == 0 {
                true => LayoutDir::Column,
                false => LayoutDir::Row,
            });
            child1.set_gaps(self.inner_gaps);
            current_node.add_child(child1);

            let child2 = LayoutNode::new_with_traversal_index(1);
            child2.set_dir(match i % 2 == 0 {
                true => LayoutDir::Column,
                false => LayoutDir::Row,
            });
            child2.set_gaps(self.inner_gaps);
            current_node.add_child(child2.clone());

            current_node = child2;
        }

        root
    }
}
