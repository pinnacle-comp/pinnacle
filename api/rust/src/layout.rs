// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Layout management.
//!
//! TODO: finish this documentation

pub mod generator;

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex},
};

use pinnacle_api_defs::pinnacle::layout::{
    self,
    v1::{layout_request, LayoutRequest},
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio_stream::StreamExt;

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
            let tree = manager.lock().unwrap().active_layout(&args).layout(args);
            from_client
                .send(LayoutRequest {
                    request: Some(layout_request::Request::TreeResponse(
                        layout_request::TreeResponse {
                            tree_id: 0, // TODO:
                            request_id: response.request_id,
                            output_name: response.output_name,
                            root_node: Some(tree.into()),
                        },
                    )),
                })
                .unwrap();
        }
    };

    tokio::spawn(fut);
    requester
}

#[derive(Debug, Clone)]
pub struct LayoutNode {
    inner: Rc<RefCell<LayoutNodeInner>>,
}

impl PartialEq for LayoutNode {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

#[derive(Debug, Clone)]
struct LayoutNodeInner {
    label: Option<String>,
    traversal_index: u32,
    style: Style,
    children: Vec<LayoutNode>,
}

impl LayoutNodeInner {
    fn new(label: Option<String>, traversal_index: u32) -> Self {
        LayoutNodeInner {
            label,
            traversal_index,
            style: Style {
                layout_dir: LayoutDir::Row,
                gaps: GapsAll::default(),
                size_proportion: 1.0,
            },
            children: Vec::new(),
        }
    }
}

impl LayoutNode {
    pub fn new() -> Self {
        LayoutNode {
            inner: Rc::new(RefCell::new(LayoutNodeInner::new(None, 0))),
        }
    }

    pub fn new_with_label(label: impl ToString) -> Self {
        LayoutNode {
            inner: Rc::new(RefCell::new(LayoutNodeInner::new(
                Some(label.to_string()),
                0,
            ))),
        }
    }

    pub fn new_with_traversal_index(index: u32) -> Self {
        LayoutNode {
            inner: Rc::new(RefCell::new(LayoutNodeInner::new(None, index))),
        }
    }

    pub fn new_with_label_and_index(label: impl ToString, index: u32) -> Self {
        LayoutNode {
            inner: Rc::new(RefCell::new(LayoutNodeInner::new(
                Some(label.to_string()),
                index,
            ))),
        }
    }

    pub fn add_child(&self, child: Self) {
        self.inner.borrow_mut().children.push(child);
    }

    pub fn set_label(&self, label: Option<impl ToString>) {
        self.inner.borrow_mut().label = label.map(|label| label.to_string());
    }

    pub fn set_traversal_index(&self, index: u32) {
        self.inner.borrow_mut().traversal_index = index;
    }

    pub fn set_children(&self, children: impl IntoIterator<Item = Self>) {
        self.inner.borrow_mut().children = children.into_iter().collect();
    }

    pub fn set_dir(&self, dir: LayoutDir) {
        self.inner.borrow_mut().style.layout_dir = dir;
    }

    pub fn set_size_proportion(&self, proportion: f32) {
        self.inner.borrow_mut().style.size_proportion = proportion;
    }

