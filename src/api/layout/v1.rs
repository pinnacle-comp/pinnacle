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
                        let root_node = tree_response.root_node.unwrap(); // TODO: unwrap
                        let tree_id = tree_response.tree_id;

                        let root_node = match crate::layout::tree::LayoutNode::try_from(root_node) {
                            Ok(root_node) => root_node,
                            Err(()) => {
                                tracing::debug!("failed to create layout tree");
                                return;
                            }
                        };

                        if let Err(err) = state.apply_layout_tree(
                            tree_id,
                            root_node,
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
            flex_basis: taffy::Dimension::percent(style.size_proportion),
            margin: style
                .gaps
                .map(|gaps| taffy::Rect {
                    left: taffy::LengthPercentageAuto::length(gaps.left),
                    right: taffy::LengthPercentageAuto::length(gaps.right),
                    top: taffy::LengthPercentageAuto::length(gaps.top),
                    bottom: taffy::LengthPercentageAuto::length(gaps.bottom),
                })
                .unwrap_or(taffy::Rect::length(0.0)),
            ..Default::default()
        };

        Ok(Self {
            label: node.label,
            traversal_index: node.traversal_index,
            traversal_overrides: node
                .traversal_overrides
                .into_iter()
                .map(|(idx, overrides)| (idx, overrides.overrides))
                .collect(),
            style: taffy_style,
            children: node
                .children
                .into_iter()
                .map(Self::try_from)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}
