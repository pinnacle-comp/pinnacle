//! Support for layer surface widgets using `wlr-layer-shell`.

use std::num::NonZeroU32;

use snowcap_api_defs::snowcap::{
    input::v0alpha1::KeyboardKeyRequest,
    layer::{
        self,
        v0alpha1::{CloseRequest, NewLayerRequest},
    },
};
use tokio_stream::StreamExt;
use tracing::error;
use xkbcommon::xkb::Keysym;

use crate::{
    block_on_tokio,
    input::Modifiers,
    widget::{WidgetDef, WidgetId},
};

/// The Layer API.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Layer;

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

impl From<Anchor> for layer::v0alpha1::Anchor {
    fn from(value: Anchor) -> Self {
        match value {
            Anchor::Top => layer::v0alpha1::Anchor::Top,
            Anchor::Bottom => layer::v0alpha1::Anchor::Bottom,
            Anchor::Left => layer::v0alpha1::Anchor::Left,
            Anchor::Right => layer::v0alpha1::Anchor::Right,
            Anchor::TopLeft => layer::v0alpha1::Anchor::TopLeft,
            Anchor::TopRight => layer::v0alpha1::Anchor::TopRight,
            Anchor::BottomLeft => layer::v0alpha1::Anchor::BottomLeft,
            Anchor::BottomRight => layer::v0alpha1::Anchor::BottomRight,
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

impl From<KeyboardInteractivity> for layer::v0alpha1::KeyboardInteractivity {
    fn from(value: KeyboardInteractivity) -> Self {
        match value {
            KeyboardInteractivity::None => layer::v0alpha1::KeyboardInteractivity::None,
            KeyboardInteractivity::OnDemand => layer::v0alpha1::KeyboardInteractivity::OnDemand,
            KeyboardInteractivity::Exclusive => layer::v0alpha1::KeyboardInteractivity::Exclusive,
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

impl From<ZLayer> for layer::v0alpha1::Layer {
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
    /// Snowcap did not return a layer id as expected.
    #[error("snowcap did not return a layer id")]
    NoLayerId,
}

impl Layer {
    /// Create a new widget.
    pub fn new_widget(
        &self,
        widget: impl Into<WidgetDef>,
        width: u32,
        height: u32,
        anchor: Option<Anchor>,
        keyboard_interactivity: KeyboardInteractivity,
        exclusive_zone: ExclusiveZone,
        layer: ZLayer,
    ) -> Result<LayerHandle, NewLayerError> {
        let response = block_on_tokio(crate::layer().new_layer(NewLayerRequest {
            widget_def: Some(widget.into().into()),
            width: Some(width),
            height: Some(height),
            anchor: anchor.map(|anchor| layer::v0alpha1::Anchor::from(anchor) as i32),
            keyboard_interactivity: Some(layer::v0alpha1::KeyboardInteractivity::from(
                keyboard_interactivity,
            ) as i32),
            exclusive_zone: Some(exclusive_zone.into()),
            layer: Some(layer::v0alpha1::Layer::from(layer) as i32),
        }))?;

        let id = response
            .into_inner()
            .layer_id
            .ok_or(NewLayerError::NoLayerId)?;

        Ok(LayerHandle { id: id.into() })
    }
}

/// A handle to a layer surface widget.
#[derive(Debug, Clone, Copy)]
pub struct LayerHandle {
    id: WidgetId,
}

impl LayerHandle {
    /// Close this layer widget.
    pub fn close(&self) {
        if let Err(status) = block_on_tokio(crate::layer().close(CloseRequest {
            layer_id: Some(self.id.into_inner()),
        })) {
            error!("Failed to close {self:?}: {status}");
        }
    }

    /// Do something on key press.
    pub fn on_key_press(
        &self,
        mut on_press: impl FnMut(LayerHandle, Keysym, Modifiers) + Send + 'static,
    ) {
        let mut stream = match block_on_tokio(crate::input().keyboard_key(KeyboardKeyRequest {
            id: Some(self.id.into_inner()),
        })) {
            Ok(stream) => stream.into_inner(),
            Err(status) => {
                error!("Failed to set `on_key_press` handler: {status}");
                return;
            }
        };

        let handle = *self;

        tokio::spawn(async move {
            while let Some(Ok(response)) = stream.next().await {
                if !response.pressed() {
                    continue;
                }

                let key = Keysym::new(response.key());
                let mods = Modifiers::from(response.modifiers.unwrap_or_default());

                on_press(handle, key, mods);
            }
        });
    }
}