    pub fn set_gaps(&self, gaps: impl Into<GapsAll>) {
        self.inner.borrow_mut().style.gaps = gaps.into();
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LayoutDir {
    Row,
    Column,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GapsAll {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl GapsAll {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn uniform(gaps: f32) -> Self {
        gaps.into()
    }
}

impl From<f32> for GapsAll {
    fn from(value: f32) -> Self {
        Self {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Style {
    layout_dir: LayoutDir,
    gaps: GapsAll,
    size_proportion: f32,
}

impl From<LayoutNode> for layout::v1::LayoutNode {
    fn from(value: LayoutNode) -> Self {
        fn api_node_from_layout_node(node: LayoutNode) -> layout::v1::LayoutNode {
            let style = node.inner.borrow().style.clone();

            layout::v1::LayoutNode {
                label: node.inner.borrow().label.clone(),
                traversal_index: node.inner.borrow().traversal_index,
                style: Some(layout::v1::NodeStyle {
                    flex_dir: match node.inner.borrow().style.layout_dir {
                        LayoutDir::Row => layout::v1::FlexDir::Row,
                        LayoutDir::Column => layout::v1::FlexDir::Column,
                    }
                    .into(),
                    size_proportion: node.inner.borrow().style.size_proportion,
                    gaps: Some(layout::v1::Gaps {
                        left: style.gaps.left,
                        right: style.gaps.right,
                        top: style.gaps.top,
                        bottom: style.gaps.bottom,
                    }),
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
    fn layout(&self, args: LayoutArgs) -> LayoutNode;
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
            panic!();
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
        self.sender
            .send(LayoutRequest {
                request: Some(layout_request::Request::ForceLayout(
                    layout_request::ForceLayout { output_name },
                )),
            })
            .unwrap();
    }

    /// Request a layout from the compositor for the given output.
    pub fn request_layout_on_output(&self, output: &OutputHandle) {
        self.sender
            .send(LayoutRequest {
                request: Some(layout_request::Request::ForceLayout(
                    layout_request::ForceLayout {
                        output_name: output.name.clone(),
                    },
                )),
            })
            .unwrap();
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

// /// A layout generator that does nothing.
// struct NoopLayout;
//
// impl LayoutGenerator for NoopLayout {
//     fn layout(&self, _args: &LayoutArgs) -> LayoutTree {
//         LayoutTree::default()
//     }
// }
//
// /// A [`LayoutGenerator`] that lays out windows in a spiral.
// ///
// /// This is similar to the [`DwindleLayout`] but in a spiral instead of
// /// towards the bottom right corner.
// #[derive(Clone, Debug, PartialEq)]
// pub struct SpiralLayout {
//     /// Gaps between windows.
//     ///
//     /// Defaults to `Gaps::Absolute(8)`.
//     pub gaps: Gaps,
//     /// The ratio for each dwindle split.
//     ///
//     /// The first split will use the factor at key `1`,
//     /// the second at key `2`, and so on.
//     ///
//     /// Splits without a factor will default to 0.5.
//     pub split_factors: HashMap<usize, f32>,
// }
//
// impl Default for SpiralLayout {
//     fn default() -> Self {
//         Self {
//             gaps: Gaps::Absolute(8),
//             split_factors: Default::default(),
//         }
//     }
// }
//
// impl LayoutGenerator for SpiralLayout {
//     fn layout(&self, args: &LayoutArgs) -> LayoutTree {
//         let win_count = args.window_count;
//
//         if win_count == 0 {
//             return LayoutTree::default();
//         }
//
//         let mut tree = LayoutTree::new(0).with_gaps(self.gaps);
//         let root = tree.new_node();
//         root.set_dir(LayoutDir::Row);
//
//         tree.set_root(root.clone());
//
//         if win_count == 1 {
//             return tree;
//         }
//
//         let windows_left = win_count - 1;
//
//         let mut current_node = root;
//
//         for i in 0..windows_left {
//             let child1 = tree.new_node();
//             child1.set_dir(match i % 2 == 0 {
//                 true => LayoutDir::Column,
//                 false => LayoutDir::Row,
//             });
//             current_node.add_child(child1.clone());
//
//             let child2 = tree.new_node();
//             child2.set_dir(match i % 2 == 0 {
//                 true => LayoutDir::Column,
//                 false => LayoutDir::Row,
//             });
//             current_node.add_child(child2.clone());
//
//             current_node = match i % 4 {
//                 0 | 1 => child2,
//                 2 | 3 => child1,
//                 _ => unreachable!(),
//             };
//         }
//
//         tree
//     }
// }
//
// /// Which corner the corner window will in.
// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// pub enum CornerLocation {
//     /// The corner window will be in the top left.
//     TopLeft,
//     /// The corner window will be in the top right.
//     TopRight,
//     /// The corner window will be in the bottom left.
//     BottomLeft,
//     /// The corner window will be in the bottom right.
//     BottomRight,
// }
//
// /// A [`LayoutGenerator`] that has one main corner window and a
// /// horizontal and vertical stack flanking it on the other two sides.
// #[derive(Debug, Clone, Copy, PartialEq)]
// pub struct CornerLayout {
//     /// Gaps between windows.
//     ///
//     /// Defaults to `Gaps::Absolute(8)`.
//     pub gaps: Gaps,
//     /// The proportion of the output that the width of the window takes up.
//     ///
//     /// Defaults to 0.5.
//     pub corner_width_factor: f32,
//     /// The proportion of the output that the height of the window takes up.
//     ///
//     /// Defaults to 0.5.
//     pub corner_height_factor: f32,
//     /// The location of the corner window.
//     pub corner_loc: CornerLocation,
// }
//
// impl Default for CornerLayout {
//     fn default() -> Self {
//         Self {
//             gaps: Gaps::Absolute(8),
//             corner_width_factor: 0.5,
//             corner_height_factor: 0.5,
//             corner_loc: CornerLocation::TopLeft,
//         }
//     }
// }
//
// impl LayoutGenerator for CornerLayout {
//     fn layout(&self, args: &LayoutArgs) -> LayoutTree {
//         let win_count = args.window_count;
//
//         if win_count == 0 {
//             return LayoutTree::default();
//         }
//
//         let mut tree = LayoutTree::new(0).with_gaps(self.gaps);
//         let root = tree.new_node();
//         root.set_dir(LayoutDir::Row);
//
//         tree.set_root(root.clone());
//
//         if win_count == 1 {
//             return tree;
//         }
//
//         let corner_width_factor = self.corner_width_factor.clamp(0.1, 0.9);
//         let corner_height_factor = self.corner_height_factor.clamp(0.1, 0.9);
//
//         let corner_and_horiz_stack_node = tree.new_node();
//         corner_and_horiz_stack_node.set_dir(LayoutDir::Column);
//         corner_and_horiz_stack_node.set_size_proportion(corner_width_factor * 10.0);
//
//         let vert_stack_node = tree.new_node();
//         vert_stack_node.set_dir(LayoutDir::Column);
//         vert_stack_node.set_size_proportion((1.0 - corner_width_factor) * 10.0);
//
//         root.set_children(match self.corner_loc {
//             CornerLocation::TopLeft | CornerLocation::BottomLeft => {
//                 [corner_and_horiz_stack_node.clone(), vert_stack_node.clone()]
//             }
//             CornerLocation::TopRight | CornerLocation::BottomRight => {
//                 [vert_stack_node.clone(), corner_and_horiz_stack_node.clone()]
//             }
//         });
//
//         if win_count == 2 {
//             return tree;
//         }
//
//         let corner_node = tree.new_node();
//         corner_node.set_size_proportion(corner_height_factor * 10.0);
//
//         let horiz_stack_node = tree.new_node();
//         horiz_stack_node.set_dir(LayoutDir::Row);
//         horiz_stack_node.set_size_proportion((1.0 - corner_height_factor) * 10.0);
//
//         corner_and_horiz_stack_node.set_children(match self.corner_loc {
//             CornerLocation::TopLeft | CornerLocation::TopRight => {
//                 [corner_node, horiz_stack_node.clone()]
//             }
//             CornerLocation::BottomLeft | CornerLocation::BottomRight => {
//                 [horiz_stack_node.clone(), corner_node]
//             }
//         });
//
//         for i in 0..win_count - 1 {
//             if i % 2 == 0 {
//                 let child = tree.new_node();
//                 vert_stack_node.add_child(child);
//             } else {
//                 let child = tree.new_node();
//                 horiz_stack_node.add_child(child);
//             }
//         }
//
//         tree
//     }
// }
//
// /// A [`LayoutGenerator`] that attempts to layout windows such that
// /// they are the same size.
// #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
// pub struct FairLayout {
//     /// The proportion of the output that the width of the window takes up.
//     ///
//     /// Defaults to 0.5.
//     pub gaps: Gaps,
//     /// Which axis the lines of windows will run.
//     ///
//     /// Defaults to [`Axis::Vertical`].
//     pub axis: Axis,
// }
//
// impl Default for FairLayout {
//     fn default() -> Self {
//         Self {
//             gaps: Gaps::Absolute(8),
//             axis: Axis::Vertical,
//         }
//     }
// }
//
// impl LayoutGenerator for FairLayout {
//     fn layout(&self, args: &LayoutArgs) -> LayoutTree {
//         let win_count = args.window_count;
//
//         if win_count == 0 {
//             return LayoutTree::default();
//         }
//
//         let mut tree = LayoutTree::new(0).with_gaps(self.gaps);
//         let root = tree.new_node();
//         root.set_dir(match self.axis {
//             Axis::Horizontal => LayoutDir::Column,
//             Axis::Vertical => LayoutDir::Row,
//         });
//
//         tree.set_root(root.clone());
//
//         if win_count == 1 {
//             return tree;
//         }
//
//         if win_count == 2 {
//             let child1 = tree.new_node();
//             let child2 = tree.new_node();
//             root.set_children([child1, child2]);
//             return tree;
//         }
//
//         let line_count = (win_count as f32).sqrt().round() as u32;
//
//         let mut wins_per_line = Vec::new();
//
//         let max_per_line = if win_count > line_count * line_count {
//             line_count + 1
//         } else {
//             line_count
//         };
//
//         for i in 1..=win_count {
//             let index = (i as f32 / max_per_line as f32).ceil() as usize - 1;
//             if wins_per_line.get(index).is_none() {
//                 wins_per_line.push(0);
//             }
//             wins_per_line[index] += 1;
//         }
//
//         let lines = wins_per_line.into_iter().map(|win_ct| {
//             let line_root = tree.new_node();
//             line_root.set_dir(match self.axis {
//                 Axis::Horizontal => LayoutDir::Row,
//                 Axis::Vertical => LayoutDir::Column,
//             });
//
//             for _ in 0..win_ct {
//                 let child = tree.new_node();
//                 line_root.add_child(child);
//             }
//
//             line_root
//         });
//
//         root.set_children(lines);
//
//         tree
//     }
// }
