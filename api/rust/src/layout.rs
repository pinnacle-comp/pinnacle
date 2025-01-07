// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Layout management.
//!
//! TODO: finish this documentation

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    },
};

use pinnacle_api_defs::pinnacle::layout::v1::{layout_request, LayoutRequest};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio_stream::StreamExt;
use tracing::debug;

use crate::{client::Client, output::OutputHandle, tag::TagHandle, util::Axis, BlockOnTokio};

/// Consume the given [`LayoutManager`] and set it as the global layout handler.
///
/// This returns a [`LayoutRequester`] that allows you to manually request layouts from
/// the compositor. The requester also contains your layout manager wrapped in an `Arc<Mutex>`
/// to allow you to mutate its settings.
pub fn set_manager<M>(manager: M) -> LayoutRequester<M>
where
    M: LayoutManager + Send + 'static,
{
    let (from_client, to_server) = unbounded_channel::<LayoutRequest>();
    let to_server_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(to_server);
    let mut from_server = Client::layout()
        .layout(to_server_stream)
        .block_on_tokio()
        .unwrap()
        .into_inner();

    let from_client_clone = from_client.clone();

    let manager = Arc::new(Mutex::new(manager));

    let requester = LayoutRequester {
        sender: from_client_clone,
        manager: manager.clone(),
    };

    let fut = async move {
        while let Some(Ok(response)) = from_server.next().await {
            let args = LayoutArgs {
                output: OutputHandle {
                    name: response.output_name.clone(),
                },
                window_count: response.window_count,
                tags: response
                    .tag_ids
                    .into_iter()
                    .map(|id| TagHandle { id })
                    .collect(),
            };
            let tree = manager.lock().unwrap().active_layout(&args).layout(&args);
            if from_client
                .send(LayoutRequest {
                    request: Some(layout_request::Request::TreeResponse(
                        layout_request::TreeResponse {
                            request_id: response.request_id,
                            tree: Some(pinnacle_api_defs::pinnacle::layout::v1::LayoutTree {
                                tree_id: tree.tree_id,
                                root: tree.root.map(|root| root.into()),
                                inner_gaps: tree.inner_gaps,
                                outer_gaps: tree.outer_gaps,
                            }),
                            output_name: response.output_name,
                        },
                    )),
                })
                .is_err()
            {
                debug!("Failed to send layout geometries: channel closed");
            }
        }
    };

    tokio::spawn(fut);
    requester
}

#[derive(Debug, Clone)]
pub struct LayoutNode {
    id_ctr: Arc<AtomicU32>,
    inner: Rc<RefCell<LayoutNodeInner>>,
}

#[derive(Debug)]
struct LayoutNodeInner {
    node_id: u32,
    style: Style,
    children: Vec<LayoutNode>,
}

impl LayoutNodeInner {
    fn new(id: u32) -> Self {
        LayoutNodeInner {
            node_id: id,
            style: Style {
                layout_dir: LayoutDir::Row,
                size_proportion: 1.0,
            },
            children: Vec::new(),
        }
    }
}

impl LayoutNode {
    pub fn add_child(&self, child: Self) {
        self.inner.borrow_mut().children.push(child);
    }

    pub fn set_children(&self, children: impl IntoIterator<Item = Self>) {
        self.inner.borrow_mut().children.extend(children);
    }

    pub fn dir(&self, dir: LayoutDir) -> &Self {
        self.inner.borrow_mut().style.layout_dir = dir;
        self
    }

    pub fn size_proportion(&self, proportion: f32) -> &Self {
        self.inner.borrow_mut().style.size_proportion = proportion;
        self
    }
}

#[derive(Default, Debug)]
pub struct LayoutTree {
    tree_id: u32,
    inner_gaps: f32,
    outer_gaps: f32,
    id_ctr: Arc<AtomicU32>,
    root: Option<LayoutNode>,
}

impl LayoutTree {
    pub fn new(tree_id: u32) -> Self {
        Self {
            tree_id,
            ..Default::default()
        }
    }

