//! Support for popup surface widgets using `xdg-shell::xdg_popup`

use std::collections::HashMap;

use bitflags::bitflags;
use snowcap_api_defs::snowcap::{
    input::v1::{KeyboardKeyRequest, keyboard_key_request::Target},
    popup::{
        self,
        v1::{CloseRequest, NewPopupRequest, OperatePopupRequest, UpdatePopupRequest, ViewRequest},
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
    widget::{self, Program, WidgetDef, WidgetId, WidgetMessage, signal},
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ParentInner {
    Layer(WidgetId),
    Decoration(WidgetId),
    Popup(WidgetId),
}

/// Popup Parent surface.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Parent(pub(crate) ParentInner);

/// Position the Popup will be placed at.
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
pub enum Position {
    /// Position the anchor point at the cursor.
    AtCursor,
    /// Position the anchor at an arbitrary point.
    Point { x: f32, y: f32 },
    /// Position the anchor on a Rectangle boundaries.
    Rectangle {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    /// Position the anchor on a Widget boundaries.
    AtWidget(String),
}

impl Position {
    /// Create a new Position to place a Popup at the cursor location.
    pub fn at_cursor() -> Self {
        Position::AtCursor
    }

    /// Create a new Position to place a Popup at an arbitrary point.
    pub fn point(x: f32, y: f32) -> Self {
        Position::Point { x, y }
    }

    /// Create a new Position to place a Popup on an arbitrary rectangle.
    pub fn rectangle(x: f32, y: f32, width: f32, height: f32) -> Self {
        Position::Rectangle {
            x,
            y,
            width,
            height,
        }
    }

    /// Create a new Position to place a Popup relative to a Widget.
    pub fn at_widget(id: impl Into<String>) -> Self {
        Position::AtWidget(id.into())
    }
}

/// Position of the anchor point on the anchor rectangle.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Anchor {
    None,
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Direction of the gravity of the Popup.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Gravity {
    None,
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Popup position offset
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Offset {
    pub x: f32,
    pub y: f32,
}

bitflags! {
    /// Define ways the compositor can adjust the popup if its position would make it partially
    /// constrained.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ConstraintsAdjust: u32 {
        /// Don't move the child surface when constrained.
        const None = 0;
        /// Move along the x axis until unconstrained.
        const SlideX = 1;
        /// Move along the y axis until unconstrained.
        const SlideY = 2;
        /// Invert the anchor and gravity on the x axis.
        const FlipX = 4;
        /// Invert the anchor and gravity on the y axis.
        const FlipY = 8;
        /// Horizontally resize the surface.
        const ResizeX = 16;
        /// Vertically resize the surface.
        const ResizeY = 32;
    }
}

/// The error type for [`Popup::new_widget`].
///
/// [`Popup::new_widget`]: self::new_widget
#[derive(thiserror::Error, Debug)]
pub enum NewPopupError {
    /// Snowcap returned a gRPC error status.
    #[error("gRPC error: `{0}`")]
    GrpcStatus(#[from] tonic::Status),
}

/// The error type for [`PopupHandle::update`].
#[derive(thiserror::Error, Debug)]
pub enum UpdatePopupError {
    /// Snowcap returned a gRPC error status.
    #[error("gRPC error: `{0}`")]
    GrpcStatus(#[from] tonic::Status),
}

/// Create a new popup.
pub fn new_widget<Msg, P>(
    mut program: P,
    parent: &impl AsParent,
    position: Position,
    anchor: Option<Anchor>,
    gravity: Option<Gravity>,
    offset: Option<Offset>,
    constraints_adjust: Option<ConstraintsAdjust>,
    no_grab: bool,
    no_replace: bool,
) -> Result<PopupHandle<Msg>, NewPopupError>
where
    Msg: Clone + Send + 'static,
    P: Program<Message = Msg> + Send + 'static,
{
    let mut callbacks = HashMap::<WidgetId, WidgetMessage<Msg>>::new();

    let widget_def = program.view();

    widget_def.collect_messages(&mut callbacks, WidgetDef::message_collector);

    let response = Client::popup()
        .new_popup(NewPopupRequest {
            widget_def: Some(widget_def.clone().into()),
            parent_id: Some(parent.as_parent().into()),
            position: Some(position.into()),
            anchor: anchor
                .map(From::from)
                .unwrap_or(popup::v1::Anchor::Unspecified) as i32,
            gravity: gravity
                .map(From::from)
                .unwrap_or(popup::v1::Gravity::Unspecified) as i32,
            offset: offset.map(From::from),
            constraints_adjust: constraints_adjust.map(From::from),
            no_grab,
            no_replace,
        })
        .block_on_tokio()?;

    let popup_id = response.into_inner().popup_id;

    let mut event_stream = Client::widget()
        .get_widget_events(GetWidgetEventsRequest {
            id: Some(get_widget_events_request::Id::PopupId(popup_id)),
        })
        .block_on_tokio()?
        .into_inner();

    let (msg_send, mut msg_recv) = tokio::sync::mpsc::unbounded_channel::<Option<Msg>>();

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
    }

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

                    if let Err(status) = Client::popup()
                        .request_view(ViewRequest { popup_id })
                        .block_on_tokio()
                    {
                        tracing::error!("Failed to request view for {popup_id}: {status}");
                    }

                    continue;
                }
                else => break,
            };

            let widget_def = program.view();

            callbacks.clear();

            widget_def.collect_messages(&mut callbacks, WidgetDef::message_collector);

            Client::popup()
                .update_popup(UpdatePopupRequest {
                    popup_id,
                    widget_def: Some(widget_def.into()),
                    ..Default::default()
                })
                .await
                .unwrap();
        }
    });

    Ok(PopupHandle {
        id: popup_id.into(),
        msg_sender: msg_send,
    })
}

