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
    v1::{layout_request, LayoutRequest, TraversalOverrides},
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
            let tree = manager
                .lock()
                .unwrap()
                .active_layout(&args)
                .layout(args.window_count);
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
    traversal_overrides: HashMap<u32, Vec<u32>>,
    style: Style,
    children: Vec<LayoutNode>,
}

impl LayoutNodeInner {
    fn new(label: Option<String>, traversal_index: u32) -> Self {
        LayoutNodeInner {
            label,
            traversal_index,
            traversal_overrides: Default::default(),
            style: Style {
                layout_dir: LayoutDir::Row,
                gaps: Gaps::default(),
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

    pub fn set_traversal_overrides(&self, overrides: impl IntoIterator<Item = (u32, Vec<u32>)>) {
        self.inner.borrow_mut().traversal_overrides = overrides.into_iter().collect();
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

    pub fn set_gaps(&self, gaps: impl Into<Gaps>) {
        self.inner.borrow_mut().style.gaps = gaps.into();
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LayoutDir {
    Row,
    Column,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Gaps {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Gaps {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn uniform(gaps: f32) -> Self {
        gaps.into()
    }
}

impl From<f32> for Gaps {
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
    gaps: Gaps,
    size_proportion: f32,
}

impl From<LayoutNode> for layout::v1::LayoutNode {
    fn from(value: LayoutNode) -> Self {
        fn api_node_from_layout_node(node: LayoutNode) -> layout::v1::LayoutNode {
            let style = node.inner.borrow().style.clone();

            layout::v1::LayoutNode {
                label: node.inner.borrow().label.clone(),
                traversal_overrides: node
                    .inner
                    .borrow()
                    .traversal_overrides
                    .iter()
                    .map(|(idx, overrides)| {
                        (
                            *idx,
                            TraversalOverrides {
                                overrides: overrides.clone(),
                            },
                        )
                    })
                    .collect(),
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
    fn layout(&self, window_count: u32) -> LayoutNode;
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
