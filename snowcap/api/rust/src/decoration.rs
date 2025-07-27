//! Decorations. TODO:

use std::collections::HashMap;

use snowcap_api_defs::snowcap::{
    decoration::{
        self,
        v1::{CloseRequest, NewDecorationRequest, UpdateDecorationRequest},
    },
    widget::v1::{GetWidgetEventsRequest, get_widget_events_request, get_widget_events_response},
};
use tokio_stream::StreamExt;
use tracing::error;

use crate::{
    BlockOnTokio,
    client::Client,
    widget::{Program, Widget, WidgetId},
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

/// The error type for [`Layer::new_widget`].
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
) -> Result<DecorationHandle, NewDecorationError>
where
    Msg: Clone + Send + 'static,
    P: Program<Message = Msg> + Send + 'static,
{
    let mut callbacks = HashMap::<WidgetId, Msg>::new();

    let widget_def = program.view();

    widget_def.collect_messages(&mut callbacks, |def, cbs| {
        if let Widget::Button(button) = &def.widget {
            cbs.extend(button.on_press.clone());
        }
    });

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

    tokio::spawn(async move {
        while let Some(Ok(event)) = event_stream.next().await {
            let id = WidgetId(event.widget_id);
            let Some(event) = event.event else {
                continue;
            };
            match event {
                get_widget_events_response::Event::Button(_event) => {
                    if let Some(msg) = callbacks.get(&id) {
                        program.update(msg.clone());
                        let widget_def = program.view();

                        callbacks.clear();

                        widget_def.collect_messages(&mut callbacks, |def, cbs| {
                            if let Widget::Button(button) = &def.widget {
                                cbs.extend(button.on_press.clone());
                            }
                        });

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
                }
            }
        }
    });

    Ok(DecorationHandle {
        id: decoration_id.into(),
    })
}

/// A handle to a decoration surface.
#[derive(Debug, Clone)]
pub struct DecorationHandle {
    id: WidgetId,
}

impl DecorationHandle {
    /// Close this layer widget.
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
}
