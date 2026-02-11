//! Support for layer surface widgets using `wlr-layer-shell`.

use std::{collections::HashMap, num::NonZeroU32};

use snowcap_api_defs::snowcap::{
    input::v1::{KeyboardKeyRequest, keyboard_key_request::Target},
    layer::{
        self,
        v1::{CloseRequest, NewLayerRequest, OperateLayerRequest, UpdateLayerRequest, ViewRequest},
    },
    widget::v1::{GetWidgetEventsRequest, get_widget_events_request},
};
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::StreamExt;
use tracing::error;
use xkbcommon::xkb::Keysym;

use crate::{
    BlockOnTokio,
    client::Client,
    input::{KeyEvent, Modifiers},
    popup::{self, AsParent},
    widget::{self, Program, WidgetDef, WidgetId, WidgetMessage, operation, signal},
};

// TODO: change to bitflag
/// An anchor for a layer surface.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Anchor {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl From<Anchor> for layer::v1::Anchor {
    fn from(value: Anchor) -> Self {
        match value {
            Anchor::Top => layer::v1::Anchor::Top,
            Anchor::Bottom => layer::v1::Anchor::Bottom,
            Anchor::Left => layer::v1::Anchor::Left,
            Anchor::Right => layer::v1::Anchor::Right,
            Anchor::TopLeft => layer::v1::Anchor::TopLeft,
            Anchor::TopRight => layer::v1::Anchor::TopRight,
            Anchor::BottomLeft => layer::v1::Anchor::BottomLeft,
            Anchor::BottomRight => layer::v1::Anchor::BottomRight,
        }
    }
}

/// Layer surface keyboard interactivity.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum KeyboardInteractivity {
    /// This layer surface cannot get keyboard focus.
    None,
    /// This layer surface can get keyboard focus through the compositor's implementation.
    OnDemand,
    /// This layer surface will take exclusive keyboard focus.
    Exclusive,
}

impl From<KeyboardInteractivity> for layer::v1::KeyboardInteractivity {
    fn from(value: KeyboardInteractivity) -> Self {
        match value {
            KeyboardInteractivity::None => layer::v1::KeyboardInteractivity::None,
            KeyboardInteractivity::OnDemand => layer::v1::KeyboardInteractivity::OnDemand,
            KeyboardInteractivity::Exclusive => layer::v1::KeyboardInteractivity::Exclusive,
        }
    }
}

/// Layer surface behavior for exclusive zones.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ExclusiveZone {
    /// This layer surface requests an exclusive zone of the given size.
    Exclusive(NonZeroU32),
    /// The layer surface does not request an exclusive zone but wants to be
    /// positioned respecting any active exclusive zones.
    Respect,
    /// The layer surface does not request an exclusive zone and wants to be
    /// positioned ignoring any active exclusive zones.
    Ignore,
}

impl From<ExclusiveZone> for i32 {
    fn from(value: ExclusiveZone) -> Self {
        match value {
            ExclusiveZone::Exclusive(size) => size.get() as i32,
            ExclusiveZone::Respect => 0,
            ExclusiveZone::Ignore => -1,
        }
    }
}

/// The layer on which a layer surface will be drawn.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ZLayer {
    Background,
    Bottom,
    Top,
    Overlay,
}

impl From<ZLayer> for layer::v1::Layer {
    fn from(value: ZLayer) -> Self {
        match value {
            ZLayer::Background => Self::Background,
            ZLayer::Bottom => Self::Bottom,
            ZLayer::Top => Self::Top,
            ZLayer::Overlay => Self::Overlay,
        }
    }
}

