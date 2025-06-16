// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Layout management.
//!
//! Read the [wiki page](https://pinnacle-comp.github.io/pinnacle/configuration/layout.html)
//! for more information.

pub mod generators;

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use pinnacle_api_defs::pinnacle::layout::{
    self,
    v1::{layout_request, LayoutRequest, TraversalOverrides},
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio_stream::StreamExt;

use crate::{client::Client, output::OutputHandle, tag::TagHandle, BlockOnTokio};

/// A response to a layout request containing a layout tree.
pub struct LayoutResponse {
    /// The root node of the layout tree.
    pub root_node: LayoutNode,
    /// An identifier for the layout tree.
    ///
    /// Trees that are considered "the same", like trees for a certain tag and layout,
    /// should have the same identifier to allow Pinnacle to remember tile sizing.
    pub tree_id: u32,
}

/// Manages layout requests from the compositor.
///
/// You must call this function to begin handling incoming layout requests.
/// Whenever a layout request comes in, `on_layout` will be called with the arguments of the
/// layout. The closure must then return a [`LayoutResponse`] containing the root of a layout tree through a [`LayoutNode`], along with a unique identifier.
///
/// This returns a [`LayoutRequester`] that allows you to force the compositor to emit a
/// layout request.
///
/// See the module level documentation for more information on how to generate layouts.
pub fn manage(
    mut on_layout: impl FnMut(LayoutArgs) -> LayoutResponse + Send + 'static,
) -> LayoutRequester {
    let (from_client, to_server) = unbounded_channel::<LayoutRequest>();
    let to_server_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(to_server);
    let mut from_server = Client::layout()
        .layout(to_server_stream)
        .block_on_tokio()
        .unwrap()
        .into_inner();

    let from_client_clone = from_client.clone();

    let requester = LayoutRequester {
        sender: from_client_clone,
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
            let tree_response = on_layout(args);
            from_client
                .send(LayoutRequest {
                    request: Some(layout_request::Request::TreeResponse(
                        layout_request::TreeResponse {
                            tree_id: tree_response.tree_id,
                            request_id: response.request_id,
                            output_name: response.output_name,
                            root_node: Some(tree_response.root_node.into()),
                        },
                    )),
                })
                .unwrap();
        }
    };

    tokio::spawn(fut);
    requester
}

/// A single node of a layout tree.
///
/// [`LayoutNode`]s allow you to hierarchically represent layouts in a tree structure.
/// They have the following properties:
/// - A layout direction, set with [`set_dir`][Self::set_dir]: This determines the direction
///   that children layout nodes are laid out in.
/// - A size proportion, set with [`set_size_proportion`][Self::set_size_proportion]: This
///   determines the proportion of space a layout node takes up in relation to its siblings.
/// - Gaps, set with [`set_gaps`][Self::set_gaps]: This determines the gaps surrounding a
///   layout node.
/// - A traversal index, set with [`set_traversal_index`][Self::set_traversal_index]: This
///   determines the order that the layout tree is traversed in when assigning layout node
///   geometries to windows.
/// - Traversal overrides, set with [`set_traversal_overrides`][Self::set_traversal_overrides]:
///   This provides a way to provide per-window overrides to tree traversal. This is used to
///   enable otherwise impossible window insertion strategies. For example, the
///   [`Corner`][self::generators::Corner] layout generator overrides traversal to allow
///   windows to be inserted into the vertical and horizontal stacks in an alternating fashion.
/// - An optional label, set with [`set_label`][Self::set_label]: This gives the compositor a hint
///   when diffing layout trees, allowing it to, for example, decide whether to move a node or
///   delete it and insert a new one.
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

impl Default for LayoutNode {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutNode {
    /// Creates a new layout node.
    pub fn new() -> Self {
        LayoutNode {
            inner: Rc::new(RefCell::new(LayoutNodeInner::new(None, 0))),
        }
    }

    /// Creates a new layout node with the given label.
    pub fn new_with_label(label: impl ToString) -> Self {
        LayoutNode {
            inner: Rc::new(RefCell::new(LayoutNodeInner::new(
                Some(label.to_string()),
                0,
            ))),
        }
    }

