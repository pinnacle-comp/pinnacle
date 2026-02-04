//! Decorations. TODO:

use std::collections::HashMap;

use snowcap_api_defs::snowcap::{
    decoration::{
        self,
        v1::{
            CloseRequest, NewDecorationRequest, OperateDecorationRequest, UpdateDecorationRequest,
            ViewRequest,
        },
    },
    widget::v1::{GetWidgetEventsRequest, get_widget_events_request},
};
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::StreamExt;
use tracing::error;

use crate::{
    BlockOnTokio,
    client::Client,
    popup::{self, AsParent},
    widget::{self, Program, WidgetDef, WidgetId, WidgetMessage},
};

/// The bounds of a window or decoration.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct Bounds {
    /// The bounds that extend the left edge.
    pub left: u32,
    /// The bounds that extend the right edge.
    pub right: u32,
    /// The bounds that extend the top edge.
    pub top: u32,
    /// The bounds that extend the bottom edge.
    pub bottom: u32,
}

impl Bounds {
    /// Creates bounds where all edges are the given amount.
    pub fn all(bounds: u32) -> Self {
        Self {
            left: bounds,
            right: bounds,
            top: bounds,
            bottom: bounds,
        }
    }
}

impl From<Bounds> for decoration::v1::Bounds {
    fn from(value: Bounds) -> Self {
        Self {
            left: value.left,
            right: value.right,
            top: value.top,
            bottom: value.bottom,
        }
    }
}

/// The error type for [`new_widget`].
#[derive(thiserror::Error, Debug)]
pub enum NewDecorationError {
    /// Snowcap returned a gRPC error status.
    #[error("gRPC error: `{0}`")]
    GrpcStatus(#[from] tonic::Status),
}

/// Create a new widget.
pub fn new_widget<Msg, P>(
    mut program: P,
    toplevel_identifier: String,
    bounds: Bounds,
    extents: Bounds,
    z_index: i32,
) -> Result<DecorationHandle<Msg>, NewDecorationError>
where
    Msg: Clone + Send + 'static,
    P: Program<Message = Msg> + Send + 'static,
{
    let mut callbacks = HashMap::<WidgetId, WidgetMessage<Msg>>::new();

    let widget_def = program.view();

    widget_def.collect_messages(&mut callbacks, WidgetDef::message_collector);

    let response = Client::decoration()
        .new_decoration(NewDecorationRequest {
            widget_def: Some(widget_def.clone().into()),
            foreign_toplevel_handle_identifier: toplevel_identifier,
            bounds: Some(bounds.into()),
            extents: Some(extents.into()),
            z_index,
        })
        .block_on_tokio()?;

    let decoration_id = response.into_inner().decoration_id;

    let mut event_stream = Client::widget()
        .get_widget_events(GetWidgetEventsRequest {
            id: Some(get_widget_events_request::Id::DecorationId(decoration_id)),
        })
        .block_on_tokio()?
        .into_inner();

    let (msg_send, mut msg_recv) = tokio::sync::mpsc::unbounded_channel::<Msg>();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(Ok(response)) = event_stream.next() => {
                    for widget_event in response.widget_events {
                        let Some(msg) = widget::message_from_event(&callbacks, widget_event) else {
                            continue;
                        };

                        program.update(msg);
                    }
                }
                Some(msg) = msg_recv.recv() => {
                    program.update(msg);

                    if let Err(status) = Client::decoration()
                        .request_view(ViewRequest { decoration_id })
                        .block_on_tokio()
                    {
                        error!("Failed to request view for {decoration_id}: {status}")
                    }

                    continue;
                }
                else => break,
            };

            let widget_def = program.view();

            callbacks.clear();

            widget_def.collect_messages(&mut callbacks, WidgetDef::message_collector);

            Client::decoration()
                .update_decoration(UpdateDecorationRequest {
                    decoration_id,
                    widget_def: Some(widget_def.into()),
                    bounds: None,
                    extents: None,
                    z_index: None,
                })
                .await
                .unwrap();
        }
    });

    Ok(DecorationHandle {
        id: decoration_id.into(),
        msg_sender: msg_send,
    })
}

/// A handle to a decoration surface.
#[derive(Clone)]
pub struct DecorationHandle<Msg> {
    id: WidgetId,
    msg_sender: UnboundedSender<Msg>,
}

impl<Msg> std::fmt::Debug for DecorationHandle<Msg> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecorationHandle")
            .field("id", &self.id)
            .finish()
    }
}

impl<Msg> DecorationHandle<Msg> {
    /// Closes this decoration.
    pub fn close(&self) {
        if let Err(status) = Client::decoration()
            .close(CloseRequest {
                decoration_id: self.id.to_inner(),
            })
            .block_on_tokio()
        {
            error!("Failed to close {self:?}: {status}");
        }
    }

    /// Sends a message to this decoration's [`Program`].
    pub fn send_message(&self, message: Msg) {
        let _ = self.msg_sender.send(message);
    }

    /// Sends an [`Operation`] to this Decoration.
    ///
    /// [`Operation`]: widget::operation::Operation
    pub fn operate(&self, operation: widget::operation::Operation) {
        if let Err(status) = Client::decoration()
            .operate_decoration(OperateDecorationRequest {
                decoration_id: self.id.to_inner(),
                operation: Some(operation.into()),
            })
            .block_on_tokio()
        {
            error!("Failed to send operation to {self:?}: {status}");
        }
    }

    /// Sets the z-index that this decoration will render at.
    pub fn set_z_index(&self, z_index: i32) {
        Client::decoration()
            .update_decoration(UpdateDecorationRequest {
                decoration_id: self.id.0,
                widget_def: None,
                bounds: None,
                extents: None,
                z_index: Some(z_index),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Sets the extents of this decoration.
    ///
    /// The extents extend the drawable area of the decorated
    /// toplevel by the specified amounts in each direction.
    pub fn set_extents(&self, extents: Bounds) {
        Client::decoration()
            .update_decoration(UpdateDecorationRequest {
                decoration_id: self.id.0,
                widget_def: None,
                bounds: None,
                extents: Some(extents.into()),
                z_index: None,
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Sets the bounds of this decoration.
    ///
    /// The bounds extend the geometry of the decorated
    /// toplevel by the specified amounts in each direction,
    /// causing parts or all of the decoration to be included.
    pub fn set_bounds(&self, bounds: Bounds) {
        Client::decoration()
            .update_decoration(UpdateDecorationRequest {
                decoration_id: self.id.0,
                widget_def: None,
                bounds: Some(bounds.into()),
                extents: None,
                z_index: None,
            })
            .block_on_tokio()
            .unwrap();
    }
}

impl<Msg> AsParent for DecorationHandle<Msg> {
    fn as_parent(&self) -> crate::popup::Parent {
        popup::Parent(popup::ParentInner::Decoration(self.id))
    }
}