    pub fn with_gaps(mut self, gaps: Gaps) -> Self {
        let (inner, outer) = gaps.to_inner_outer();
        self.inner_gaps = inner;
        self.outer_gaps = outer;
        self
    }

    pub fn new_node(&self) -> LayoutNode {
        LayoutNode {
            id_ctr: self.id_ctr.clone(),
            inner: Rc::new(RefCell::new(LayoutNodeInner::new(
                self.id_ctr.fetch_add(1, Ordering::Relaxed),
            ))),
        }
    }

    /// Creates and returns the root layout node.
    pub fn set_root(&mut self, node: LayoutNode) {
        self.root.replace(node);
    }
}

#[derive(Debug)]
pub enum LayoutDir {
    Row,
    Column,
}

#[derive(Debug)]
pub struct Style {
    layout_dir: LayoutDir,
    size_proportion: f32,
}

impl From<LayoutNode> for pinnacle_api_defs::pinnacle::layout::v1::LayoutNode {
    fn from(value: LayoutNode) -> Self {
        fn api_node_from_layout_node(
            node: LayoutNode,
        ) -> pinnacle_api_defs::pinnacle::layout::v1::LayoutNode {
            pinnacle_api_defs::pinnacle::layout::v1::LayoutNode {
                node_id: node.inner.borrow().node_id,
                style: Some(pinnacle_api_defs::pinnacle::layout::v1::NodeStyle {
                    flex_dir: match node.inner.borrow().style.layout_dir {
                        LayoutDir::Row => pinnacle_api_defs::pinnacle::layout::v1::FlexDir::Row,
                        LayoutDir::Column => {
                            pinnacle_api_defs::pinnacle::layout::v1::FlexDir::Column
                        }
                    }
                    .into(),
                    size_proportion: node.inner.borrow().style.size_proportion,
                }),
                children: node
                    .inner
                    .borrow()
                    .children
                    .iter()
                    .map(|node| api_node_from_layout_node(node.clone()))
                    .collect(),
            }
        }
        api_node_from_layout_node(value)
    }
}

/// Arguments that [`LayoutGenerator`]s receive when a layout is requested.
#[derive(Clone, Debug)]
pub struct LayoutArgs {
    /// The output that is being laid out.
    pub output: OutputHandle,
    /// The number of windows being laid out.
    pub window_count: u32,
    /// The *focused* tags on the output.
    pub tags: Vec<TagHandle>,
}

/// Types that can manage layouts.
pub trait LayoutManager {
    /// Get the currently active layout for layouting.
    fn active_layout(&mut self, args: &LayoutArgs) -> &dyn LayoutGenerator;
}

/// Types that can generate layouts by computing a vector of [geometries][Geometry].
pub trait LayoutGenerator {
    /// Generate a vector of [geometries][Geometry] using the given [`LayoutArgs`].
    fn layout(&self, args: &LayoutArgs) -> LayoutTree;
}

/// Gaps between windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gaps {
    /// An absolute amount of pixels between windows and the edge of the output.
    ///
    /// For example, `Gaps::Absolute(8)` means there will be 8 pixels between each window
    /// and between the edge of the output.
    Absolute(u32),
    /// A split amount of pixels between windows and the edge of the output.
    Split {
        /// The amount of gap in pixels around *each* window.
        ///
        /// For example, `Gaps::Split { inner: 2, ... }` means there will be
        /// 4 pixels between windows, 2 around each window.
        inner: u32,
        /// The amount of gap in pixels inset from the edge of the output.
        outer: u32,
    },
}

impl Gaps {
    fn to_inner_outer(self) -> (f32, f32) {
        match self {
            Gaps::Absolute(abs) => (abs as f32 / 2.0, abs as f32 / 2.0),
            Gaps::Split { inner, outer } => (inner as f32, outer as f32),
        }
    }
}