    /// Creates a new layout node with the given traversal index.
    pub fn new_with_traversal_index(index: u32) -> Self {
        LayoutNode {
            inner: Rc::new(RefCell::new(LayoutNodeInner::new(None, index))),
        }
    }

    /// Creates a new layout node with the given label and traversal index.
    pub fn new_with_label_and_index(label: impl ToString, index: u32) -> Self {
        LayoutNode {
            inner: Rc::new(RefCell::new(LayoutNodeInner::new(
                Some(label.to_string()),
                index,
            ))),
        }
    }

    /// Sets this node's traversal overrides, allowing it to change how windows are assigned
    /// geometries.
    pub fn set_traversal_overrides(&self, overrides: impl IntoIterator<Item = (u32, Vec<u32>)>) {
        self.inner.borrow_mut().traversal_overrides = overrides.into_iter().collect();
    }

    /// Adds a child layout node to this node.
    pub fn add_child(&self, child: Self) {
        self.inner.borrow_mut().children.push(child);
    }

    /// Sets this node's label.
    pub fn set_label(&self, label: Option<impl ToString>) {
        self.inner.borrow_mut().label = label.map(|label| label.to_string());
    }

    /// Sets this node's traversal index, changing how the compositor traverses the tree when
    /// assigning geometries to windows.
    pub fn set_traversal_index(&self, index: u32) {
        self.inner.borrow_mut().traversal_index = index;
    }

    /// Sets this node's children.
    pub fn set_children(&self, children: impl IntoIterator<Item = Self>) {
        self.inner.borrow_mut().children = children.into_iter().collect();
    }

    /// Sets this node's [`LayoutDir`].
    pub fn set_dir(&self, dir: LayoutDir) {
        self.inner.borrow_mut().style.layout_dir = dir;
    }

    /// Sets this node's size proportion in relation to its siblings.
    pub fn set_size_proportion(&self, proportion: f32) {
        self.inner.borrow_mut().style.size_proportion = proportion;
    }

    /// Sets the gaps this node places around its children.
    pub fn set_gaps(&self, gaps: impl Into<Gaps>) {
        self.inner.borrow_mut().style.gaps = gaps.into();
    }
}

/// A layout direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayoutDir {
    /// Lays out nodes in a row.
    Row,
    /// Lays out nodes in a column.
    Column,
}

/// Gaps around a layout node.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Gaps {
    /// How many pixels should be inset from the left.
    pub left: f32,
    /// How many pixels should be inset from the right.
    pub right: f32,
    /// How many pixels should be inset from the top.
    pub top: f32,
    /// How many pixels should be inset from the bottom.
    pub bottom: f32,
}

impl Gaps {
    /// Creates a gap of 0 pixels on all sides.
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a gap with a uniform number of pixels on all sides.
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

impl From<u32> for Gaps {
    fn from(value: u32) -> Self {
        let value = value as f32;
        Self {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }
}

impl From<u16> for Gaps {
    fn from(value: u16) -> Self {
        let value = value as f32;
        Self {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }
}

impl From<u8> for Gaps {
    fn from(value: u8) -> Self {
        let value = value as f32;
        Self {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }
}

#[derive(Debug, Clone)]
struct Style {
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

/// Arguments from an incoming layout request.
#[derive(Clone, Debug)]
pub struct LayoutArgs {
    /// The output that is being laid out.
    pub output: OutputHandle,
    /// The number of windows being laid out.
    pub window_count: u32,
    /// The *focused* tags on the output.
    pub tags: Vec<TagHandle>,
}

/// Types that can generate layouts by computing a tree of [`LayoutNode`]s.
pub trait LayoutGenerator {
    /// Generates a tree of [`LayoutNode`]s.
    fn layout(&self, window_count: u32) -> LayoutNode;
}

/// A struct that can request layouts.
#[derive(Debug, Clone)]
pub struct LayoutRequester {
    sender: UnboundedSender<LayoutRequest>,
}

impl LayoutRequester {
    /// Requests a layout from the compositor.
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

    /// Requests a layout from the compositor for the given output.
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
