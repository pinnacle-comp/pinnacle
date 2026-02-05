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

/// A handle to a surface.
#[derive(Clone)]
pub enum SurfaceHandle<Msg> {
    /// A handle to a layer surface.
    Layer(LayerHandle<Msg>),
    /// A handle to a decoration surface.
    Decoration(DecorationHandle<Msg>),
    /// A handle to a popup surface.
    Popup(PopupHandle<Msg>),
}

impl<Msg> SurfaceHandle<Msg> {
    /// Closes this surface.
    pub fn close(&self) {
        match self {
            SurfaceHandle::Layer(layer_handle) => layer_handle.close(),
            SurfaceHandle::Decoration(decoration_handle) => decoration_handle.close(),
            SurfaceHandle::Popup(popup_handle) => popup_handle.close(),
        }
    }

    /// Sends an [`Operation`] to this surface.
    pub fn operate(&self, operation: Operation) {
        match self {
            SurfaceHandle::Layer(layer_handle) => layer_handle.operate(operation),
            SurfaceHandle::Decoration(decoration_handle) => decoration_handle.operate(operation),
            SurfaceHandle::Popup(popup_handle) => popup_handle.operate(operation),
        }
    }

    /// Sends a message to this surface.
    pub fn send_message(&self, message: Msg) {
        match self {
            SurfaceHandle::Layer(layer_handle) => layer_handle.send_message(message),
            SurfaceHandle::Decoration(decoration_handle) => decoration_handle.send_message(message),
            SurfaceHandle::Popup(popup_handle) => popup_handle.send_message(message),
        }
    }

    /// Forces this surface to redraw.
    pub fn force_redraw(&self) {
        match self {
            SurfaceHandle::Layer(layer_handle) => layer_handle.force_redraw(),
            SurfaceHandle::Decoration(decoration_handle) => decoration_handle.force_redraw(),
            SurfaceHandle::Popup(popup_handle) => popup_handle.force_redraw(),
        }
    }
}

impl<Msg> AsParent for SurfaceHandle<Msg> {
    fn as_parent(&self) -> Parent {
        match self {
            SurfaceHandle::Layer(layer_handle) => layer_handle.as_parent(),
            SurfaceHandle::Decoration(decoration_handle) => decoration_handle.as_parent(),
            SurfaceHandle::Popup(popup_handle) => popup_handle.as_parent(),
        }
    }
}