/// The error type for [`new_widget`].
#[derive(thiserror::Error, Debug)]
pub enum NewLayerError {
    /// Snowcap returned a gRPC error status.
    #[error("gRPC error: `{0}`")]
    GrpcStatus(#[from] tonic::Status),
}

/// The error type for [`LayerHandle::update`] and set_* functions.
#[derive(thiserror::Error, Debug)]
pub enum UpdateLayerError {
    /// Snowcap returned a gRPC error status.
    #[error("gRPC error: `{0}`")]
    GrpcStatus(#[from] tonic::Status),
}

/// Create a new widget.
pub fn new_widget<Msg, P>(
    mut program: P,
    anchor: Option<Anchor>,
    keyboard_interactivity: KeyboardInteractivity,
    exclusive_zone: ExclusiveZone,
    layer: ZLayer,
) -> Result<LayerHandle<Msg>, NewLayerError>
where
    Msg: Clone + Send + 'static,
    P: Program<Message = Msg> + Send + 'static,
{
    let mut callbacks = HashMap::<WidgetId, WidgetMessage<Msg>>::new();

    let widget_def = program.view();

    widget_def.collect_messages(&mut callbacks, WidgetDef::message_collector);

    let response = Client::layer()
        .new_layer(NewLayerRequest {
            widget_def: Some(widget_def.clone().into()),
            anchor: anchor
                .map(From::from)
                .unwrap_or(layer::v1::Anchor::Unspecified) as i32,
            keyboard_interactivity: layer::v1::KeyboardInteractivity::from(keyboard_interactivity)
                as i32,
            exclusive_zone: exclusive_zone.into(),
            layer: layer::v1::Layer::from(layer) as i32,
        })
        .block_on_tokio()?;

    let layer_id = response.into_inner().layer_id;

    let mut event_stream = Client::widget()
        .get_widget_events(GetWidgetEventsRequest {
            id: Some(get_widget_events_request::Id::LayerId(layer_id)),
        })
        .block_on_tokio()?
        .into_inner();

    let (msg_send, mut msg_recv) = tokio::sync::mpsc::unbounded_channel::<Option<Msg>>();

    let handle = LayerHandle {
        id: layer_id.into(),
        msg_sender: msg_send.clone(),
    };

    if let Some(signaler) = program.signaler() {
        signaler.connect({
            let msg_send = msg_send.clone();

            move |msg: signal::Message<Msg>| {
                if let Err(err) = msg_send.send(Some(msg.into_inner())) {
                    error!("Failed to send emitted msg: {err}");
                    crate::signal::HandlerPolicy::Discard
                } else {
                    crate::signal::HandlerPolicy::Keep
                }
            }
        });

        signaler.connect({
            let msg_send = msg_send.clone();

            move |_: signal::RedrawNeeded| {
                if let Err(err) = msg_send.send(None) {
                    error!("Failed to send redraw signal: {err}");
                    crate::signal::HandlerPolicy::Discard
                } else {
                    crate::signal::HandlerPolicy::Keep
                }
            }
        });

        signaler.connect({
            let handle = handle.clone();

            move |operation: operation::Operation| {
                handle.operate(operation);
                crate::signal::HandlerPolicy::Keep
            }
        });
    }

    program.created(handle.clone().into());

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
                    if let Some(msg) = msg {
                        program.update(msg);
                    }

                    if let Err(status) = Client::layer()
                        .request_view(ViewRequest { layer_id })
                        .block_on_tokio()
                    {
                        error!("Failed to request view for {layer_id}: {status}");
                    }

                    continue;
                }
                else => break,
            };

            let widget_def = program.view();

            callbacks.clear();

            widget_def.collect_messages(&mut callbacks, WidgetDef::message_collector);

            Client::layer()
                .update_layer(UpdateLayerRequest {
                    layer_id,
                    widget_def: Some(widget_def.into()),
                    anchor: None,
                    keyboard_interactivity: None,
                    exclusive_zone: None,
                    layer: None,
                })
                .await
                .unwrap();
        }
    });

    Ok(handle)
}

/// A handle to a layer surface.
#[derive(Clone)]
pub struct LayerHandle<Msg> {
    id: WidgetId,
    msg_sender: UnboundedSender<Option<Msg>>,
}

impl<Msg> std::fmt::Debug for LayerHandle<Msg> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayerHandle").field("id", &self.id).finish()
    }
}