/// A handle to a popup surface.
#[derive(Clone)]
pub struct PopupHandle<Msg> {
    id: WidgetId,
    msg_sender: UnboundedSender<Option<Msg>>,
}

impl<Msg> PopupHandle<Msg> {
    /// Close this popup widget.
    pub fn close(&self) {
        if let Err(status) = Client::popup()
            .close(CloseRequest {
                popup_id: self.id.to_inner(),
            })
            .block_on_tokio()
        {
            tracing::error!("Failed to close {self:?}: {status}");
        }
    }
}

impl<Msg> PopupHandle<Msg>
where
    Msg: Clone + Send + 'static,
{
    /// Update this popup's attributes.
    pub fn update(
        &self,
        position: Option<Position>,
        anchor: Option<Anchor>,
        gravity: Option<Gravity>,
        offset: Option<Offset>,
        constraints_adjust: Option<ConstraintsAdjust>,
    ) -> Result<(), UpdatePopupError> {
        Client::popup()
            .update_popup(UpdatePopupRequest {
                popup_id: self.id.to_inner(),
                widget_def: None,
                position: position.map(From::from),
                anchor: anchor
                    .map(From::from)
                    .or(Some(popup::v1::Anchor::Unspecified))
                    .map(i32::from),
                gravity: gravity
                    .map(From::from)
                    .or(Some(popup::v1::Gravity::Unspecified))
                    .map(i32::from),
                offset: offset.map(From::from),
                constraints_adjust: constraints_adjust.map(From::from),
            })
            .block_on_tokio()?;

        Ok(())
    }

    /// Update this popup's position.
    pub fn set_position(&self, position: Position) -> Result<(), UpdatePopupError> {
        self.update(Some(position), None, None, None, None)
    }

    /// Update this popup's anchor.
    pub fn set_anchor(&self, anchor: Anchor) -> Result<(), UpdatePopupError> {
        self.update(None, Some(anchor), None, None, None)
    }

    /// Update this popup's gravity.
    pub fn set_gravity(&self, gravity: Gravity) -> Result<(), UpdatePopupError> {
        self.update(None, None, Some(gravity), None, None)
    }

    /// Update this popup's offset.
    pub fn set_offset(&self, offset: Offset) -> Result<(), UpdatePopupError> {
        self.update(None, None, None, Some(offset), None)
    }

    /// Update this popup's contraints adjustment.
    pub fn set_constraints_adjust(
        &self,
        constraints_adjust: ConstraintsAdjust,
    ) -> Result<(), UpdatePopupError> {
        self.update(None, None, None, None, Some(constraints_adjust))
    }

    /// Sends an [`Operation`] to this Popup.
    ///
    /// [`Operation`]: widget::operation::Operation
    pub fn operate(&self, operation: widget::operation::Operation) {
        if let Err(status) = Client::popup()
            .operate_popup(OperatePopupRequest {
                popup_id: self.id.to_inner(),
                operation: Some(operation.into()),
            })
            .block_on_tokio()
        {
            tracing::error!("Failed to send operation to {self:?}: {status}");
        }
    }

    /// Sends a message to this Popup [`Program`].
    pub fn send_message(&self, message: Msg) {
        let _ = self.msg_sender.send(Some(message));
    }

    /// Forces this popup to redraw.
    pub fn force_redraw(&self) {
        let _ = self.msg_sender.send(None);
    }

    /// Do something when a key event is received.
    pub fn on_key_event(
        &self,
        mut on_event: impl FnMut(PopupHandle<Msg>, KeyEvent) + Send + 'static,
    ) {
        let mut stream = match Client::input()
            .keyboard_key(KeyboardKeyRequest {
                target: Some(Target::PopupId(self.id.to_inner())),
            })
            .block_on_tokio()
        {
            Ok(stream) => stream.into_inner(),
            Err(status) => {
                tracing::error!("Failed to set `on_key_event` handler: {status}");
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
        mut on_press: impl FnMut(PopupHandle<Msg>, Keysym, Modifiers) + Send + 'static,
    ) {
        self.on_key_event(move |handle, event| {
            if !event.pressed || event.captured {
                return;
            }

            on_press(handle, event.key, event.mods)
        });
    }
}

impl<Msg> std::fmt::Debug for PopupHandle<Msg> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PopupHandle").field("id", &self.id).finish()
    }
}

