//! Various builtin generators.

use std::collections::HashMap;

use crate::{tag::TagHandle, util::Axis};

use super::{Gaps, LayoutDir, LayoutGenerator, LayoutNode};

/// A [`LayoutGenerator`] that lays out windows in a line.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// The gaps between the outer container and this layout.
    pub outer_gaps: Gaps,
    /// The gaps between windows within this layout.
    pub inner_gaps: Gaps,
    /// THe direction the windows should be laid out.
    pub direction: LayoutDir,
    /// Whether or not windows are inserted backwards.
    pub reversed: bool,
}

impl LayoutGenerator for Line {
    fn layout(&self, window_count: u32) -> LayoutNode {
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
pub struct MasterStack {
    /// The gaps between the outer container and this layout.
    pub outer_gaps: Gaps,
    /// The gaps between windows within this layout.
    pub inner_gaps: Gaps,
    /// The proportion of the output the master area will take up.
    ///
    /// This will be clamped between 0.1 and 0.9.
    pub master_factor: f32,
    /// Which side the master area will be.
    pub master_side: MasterSide,
    /// How many windows will be in the master area.
    pub master_count: u32,
    /// Reverses the direction of window insertion i.e. new windows
    /// are inserted at the top of the master stack instead of at the
    /// bottom of the side stack.
    pub reversed: bool,
}

impl Default for MasterStack {
    fn default() -> Self {
        Self {
            outer_gaps: Gaps::from(4.0),
            inner_gaps: Gaps::from(4.0),
            master_factor: 0.5,
            master_side: MasterSide::Left,
            master_count: 1,
            reversed: false,
        }
    }
}

impl LayoutGenerator for MasterStack {
    fn layout(&self, window_count: u32) -> LayoutNode {
        let root = LayoutNode::new_with_label("builtin.master_stack");
        root.set_gaps(self.outer_gaps);
        root.set_dir(match self.master_side {
            MasterSide::Left | MasterSide::Right => LayoutDir::Row,
            MasterSide::Top | MasterSide::Bottom => LayoutDir::Column,
        });

        if window_count == 0 {
            return root;
        }

        let master_factor = self.master_factor.clamp(0.1, 0.9);

        let (master_tv_idx, stack_tv_idx) = match self.reversed {
            true => (1, 0),
            false => (0, 1),
        };

        let master_count = u32::min(self.master_count, window_count);

        let line = Line {
            outer_gaps: 0.0.into(),
            inner_gaps: self.inner_gaps,
            direction: match self.master_side {
                MasterSide::Left | MasterSide::Right => LayoutDir::Column,
                MasterSide::Top | MasterSide::Bottom => LayoutDir::Row,
            },
            reversed: self.reversed,
        };

        let master_side = line.layout(master_count);
        master_side.set_traversal_index(master_tv_idx);
        master_side.set_size_proportion(master_factor * 10.0);

        if window_count <= self.master_count {
            root.add_child(master_side);
            return root;
        }

        let stack_count = window_count - u32::min(self.master_count, window_count);
        let stack_side = line.layout(stack_count);
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
pub struct Dwindle {
    /// The gaps between the outer container and this layout.
    pub outer_gaps: Gaps,
    /// The gaps between windows within this layout.
    pub inner_gaps: Gaps,
}

impl Default for Dwindle {
    fn default() -> Self {
        Self {
            inner_gaps: 4.0.into(),
            outer_gaps: 4.0.into(),
        }
    }
}

impl LayoutGenerator for Dwindle {
    fn layout(&self, win_count: u32) -> LayoutNode {
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

/// A [`LayoutGenerator`] that lays out windows in a spiral.
///
/// This is similar to the [`Dwindle`] layout but in a spiral instead of
/// towards the bottom right corner.
#[derive(Clone, Debug, PartialEq)]
pub struct Spiral {
    /// The gaps between the outer container and this layout.
    pub outer_gaps: Gaps,
    /// The gaps between windows within this layout.
    pub inner_gaps: Gaps,
}

impl Default for Spiral {
    fn default() -> Self {
        Self {
            inner_gaps: 4.0.into(),
            outer_gaps: 4.0.into(),
        }
    }
}

impl LayoutGenerator for Spiral {
    fn layout(&self, win_count: u32) -> LayoutNode {
        let root = LayoutNode::new_with_label("builtin.spiral");
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
                current_node.set_label(Some("builtin.spiral.split"));
                current_node.set_gaps(0.0);
            }

            let child1 = LayoutNode::new_with_traversal_index(0);
            child1.set_dir(match i % 2 == 0 {
                true => LayoutDir::Column,
                false => LayoutDir::Row,
            });
            child1.set_gaps(self.inner_gaps);
            current_node.add_child(child1.clone());

            let child2 = LayoutNode::new_with_traversal_index(1);
            child2.set_dir(match i % 2 == 0 {
                true => LayoutDir::Column,
                false => LayoutDir::Row,
            });
            child2.set_gaps(self.inner_gaps);
            current_node.add_child(child2.clone());

            current_node = match i % 4 {
                0 | 1 => child2,
                2 | 3 => child1,
                _ => unreachable!(),
            };
        }

        root
    }
}

/// Which corner the corner window will in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CornerLocation {
    /// The corner window will be in the top left.
    TopLeft,
    /// The corner window will be in the top right.
    TopRight,
    /// The corner window will be in the bottom left.
    BottomLeft,
    /// The corner window will be in the bottom right.
    BottomRight,
}

/// A [`LayoutGenerator`] that has one main corner window and a
/// horizontal and vertical stack flanking it on the other two sides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Corner {
    /// The gaps between the outer container and this layout.
    pub outer_gaps: Gaps,
    /// The gaps between windows within this layout.
    pub inner_gaps: Gaps,
    /// The proportion of the output that the width of the window takes up.
    pub corner_width_factor: f32,
    /// The proportion of the output that the height of the window takes up.
    pub corner_height_factor: f32,
    /// The location of the corner window.
    pub corner_loc: CornerLocation,
}

impl Default for Corner {
    fn default() -> Self {
        Self {
            inner_gaps: 4.0.into(),
            outer_gaps: 4.0.into(),
            corner_width_factor: 0.5,
            corner_height_factor: 0.5,
            corner_loc: CornerLocation::TopLeft,
        }
    }
}

impl LayoutGenerator for Corner {
    fn layout(&self, win_count: u32) -> LayoutNode {
        let root = LayoutNode::new_with_label("builtin.corner");
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

        let corner_width_factor = self.corner_width_factor.clamp(0.1, 0.9);
        let corner_height_factor = self.corner_height_factor.clamp(0.1, 0.9);

        let corner_and_horiz_stack_node =
            LayoutNode::new_with_label_and_index("builtin.corner.corner_and_stack", 0);
        corner_and_horiz_stack_node.set_dir(LayoutDir::Column);
        corner_and_horiz_stack_node.set_size_proportion(corner_width_factor * 10.0);

        let vert_count = (win_count - 1).div_ceil(2);
        let horiz_count = (win_count - 1) / 2;

        let vert_stack = Line {
            outer_gaps: 0.0.into(),
            inner_gaps: self.inner_gaps,
            direction: LayoutDir::Column,
            reversed: false,
        };

        let vert_stack_node = vert_stack.layout(vert_count);
        vert_stack_node.set_size_proportion((1.0 - corner_width_factor) * 10.0);
        vert_stack_node.set_traversal_index(1);

        root.set_children(match self.corner_loc {
            CornerLocation::TopLeft | CornerLocation::BottomLeft => {
                [corner_and_horiz_stack_node.clone(), vert_stack_node.clone()]
            }
            CornerLocation::TopRight | CornerLocation::BottomRight => {
                [vert_stack_node.clone(), corner_and_horiz_stack_node.clone()]
            }
        });

        if horiz_count == 0 {
            corner_and_horiz_stack_node.set_gaps(self.inner_gaps);
            return root;
        }

        let corner_node = LayoutNode::new_with_traversal_index(0);
        corner_node.set_size_proportion(corner_height_factor * 10.0);
        corner_node.set_gaps(self.inner_gaps);

        let horiz_stack = Line {
            outer_gaps: 0.0.into(),
            inner_gaps: self.inner_gaps,
            direction: LayoutDir::Row,
            reversed: false,
        };

        let horiz_stack_node = horiz_stack.layout(horiz_count);
        horiz_stack_node.set_size_proportion((1.0 - corner_height_factor) * 10.0);
        horiz_stack_node.set_traversal_index(1);

        corner_and_horiz_stack_node.set_children(match self.corner_loc {
            CornerLocation::TopLeft | CornerLocation::TopRight => {
                [corner_node, horiz_stack_node.clone()]
            }
            CornerLocation::BottomLeft | CornerLocation::BottomRight => {
                [horiz_stack_node.clone(), corner_node]
            }
        });

        let traversal_overrides = (0..win_count).map(|idx| (idx, vec![(idx % 2 == 1) as u32]));

        root.set_traversal_overrides(traversal_overrides);

        root
    }
}

/// A [`LayoutGenerator`] that attempts to layout windows such that
/// they are the same size.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Fair {
    /// The gaps between the outer container and this layout.
    pub outer_gaps: Gaps,
    /// The gaps between windows within this layout.
    pub inner_gaps: Gaps,
    /// Which axis the lines of windows will run.
    pub axis: Axis,
}

impl Default for Fair {
    fn default() -> Self {
        Self {
            inner_gaps: 4.0.into(),
            outer_gaps: 4.0.into(),
            axis: Axis::Vertical,
        }
    }
}

impl LayoutGenerator for Fair {
    fn layout(&self, win_count: u32) -> LayoutNode {
        let root = LayoutNode::new_with_label("builtin.fair");
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

        if win_count == 2 {
            let child = LayoutNode::new();
            child.set_gaps(self.inner_gaps);
            root.add_child(child);
            let child2 = LayoutNode::new();
            child2.set_gaps(self.inner_gaps);
            root.add_child(child2);
            return root;
        }

        let line_count = (win_count as f32).sqrt().round() as u32;

        let mut wins_per_line = Vec::new();

        let max_per_line = if win_count > line_count * line_count {
            line_count + 1
        } else {
            line_count
        };

        for i in 1..=win_count {
            let index = (i as f32 / max_per_line as f32).ceil() as usize - 1;
            if wins_per_line.get(index).is_none() {
                wins_per_line.push(0);
            }
            wins_per_line[index] += 1;
        }

        let line = Line {
            outer_gaps: 0.0.into(),
            inner_gaps: self.inner_gaps,
            direction: match self.axis {
                Axis::Horizontal => LayoutDir::Row,
                Axis::Vertical => LayoutDir::Column,
            },
            reversed: false,
        };

        let lines = wins_per_line.into_iter().map(|win_ct| line.layout(win_ct));

        root.set_children(lines);

        root.set_dir(match self.axis {
            Axis::Horizontal => LayoutDir::Column,
            Axis::Vertical => LayoutDir::Row,
        });

        root
    }
}

/// A [`LayoutGenerator`] that keeps track of layouts per tag and provides
/// methods to cycle between them.
pub struct Cycle<T> {
    /// The layouts this generator will cycle between.
    pub layouts: Vec<T>,
    tag_indices: HashMap<u32, usize>,
    current_tag: Option<TagHandle>,
}

impl<T: LayoutGenerator + ?Sized> LayoutGenerator for Box<T> {
    fn layout(&self, window_count: u32) -> LayoutNode {
        (**self).layout(window_count)
    }
}

impl<T: LayoutGenerator + ?Sized> LayoutGenerator for std::sync::Arc<T> {
    fn layout(&self, window_count: u32) -> LayoutNode {
        (**self).layout(window_count)
    }
}

impl<T: LayoutGenerator + ?Sized> LayoutGenerator for std::rc::Rc<T> {
    fn layout(&self, window_count: u32) -> LayoutNode {
        (**self).layout(window_count)
    }
}

impl<T: LayoutGenerator> Cycle<T> {
    /// Creates a new [`Cycle`] from the given [`LayoutGenerator`]s.
    ///
    /// # Examples
    ///
    /// ```
    /// let cycling_layout_manager = CyclingLayoutManager::new([
    ///     Box::<MasterStackLayout>::default() as Box<dyn LayoutGenerator + Send>,
    ///     Box::<DwindleLayout>::default() as _,
    ///     Box::<CornerLayout>::default() as _,
    /// ]);
    /// ```
    pub fn new(layouts: impl IntoIterator<Item = T>) -> Self {
        Self {
            layouts: layouts.into_iter().collect(),
            tag_indices: HashMap::default(),
            current_tag: None,
        }
    }