impl<Msg> LayerHandle<Msg> {
    /// Update this layer's attributes.
    pub fn update(
        &self,
        anchor: Option<Option<Anchor>>,
        keyboard_interactivity: Option<KeyboardInteractivity>,
        exclusive_zone: Option<ExclusiveZone>,
        layer: Option<ZLayer>,
    ) -> Result<(), UpdateLayerError> {
        let anchor = if let Some(anchor) = anchor {
            anchor
                .map(layer::v1::Anchor::from)
                .or(Some(layer::v1::Anchor::Unspecified))
                .map(i32::from)
        } else {
            None
        };

        let keyboard_interactivity = keyboard_interactivity
            .map(layer::v1::KeyboardInteractivity::from)
            .map(i32::from);

        let exclusive_zone = exclusive_zone.map(i32::from);

        let layer = layer.map(layer::v1::Layer::from).map(i32::from);

        Client::layer()
            .update_layer(UpdateLayerRequest {
                layer_id: self.id.to_inner(),
                widget_def: None,
                anchor,
                keyboard_interactivity,
                exclusive_zone,
                layer,
            })
            .block_on_tokio()?;

        Ok(())
    }

    /// Update this layer's anchor.
    pub fn set_anchor(&self, anchor: Option<Anchor>) -> Result<(), UpdateLayerError> {
        self.update(Some(anchor), None, None, None)
    }

    /// Update this layer's keyboard_interactivity.
    pub fn set_keyboard_interactivity(
        &self,
        keyboard_interactivity: KeyboardInteractivity,
    ) -> Result<(), UpdateLayerError> {
        self.update(None, Some(keyboard_interactivity), None, None)
    }

    /// Update this layer's exclusive_one.
    pub fn set_exclusive_zone(
        &self,
        exclusive_zone: ExclusiveZone,
    ) -> Result<(), UpdateLayerError> {
        self.update(None, None, Some(exclusive_zone), None)
    }

    /// Update this layer's ZLayer.
    pub fn set_layer(&self, layer: ZLayer) -> Result<(), UpdateLayerError> {
        self.update(None, None, None, Some(layer))
    }

    /// Close this layer widget.
    pub fn close(&self) {
        if let Err(status) = Client::layer()
            .close(CloseRequest {
                layer_id: self.id.to_inner(),
            })
            .block_on_tokio()
        {
            error!("Failed to close {self:?}: {status}");
        }
    }

    /// Sends a message to this Layer [`Program`].
    pub fn send_message(&self, message: Msg) {
        let _ = self.msg_sender.send(Some(message));
    }

    /// Forces this layer to redraw.
    pub fn force_redraw(&self) {
        let _ = self.msg_sender.send(None);
    }

    /// Sends an [`Operation`] to this Layer.
    ///
    /// [`Operation`]: widget::operation::Operation
    pub fn operate(&self, operation: widget::operation::Operation) {
        if let Err(status) = Client::layer()
            .operate_layer(OperateLayerRequest {
                layer_id: self.id.to_inner(),
                operation: Some(operation.into()),
            })
            .block_on_tokio()
        {
            error!("Failed to send operation to {self:?}: {status}");
        }
    }
}

impl<Msg> LayerHandle<Msg>
where
    Msg: Clone + Send + 'static,
{
    /// Do something when a key event is received
    pub fn on_key_event(
        &self,
        mut on_event: impl FnMut(LayerHandle<Msg>, KeyEvent) + Send + 'static,
    ) {
        let mut stream = match Client::input()
            .keyboard_key(KeyboardKeyRequest {
                target: Some(Target::LayerId(self.id.to_inner())),
            })
            .block_on_tokio()
        {
            Ok(stream) => stream.into_inner(),
            Err(status) => {
                error!("Failed to set `on_key_event` handler: {status}");
                return;
            }
        };

        let handle = self.clone();

        tokio::spawn(async move {
            while let Some(Ok(response)) = stream.next().await {
                let event = KeyEvent::from(response);

                on_event(handle.clone(), event);
            }
        });
    }

    /// Do something on key press.
    pub fn on_key_press(
        &self,
        mut on_press: impl FnMut(LayerHandle<Msg>, Keysym, Modifiers) + Send + 'static,
    ) {
        self.on_key_event(move |handle, event| {
            if !event.pressed || event.captured {
                return;
            }

            on_press(handle, event.key, event.mods)
        });
    }
}

impl<Msg> AsParent for LayerHandle<Msg> {
    fn as_parent(&self) -> crate::popup::Parent {
        popup::Parent(popup::ParentInner::Layer(self.id))
    }
}
