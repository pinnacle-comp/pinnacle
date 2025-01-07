use pinnacle_api_defs::pinnacle::layout::{
    self,
    v1::{LayoutRequest, LayoutResponse},
};
use tokio::sync::mpsc::unbounded_channel;
use tonic::{Request, Streaming};

use crate::{
    api::{run_bidirectional_streaming, ResponseStream, TonicResult},
    layout::LayoutInfo,
    output::OutputName,
};

#[tonic::async_trait]
impl layout::v1::layout_service_server::LayoutService for super::LayoutService {
    type LayoutStream = ResponseStream<LayoutResponse>;

    async fn layout(
        &self,
        request: Request<Streaming<LayoutRequest>>,
    ) -> TonicResult<Self::LayoutStream> {
        let in_stream = request.into_inner();

        run_bidirectional_streaming(
            self.sender.clone(),
            in_stream,
            |state, request| {
                let Some(request) = request.request else {
                    return;
                };

                match request {
                    layout::v1::layout_request::Request::TreeResponse(tree_response) => {
                        let tree = tree_response.tree.unwrap();
                        let tree_id = tree.tree_id;

                        let tree = match crate::layout::tree::LayoutTree::try_from(tree) {
                            Ok(tree) => tree,
                            Err(()) => {
                                tracing::debug!("failed to create layout tree");
                                return;
                            }
                        };

                        if let Err(err) = state.apply_layout_tree(
                            tree_id,
                            tree,
                            tree_response.request_id,
                            tree_response.output_name,
                        ) {
                            tracing::debug!("{err}")
                        }
                    }
                    layout::v1::layout_request::Request::ForceLayout(force_layout) => {
                        let output_name = force_layout.output_name;
                        if let Some(output) = OutputName(output_name)
                            .output(&state.pinnacle)
                            .or_else(|| state.pinnacle.focused_output().cloned())
                        {
                            state.pinnacle.request_layout(&output);
                        }
                    }
                }
            },
            |state, sender, _join_handle| {
                let (send, mut recv) = unbounded_channel::<LayoutInfo>();
                tokio::spawn(async move {
                    while let Some(info) = recv.recv().await {
                        if sender
                            .send(Ok(LayoutResponse {
                                request_id: info.request_id.to_inner(),
                                output_name: info.output_name.0,
                                window_count: info.window_count,
                                tag_ids: info.tag_ids.into_iter().map(|id| id.to_inner()).collect(),
                            }))
                            .is_err()
                        {
                            break;
                        }
                    }
                });
                state
                    .pinnacle
                    .layout_state
                    .layout_request_sender
                    .replace(send);
            },
        )
    }
}

impl TryFrom<layout::v1::LayoutNode> for crate::layout::tree::LayoutNode {
    type Error = ();

    fn try_from(node: layout::v1::LayoutNode) -> Result<Self, Self::Error> {
        let style = node.style.ok_or(())?;

        let taffy_style = taffy::Style {
            flex_direction: match style.flex_dir() {
                layout::v1::FlexDir::Unspecified | layout::v1::FlexDir::Row => {
                    taffy::FlexDirection::Row
                }
                layout::v1::FlexDir::Column => taffy::FlexDirection::Column,
            },
            flex_basis: taffy::Dimension::Percent(style.size_proportion),
            ..Default::default()
        };

        Ok(Self {
            style: taffy_style,
            children: node
                .children
                .into_iter()
                .map(|child| {
                    let node_id = child.node_id;
                    Self::try_from(child).map(|child| (node_id, child))
                })
                .collect::<Result<indexmap::IndexMap<_, _>, _>>()?,
        })
    }
}

impl TryFrom<layout::v1::LayoutTree> for crate::layout::tree::LayoutTree {
    type Error = ();

    fn try_from(tree: layout::v1::LayoutTree) -> Result<Self, Self::Error> {
        let root = tree.root.ok_or(())?;
        let root_id = root.node_id;
        Ok(Self::new(
            root.try_into()?,
            root_id,
            tree.inner_gaps,
            tree.outer_gaps,
        ))
    }
}