/// A [`LayoutManager`] that keeps track of layouts per output and provides
/// methods to cycle between them.
pub struct CyclingLayoutManager {
    layouts: Vec<Box<dyn LayoutGenerator + Send>>,
    tag_indices: HashMap<u32, usize>,
}

impl CyclingLayoutManager {
    /// Create a new [`CyclingLayoutManager`] from the given [`LayoutGenerator`]s.
    ///
    /// `LayoutGenerator`s must be boxed then coerced to trait objects, so you
    /// will need to do an unsizing cast to use them here.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::layout::CyclingLayoutManager;
    /// use pinnacle_api::layout::{MasterStackLayout, DwindleLayout, CornerLayout};
    ///
    /// let cycling_layout_manager = CyclingLayoutManager::new([
    ///     // The `as _` is necessary to coerce to a boxed trait object
    ///     Box::<MasterStackLayout>::default() as _,
    ///     Box::<DwindleLayout>::default() as _,
    ///     Box::<CornerLayout>::default() as _,
    /// ]);
    /// ```
    pub fn new(layouts: impl IntoIterator<Item = Box<dyn LayoutGenerator + Send>>) -> Self {
        Self {
            layouts: layouts.into_iter().collect(),
            tag_indices: HashMap::default(),
        }
    }

    /// Cycle the layout forward on the given tag.
    pub fn cycle_layout_forward(&mut self, tag: &TagHandle) {
        let index = self.tag_indices.entry(tag.id).or_default();
        *index += 1;
        if *index >= self.layouts.len() {
            *index = 0;
        }
    }

    /// Cycle the layout backward on the given tag.
    pub fn cycle_layout_backward(&mut self, tag: &TagHandle) {
        let index = self.tag_indices.entry(tag.id).or_default();
        if let Some(i) = index.checked_sub(1) {
            *index = i;
        } else {
            *index = self.layouts.len().saturating_sub(1);
        }
    }
}

impl LayoutManager for CyclingLayoutManager {
    fn active_layout(&mut self, args: &LayoutArgs) -> &dyn LayoutGenerator {
        let Some(first_tag) = args.tags.first() else {
            return &NoopLayout;
        };

        self.layouts
            .get(*self.tag_indices.entry(first_tag.id).or_default())
            .expect("no layouts in manager")
            .as_ref()
    }
}

/// A struct that can request layouts and provides access to a consumed [`LayoutManager`].
#[derive(Debug)]
pub struct LayoutRequester<T> {
    sender: UnboundedSender<LayoutRequest>,
    /// The manager that was consumed, wrapped in an `Arc<Mutex>`.
    pub manager: Arc<Mutex<T>>,
}

impl<T> Clone for LayoutRequester<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            manager: self.manager.clone(),
        }
    }
}

impl<T> LayoutRequester<T> {
    /// Request a layout from the compositor.
    ///
    /// This uses the focused output for the request.
    /// If you want to layout a specific output, see [`LayoutRequester::request_layout_on_output`].
    pub fn request_layout(&self) {
        let Some(output_name) = crate::output::get_focused().map(|op| op.name) else {
            return;
        };
        if self
            .sender
            .send(LayoutRequest {
                request: Some(layout_request::Request::ForceLayout(
                    layout_request::ForceLayout { output_name },
                )),
            })
            .is_err()
        {
            debug!("Failed to request layout: channel closed");
        }
    }

    /// Request a layout from the compositor for the given output.
    pub fn request_layout_on_output(&self, output: &OutputHandle) {
        if self
            .sender
            .send(LayoutRequest {
                request: Some(layout_request::Request::ForceLayout(
                    layout_request::ForceLayout {
                        output_name: output.name.clone(),
                    },
                )),
            })
            .is_err()
        {
            debug!("Failed to request layout on output: channel closed");
        }
    }
}

impl LayoutRequester<CyclingLayoutManager> {
    /// Cycle the layout forward for the given tag.
    pub fn cycle_layout_forward(&self, tag: &TagHandle) {
        let mut lock = self.manager.lock().unwrap();
        lock.cycle_layout_forward(tag);
    }