    /// Cycles the layout forward on the given tag.
    pub fn cycle_layout_forward(&mut self, tag: &TagHandle) {
        let index = self.tag_indices.entry(tag.id).or_default();
        *index += 1;
        if *index >= self.layouts.len() {
            *index = 0;
        }
    }

    /// Cycles the layout backward on the given tag.
    pub fn cycle_layout_backward(&mut self, tag: &TagHandle) {
        let index = self.tag_indices.entry(tag.id).or_default();
        if let Some(i) = index.checked_sub(1) {
            *index = i;
        } else {
            *index = self.layouts.len().saturating_sub(1);
        }
    }

    /// Retrieves the current layout.
    ///
    /// Returns `None` if no layouts were given.
    pub fn current_layout(&self, tag: &TagHandle) -> Option<&T> {
        self.layouts
            .get(self.tag_indices.get(&tag.id).copied().unwrap_or_default())
    }

    /// Sets the current tag to choose a layout for.
    pub fn set_current_tag(&mut self, tag: TagHandle) {
        self.current_tag = Some(tag);
    }
}

impl<T: LayoutGenerator> LayoutGenerator for Cycle<T> {
    fn layout(&self, window_count: u32) -> LayoutNode {
        let Some(current_tag) = self.current_tag.as_ref() else {
            return LayoutNode::new();
        };
        let Some(current_layout) = self.current_layout(current_tag) else {
            return LayoutNode::new();
        };
        current_layout.layout(window_count)
    }
}
