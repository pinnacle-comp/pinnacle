//! Surfaces that widgets can be created on.

use crate::{
    decoration::DecorationHandle,
    layer::LayerHandle,
    popup::{AsParent, Parent, PopupHandle},
    widget::operation::Operation,
};

pub mod decoration;
pub mod layer;
pub mod popup;

/// Implementation detail for [`SurfaceHandle`]
#[derive(Clone)]
enum Inner<Msg> {
    /// A handle to a layer surface.
    Layer(LayerHandle<Msg>),
    /// A handle to a decoration surface.
    Decoration(DecorationHandle<Msg>),
    /// A handle to a popup surface.
    Popup(PopupHandle<Msg>),
}

/// A handle to a surface.
#[derive(Clone)]
pub struct SurfaceHandle<Msg>(Inner<Msg>);

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
    /// Closes this surface.
    pub fn close(&self) {
        match &self.0 {
            Inner::Layer(layer_handle) => layer_handle.close(),
            Inner::Decoration(decoration_handle) => decoration_handle.close(),
            Inner::Popup(popup_handle) => popup_handle.close(),
        }
    }

    /// Sends an [`Operation`] to this surface.
    pub fn operate(&self, operation: Operation) {
        match &self.0 {
            Inner::Layer(layer_handle) => layer_handle.operate(operation),
            Inner::Decoration(decoration_handle) => decoration_handle.operate(operation),
            Inner::Popup(popup_handle) => popup_handle.operate(operation),
        }
    }

    /// Sends a message to this surface.
    pub fn send_message(&self, message: Msg) {
        match &self.0 {
            Inner::Layer(layer_handle) => layer_handle.send_message(message),
            Inner::Decoration(decoration_handle) => decoration_handle.send_message(message),
            Inner::Popup(popup_handle) => popup_handle.send_message(message),
        }
    }

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