    /// Cycle the layout backward for the given tag.
    pub fn cycle_layout_backward(&mut self, tag: &TagHandle) {
        let mut lock = self.manager.lock().unwrap();
        lock.cycle_layout_backward(tag);
    }
}

/// A layout generator that does nothing.
struct NoopLayout;

impl LayoutGenerator for NoopLayout {
    fn layout(&self, _args: &LayoutArgs) -> LayoutTree {
        LayoutTree::default()
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
    ///
    /// Defaults to `Gaps::Absolute(8)`.
    pub gaps: Gaps,
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
}

impl Default for MasterStackLayout {
    fn default() -> Self {
        Self {
            gaps: Gaps::Absolute(8),
            master_factor: 0.5,
            master_side: MasterSide::Left,
            master_count: 1,
        }
    }
}

impl LayoutGenerator for MasterStackLayout {
    fn layout(&self, args: &LayoutArgs) -> LayoutTree {
        let win_count = args.window_count;

        if win_count == 0 {
            return LayoutTree::default();
        }

        let mut tree = LayoutTree::new(0).with_gaps(self.gaps);
        let root = tree.new_node();
        root.dir(match self.master_side {
            MasterSide::Left | MasterSide::Right => LayoutDir::Row,
            MasterSide::Top | MasterSide::Bottom => LayoutDir::Column,
        });

        tree.set_root(root.clone());

        let master_factor = self.master_factor.clamp(0.1, 0.9);

        let master_side_node = tree.new_node();

        master_side_node
            .dir(match self.master_side {
                MasterSide::Left | MasterSide::Right => LayoutDir::Column,
                MasterSide::Top | MasterSide::Bottom => LayoutDir::Row,
            })
            .size_proportion(master_factor * 10.0);

        for _ in 0..u32::min(win_count, self.master_count) {
            let child = tree.new_node();
            child.size_proportion(10.0);
            master_side_node.add_child(child);
        }

        let stack_side_node = tree.new_node();
        stack_side_node
            .dir(match self.master_side {
                MasterSide::Left | MasterSide::Right => LayoutDir::Column,
                MasterSide::Top | MasterSide::Bottom => LayoutDir::Row,
            })
            .size_proportion((1.0 - master_factor) * 10.0);

        for _ in self.master_count..win_count {
            let child = tree.new_node();
            child.size_proportion(10.0);
            stack_side_node.add_child(child);
        }

        if win_count <= self.master_count {
            root.set_children([master_side_node]);
            return tree;
        }

        match self.master_side {
            MasterSide::Left | MasterSide::Top => {
                root.set_children([master_side_node, stack_side_node]);
            }
            MasterSide::Right | MasterSide::Bottom => {
                root.set_children([stack_side_node, master_side_node]);
            }
        }

        tree
    }
}

/// A [`LayoutGenerator`] that lays out windows in a shrinking fashion
/// towards the bottom right corner.
#[derive(Clone, Debug, PartialEq)]
pub struct DwindleLayout {
    /// Gaps between windows.
    ///
    /// Defaults to `Gaps::Absolute(8)`.
    pub gaps: Gaps,
    /// The ratio for each dwindle split.
    ///
    /// The first split will use the factor at key `1`,
    /// the second at key `2`, and so on.
    ///
    /// Splits without a factor will default to 0.5.
    pub split_factors: HashMap<usize, f32>,
}

impl Default for DwindleLayout {
    fn default() -> Self {
        Self {
            gaps: Gaps::Absolute(8),
            split_factors: Default::default(),
        }
    }
}

impl LayoutGenerator for DwindleLayout {
    fn layout(&self, args: &LayoutArgs) -> LayoutTree {
        let win_count = args.window_count;

        if win_count == 0 {
            return LayoutTree::default();
        }

        let mut tree = LayoutTree::new(0).with_gaps(self.gaps);
        let root = tree.new_node();
        root.dir(LayoutDir::Row);

        tree.set_root(root.clone());

        if win_count == 1 {
            return tree;
        }

        let windows_left = win_count - 1;

        let mut current_node = root.clone();

        for i in 0..windows_left {
            let child1 = tree.new_node();
            child1.dir(match i % 2 == 0 {
                true => LayoutDir::Column,
                false => LayoutDir::Row,
            });
            current_node.add_child(child1);

            let child2 = tree.new_node();
            child2.dir(match i % 2 == 0 {
                true => LayoutDir::Column,
                false => LayoutDir::Row,
            });
            current_node.add_child(child2.clone());

            current_node = child2;
        }

        tree
    }
}

/// A [`LayoutGenerator`] that lays out windows in a spiral.
///
/// This is similar to the [`DwindleLayout`] but in a spiral instead of
/// towards the bottom right corner.
#[derive(Clone, Debug, PartialEq)]
pub struct SpiralLayout {
    /// Gaps between windows.
    ///
    /// Defaults to `Gaps::Absolute(8)`.
    pub gaps: Gaps,
    /// The ratio for each dwindle split.
    ///
    /// The first split will use the factor at key `1`,
    /// the second at key `2`, and so on.
    ///
    /// Splits without a factor will default to 0.5.
    pub split_factors: HashMap<usize, f32>,
}

impl Default for SpiralLayout {
    fn default() -> Self {
        Self {
            gaps: Gaps::Absolute(8),
            split_factors: Default::default(),
        }
    }
}

impl LayoutGenerator for SpiralLayout {
    fn layout(&self, args: &LayoutArgs) -> LayoutTree {
        let win_count = args.window_count;

        if win_count == 0 {
            return LayoutTree::default();
        }

        let mut tree = LayoutTree::new(0).with_gaps(self.gaps);
        let root = tree.new_node();
        root.dir(LayoutDir::Row);

        tree.set_root(root.clone());

        if win_count == 1 {
            return tree;
        }

        let windows_left = win_count - 1;

        let mut current_node = root;

        for i in 0..windows_left {
            let child1 = tree.new_node();
            child1.dir(match i % 2 == 0 {
                true => LayoutDir::Column,
                false => LayoutDir::Row,
            });
            current_node.add_child(child1.clone());

            let child2 = tree.new_node();
            child2.dir(match i % 2 == 0 {
                true => LayoutDir::Column,
                false => LayoutDir::Row,
            });
            current_node.add_child(child2.clone());

            current_node = match i % 4 {
                0 | 1 => child2,
                2 | 3 => child1,
                _ => unreachable!(),
            };
        }

        tree
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
pub struct CornerLayout {
    /// Gaps between windows.
    ///
    /// Defaults to `Gaps::Absolute(8)`.
    pub gaps: Gaps,
    /// The proportion of the output that the width of the window takes up.
    ///
    /// Defaults to 0.5.
    pub corner_width_factor: f32,
    /// The proportion of the output that the height of the window takes up.
    ///
    /// Defaults to 0.5.
    pub corner_height_factor: f32,
    /// The location of the corner window.
    pub corner_loc: CornerLocation,
}

impl Default for CornerLayout {
    fn default() -> Self {
        Self {
            gaps: Gaps::Absolute(8),
            corner_width_factor: 0.5,
            corner_height_factor: 0.5,
            corner_loc: CornerLocation::TopLeft,
        }
    }
}

impl LayoutGenerator for CornerLayout {
    fn layout(&self, args: &LayoutArgs) -> LayoutTree {
        let win_count = args.window_count;

        if win_count == 0 {
            return LayoutTree::default();
        }

        let mut tree = LayoutTree::new(0).with_gaps(self.gaps);
        let root = tree.new_node();
        root.dir(LayoutDir::Row);

        tree.set_root(root.clone());

        if win_count == 1 {
            return tree;
        }

        let corner_width_factor = self.corner_width_factor.clamp(0.1, 0.9);
        let corner_height_factor = self.corner_height_factor.clamp(0.1, 0.9);

        let corner_and_horiz_stack_node = tree.new_node();
        corner_and_horiz_stack_node
            .dir(LayoutDir::Column)
            .size_proportion(corner_width_factor * 10.0);

        let vert_stack_node = tree.new_node();
        vert_stack_node
            .dir(LayoutDir::Column)
            .size_proportion((1.0 - corner_width_factor) * 10.0);

        root.set_children(match self.corner_loc {
            CornerLocation::TopLeft | CornerLocation::BottomLeft => {
                [corner_and_horiz_stack_node.clone(), vert_stack_node.clone()]
            }
            CornerLocation::TopRight | CornerLocation::BottomRight => {
                [vert_stack_node.clone(), corner_and_horiz_stack_node.clone()]
            }
        });

        if win_count == 2 {
            return tree;
        }

        let corner_node = tree.new_node();
        corner_node.size_proportion(corner_height_factor * 10.0);

        let horiz_stack_node = tree.new_node();
        horiz_stack_node
            .dir(LayoutDir::Row)
            .size_proportion((1.0 - corner_height_factor) * 10.0);

        corner_and_horiz_stack_node.set_children(match self.corner_loc {
            CornerLocation::TopLeft | CornerLocation::TopRight => {
                [corner_node, horiz_stack_node.clone()]
            }
            CornerLocation::BottomLeft | CornerLocation::BottomRight => {
                [horiz_stack_node.clone(), corner_node]
            }
        });

        for i in 0..win_count - 1 {
            if i % 2 == 0 {
                let child = tree.new_node();
                vert_stack_node.add_child(child);
            } else {
                let child = tree.new_node();
                horiz_stack_node.add_child(child);
            }
        }

        tree
    }
}

/// A [`LayoutGenerator`] that attempts to layout windows such that
/// they are the same size.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FairLayout {
    /// The proportion of the output that the width of the window takes up.
    ///
    /// Defaults to 0.5.
    pub gaps: Gaps,
    /// Which axis the lines of windows will run.
    ///
    /// Defaults to [`Axis::Vertical`].
    pub axis: Axis,
}

impl Default for FairLayout {
    fn default() -> Self {
        Self {
            gaps: Gaps::Absolute(8),
            axis: Axis::Vertical,
        }
    }
}

impl LayoutGenerator for FairLayout {
    fn layout(&self, args: &LayoutArgs) -> LayoutTree {
        let win_count = args.window_count;

        if win_count == 0 {
            return LayoutTree::default();
        }

        let mut tree = LayoutTree::new(0).with_gaps(self.gaps);
        let root = tree.new_node();
        root.dir(match self.axis {
            Axis::Horizontal => LayoutDir::Column,
            Axis::Vertical => LayoutDir::Row,
        });

        tree.set_root(root.clone());

        if win_count == 1 {
            return tree;
        }

        if win_count == 2 {
            let child1 = tree.new_node();
            let child2 = tree.new_node();
            root.set_children([child1, child2]);
            return tree;
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

        let lines = wins_per_line.into_iter().map(|win_ct| {
            let line_root = tree.new_node();
            line_root.dir(match self.axis {
                Axis::Horizontal => LayoutDir::Row,
                Axis::Vertical => LayoutDir::Column,
            });

            for _ in 0..win_ct {
                let child = tree.new_node();
                line_root.add_child(child);
            }

            line_root
        });

        root.set_children(lines);

        tree
    }
}
//     fn layout(&self, args: &LayoutArgs) -> Vec<Geometry> {
//         let win_count = args.windows.len() as u32;
//
//         if win_count == 0 {
//             return Vec::new();
//         }
//
//         let width = args.output_width;
//         let height = args.output_height;
//
//         let mut geos = Vec::<Geometry>::new();
//
//         let (outer_gaps, inner_gaps) = match self.gaps {
//             Gaps::Absolute(gaps) => (gaps, None),
//             Gaps::Split { inner, outer } => (outer, Some(inner)),
//         };
//
//         let gaps = match inner_gaps {
//             Some(_) => 0,
//             None => outer_gaps,
//         };
//
//         let mut rect = Geometry {
//             x: 0,
//             y: 0,
//             width,
//             height,
//         }
//         .split_at(Axis::Horizontal, 0, outer_gaps)
//         .0
//         .split_at(Axis::Horizontal, (height - outer_gaps) as i32, outer_gaps)
//         .0
//         .split_at(Axis::Vertical, 0, outer_gaps)
//         .0
//         .split_at(Axis::Vertical, (width - outer_gaps) as i32, outer_gaps)
//         .0;
//
//         if win_count == 1 {
//             geos.push(rect);
//         } else if win_count == 2 {
//             let len = match self.axis {
//                 Axis::Vertical => rect.width,
//                 Axis::Horizontal => rect.height,
//             };
//
//             let coord = match self.axis {
//                 Axis::Vertical => rect.x,
//                 Axis::Horizontal => rect.y,
//             };
//
//             let (rect1, rect2) =
//                 rect.split_at(self.axis, coord + len as i32 / 2 - gaps as i32 / 2, gaps);
//
//             geos.push(rect1);
//             if let Some(rect2) = rect2 {
//                 geos.push(rect2);
//             }
//         } else {
//             let line_count = (win_count as f32).sqrt().round() as u32;
//
//             let mut wins_per_line = Vec::new();
//
//             let max_per_line = if win_count > line_count * line_count {
//                 line_count + 1
//             } else {
//                 line_count
//             };
//
//             for i in 1..=win_count {
//                 let index = (i as f32 / max_per_line as f32).ceil() as usize - 1;
//                 if wins_per_line.get(index).is_none() {
//                     wins_per_line.push(0);
//                 }
//                 wins_per_line[index] += 1;
//             }
//
//             assert_eq!(wins_per_line.len(), line_count as usize);
//
//             let mut line_rects = Vec::new();
//
//             let (coord, len, axis) = match self.axis {
//                 Axis::Horizontal => (
//                     rect.y,
//                     rect.height as f32 / line_count as f32,
//                     Axis::Horizontal,
//                 ),
//                 Axis::Vertical => (
//                     rect.x,
//                     rect.width as f32 / line_count as f32,
//                     Axis::Vertical,
//                 ),
//             };
//
//             for i in 1..line_count {
//                 let slice_point = coord + (len * i as f32) as i32 - gaps as i32 / 2;
//                 let (to_push, rest) = rect.split_at(axis, slice_point, gaps);
//                 line_rects.push(to_push);
//                 if let Some(rest) = rest {
//                     rect = rest;
//                 } else {
//                     break;
//                 }
//             }
//
//             line_rects.push(rect);
//
//             for (i, mut line_rect) in line_rects.into_iter().enumerate() {
//                 let (coord, len, axis) = match self.axis {
//                     Axis::Vertical => (
//                         line_rect.y,
//                         line_rect.height as f32 / wins_per_line[i] as f32,
//                         Axis::Horizontal,
//                     ),
//                     Axis::Horizontal => (
//                         line_rect.x,
//                         line_rect.width as f32 / wins_per_line[i] as f32,
//                         Axis::Vertical,
//                     ),
//                 };
//
//                 for j in 1..wins_per_line[i] {
//                     let slice_point = coord + (len * j as f32) as i32 - gaps as i32 / 2;
//                     let (to_push, rest) = line_rect.split_at(axis, slice_point, gaps);
//                     geos.push(to_push);
//                     if let Some(rest) = rest {
//                         line_rect = rest;
//                     } else {
//                         break;
//                     }
//                 }
//
//                 geos.push(line_rect);
//             }
//         }
//
//         if let Some(inner_gaps) = inner_gaps {
//             for geo in geos.iter_mut() {
//                 geo.x += inner_gaps as i32;
//                 geo.y += inner_gaps as i32;
//                 geo.width -= inner_gaps * 2;
//                 geo.height -= inner_gaps * 2;
//             }
//         }
//
//         geos
//     }
// }