impl std::fmt::Debug for Parent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            ParentInner::Layer(id) => f.debug_tuple("Parent::Layer").field(&id).finish(),
            ParentInner::Decoration(id) => f.debug_tuple("Parent::Decoration").field(&id).finish(),
            ParentInner::Popup(id) => f.debug_tuple("Parent::Popup").field(&id).finish(),
        }
    }
}

/// Used to convert a handle to a popup's [`Parent`].
pub trait AsParent {
    /// Convert a reference to a surface handle to a popup [`Parent`].
    fn as_parent(&self) -> Parent;
}

impl<Msg> AsParent for PopupHandle<Msg> {
    fn as_parent(&self) -> Parent {
        Parent(ParentInner::Popup(self.id))
    }
}

impl AsParent for Parent {
    fn as_parent(&self) -> Parent {
        *self
    }
}

impl From<Parent> for popup::v1::new_popup_request::ParentId {
    fn from(value: Parent) -> Self {
        use popup::v1::new_popup_request::ParentId;
        match value.0 {
            ParentInner::Layer(id) => ParentId::LayerId(id.to_inner()),
            ParentInner::Decoration(id) => ParentId::DecoId(id.to_inner()),
            ParentInner::Popup(id) => ParentId::PopupId(id.to_inner()),
        }
    }
}

impl From<Position> for popup::v1::Position {
    fn from(value: Position) -> Self {
        Self {
            strategy: Some(value.into()),
        }
    }
}

impl From<Position> for popup::v1::position::Strategy {
    fn from(value: Position) -> Self {
        use popup::v1::{Rectangle, position::Strategy};

        match value {
            Position::AtCursor => Strategy::AtCursor(()),
            Position::Point { x, y } => Strategy::Absolute(Rectangle {
                x,
                y,
                width: 1.,
                height: 1.,
            }),
            Position::Rectangle {
                x,
                y,
                width,
                height,
            } => Strategy::Absolute(Rectangle {
                x,
                y,
                width,
                height,
            }),
            Position::AtWidget(id) => Strategy::AtWidget(id),
        }
    }
}

impl From<Anchor> for popup::v1::Anchor {
    fn from(value: Anchor) -> Self {
        use popup::v1;

        match value {
            Anchor::None => v1::Anchor::None,
            Anchor::Top => v1::Anchor::Top,
            Anchor::Bottom => v1::Anchor::Bottom,
            Anchor::Left => v1::Anchor::Left,
            Anchor::Right => v1::Anchor::Right,
            Anchor::TopLeft => v1::Anchor::TopLeft,
            Anchor::TopRight => v1::Anchor::TopRight,
            Anchor::BottomLeft => v1::Anchor::BottomLeft,
            Anchor::BottomRight => v1::Anchor::BottomRight,
        }
    }
}

impl From<Gravity> for popup::v1::Gravity {
    fn from(value: Gravity) -> Self {
        use popup::v1;

        match value {
            Gravity::None => v1::Gravity::None,
            Gravity::Top => v1::Gravity::Top,
            Gravity::Bottom => v1::Gravity::Bottom,
            Gravity::Left => v1::Gravity::Left,
            Gravity::Right => v1::Gravity::Right,
            Gravity::TopLeft => v1::Gravity::TopLeft,
            Gravity::TopRight => v1::Gravity::TopRight,
            Gravity::BottomLeft => v1::Gravity::BottomLeft,
            Gravity::BottomRight => v1::Gravity::BottomRight,
        }
    }
}

impl From<Offset> for popup::v1::Offset {
    fn from(value: Offset) -> Self {
        let Offset { x, y } = value;

        Self { x, y }
    }
}

impl From<ConstraintsAdjust> for popup::v1::ConstraintsAdjust {
    fn from(value: ConstraintsAdjust) -> Self {
        if value == ConstraintsAdjust::None {
            return Self {
                none: true,
                ..Default::default()
            };
        };

        Self {
            none: false,
            slide_x: value.contains(ConstraintsAdjust::SlideX),
            slide_y: value.contains(ConstraintsAdjust::SlideY),
            flip_x: value.contains(ConstraintsAdjust::FlipX),
            flip_y: value.contains(ConstraintsAdjust::FlipY),
            resize_x: value.contains(ConstraintsAdjust::ResizeX),
            resize_y: value.contains(ConstraintsAdjust::ResizeY),
        }
    }
}
