//! Surfaces that widgets can be created on.

use crate::{
    decoration::DecorationHandle,
    layer::LayerHandle,
    popup::{AsParent, Parent, PopupHandle},
};

pub mod decoration;
pub mod layer;
pub mod popup;

/// Events emitted by the surface to notify [`Program`] of state changes.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum SurfaceEvent<Msg> {
    /// Emitted when the surface is created.
    ///
    /// A [`SurfaceHandle`] is provided to allow the program to manipulate the
    /// surface. The handle will remains valid for the lifetime of the program
    /// and may be cloned for later use.
    Created {
        /// The surface's handle.
        surface: SurfaceHandle<Msg>,
    },
    /// Emitted when the surface is being closed.
    ///
    /// This event is emitted during the surface termination. If the program
    /// stored the handle passed via [`Self::Created`], this handle should be
    /// considered stale.
    Closing,

    /// Emitted when the surface gains focus.
    FocusGained,
    /// Emitted when the surface loses focus.
    FocusLost,
}

/// Implementation detail for [`SurfaceHandle`]
enum Inner<Msg> {
    /// A handle to a layer surface.
    Layer(LayerHandle<Msg>),
    /// A handle to a decoration surface.
    Decoration(DecorationHandle<Msg>),
    /// A handle to a popup surface.
    Popup(PopupHandle<Msg>),
}

impl<Msg> Clone for Inner<Msg> {
    fn clone(&self) -> Self {
        match &self {
            Self::Layer(handle) => Self::Layer(handle.clone()),
            Self::Decoration(handle) => Self::Decoration(handle.clone()),
            Self::Popup(handle) => Self::Popup(handle.clone()),
        }
    }
}

/// A handle to a surface.
pub struct SurfaceHandle<Msg>(Inner<Msg>);

impl<Msg> Clone for SurfaceHandle<Msg> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<Msg> std::fmt::Debug for Inner<Msg> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Layer(handle) => handle.fmt(f),
            Self::Decoration(handle) => handle.fmt(f),
            Self::Popup(handle) => handle.fmt(f),
        }
    }
}

impl<Msg> std::fmt::Debug for SurfaceHandle<Msg> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SurfaceHandle").field(&self.0).finish()
    }
}

impl<Msg> From<LayerHandle<Msg>> for SurfaceHandle<Msg> {
    fn from(value: LayerHandle<Msg>) -> Self {
        Self(Inner::Layer(value))
    }
}

impl<Msg> From<DecorationHandle<Msg>> for SurfaceHandle<Msg> {
    fn from(value: DecorationHandle<Msg>) -> Self {
        Self(Inner::Decoration(value))
    }
}

impl<Msg> From<PopupHandle<Msg>> for SurfaceHandle<Msg> {
    fn from(value: PopupHandle<Msg>) -> Self {
        Self(Inner::Popup(value))
    }
}

impl<Msg> SurfaceHandle<Msg> {
    /// Forces this surface to redraw.
    pub fn force_redraw(&self) {
        match &self.0 {
            Inner::Layer(layer_handle) => layer_handle.force_redraw(),
            Inner::Decoration(decoration_handle) => decoration_handle.force_redraw(),
            Inner::Popup(popup_handle) => popup_handle.force_redraw(),
        }
    }
}

impl<Msg> AsParent for SurfaceHandle<Msg> {
    fn as_parent(&self) -> Parent {
        match &self.0 {
            Inner::Layer(layer_handle) => layer_handle.as_parent(),
            Inner::Decoration(decoration_handle) => decoration_handle.as_parent(),
            Inner::Popup(popup_handle) => popup_handle.as_parent(),
        }
    }
}
