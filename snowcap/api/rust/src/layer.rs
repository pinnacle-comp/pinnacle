//! Support for layer surface widgets using `wlr-layer-shell`.

use std::{collections::HashMap, num::NonZeroU32};

use snowcap_api_defs::snowcap::{
    input::v1::KeyboardKeyRequest,
    layer::{
        self,
        v1::{CloseRequest, NewLayerRequest, UpdateLayerRequest},
    },
    widget::v1::{GetWidgetEventsRequest, get_widget_events_response},
};
use tokio_stream::StreamExt;
use tracing::error;
use xkbcommon::xkb::Keysym;

use crate::{
    BlockOnTokio,
    client::Client,
    input::Modifiers,
    widget::{Program, Widget, WidgetId},
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

/// The error type for [`Layer::new_widget`].
#[derive(thiserror::Error, Debug)]
pub enum NewLayerError {
    /// Snowcap returned a gRPC error status.
    #[error("gRPC error: `{0}`")]
    GrpcStatus(#[from] tonic::Status),
}

/// Create a new widget.
pub fn new_widget<Msg, P>(
    mut program: P,
    width: u32,
    height: u32,
    anchor: Option<Anchor>,
    keyboard_interactivity: KeyboardInteractivity,
    exclusive_zone: ExclusiveZone,
    layer: ZLayer,
) -> Result<LayerHandle, NewLayerError>
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

    let response = Client::layer()
        .new_layer(NewLayerRequest {
            widget_def: Some(widget_def.clone().into()),
            width,
            height,
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
        .get_widget_events(GetWidgetEventsRequest { layer_id })
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

                        Client::layer()
                            .update_layer(UpdateLayerRequest {
                                layer_id,
                                widget_def: Some(widget_def.into()),
                                width: None,
                                height: None,
                                anchor: None,
                                keyboard_interactivity: None,
                                exclusive_zone: None,
                                layer: None,
                            })
                            .await
                            .unwrap();
                    }
                }
            }
        }
    });

    Ok(LayerHandle {
        id: layer_id.into(),
    })
}

/// A handle to a layer surface widget.
#[derive(Debug, Clone, Copy)]
pub struct LayerHandle {
    id: WidgetId,
}

impl LayerHandle {
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

    /// Do something on key press.
    pub fn on_key_press(
        &self,
        mut on_press: impl FnMut(LayerHandle, Keysym, Modifiers) + Send + 'static,
    ) {
        let mut stream = match Client::input()
            .keyboard_key(KeyboardKeyRequest {
                id: self.id.to_inner(),
            })
            .block_on_tokio()
        {
            Ok(stream) => stream.into_inner(),
            Err(status) => {
                error!("Failed to set `on_key_press` handler: {status}");
                return;
            }
        };

        let handle = *self;

        tokio::spawn(async move {
            while let Some(Ok(response)) = stream.next().await {
                if !response.pressed {
                    continue;
                }

                let key = Keysym::new(response.key);
                let mods = Modifiers::from(response.modifiers.unwrap_or_default());

                on_press(handle, key, mods);
            }
        });
    }
}
