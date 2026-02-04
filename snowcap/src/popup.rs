use iced_runtime::core::widget::{
    self,
    operation::{self, Operation, Outcome},
};
use smithay_client_toolkit::{
    reexports::{
        client::protocol::wl_output::WlOutput,
        protocols::xdg::shell::client::xdg_positioner::{self, ConstraintAdjustment},
    },
    shell::xdg::{XdgPositioner, popup::Popup},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    decoration::DecorationId, handlers::keyboard::KeyboardKey, layer::LayerId, state::State,
    surface::SnowcapSurface, widget::ViewFn,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct PopupId(pub u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct PopupIdCounter(PopupId);

impl PopupIdCounter {
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> PopupId {
        let ret = self.0;
        self.0.0 += 1;
        ret
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ParentId {
    Layer(LayerId),
    Decoration(DecorationId),
    Popup(PopupId),
}

impl State {
    pub fn popup_for_id(&mut self, id: PopupId) -> Option<&mut SnowcapPopup> {
        self.popups.iter_mut().find(|popup| popup.popup_id == id)
    }

    pub fn popup_destroy(&mut self, id: PopupId) {
        let mut to_destroy = vec![id];

        while let Some(destroy_first) = to_destroy.last().and_then(|id| {
            self.popups
                .iter()
                .find(|p| p.parent_id == ParentId::Popup(*id))
        }) {
            to_destroy.push(destroy_first.popup_id)
        }

        for popup_id in to_destroy.iter().rev() {
            self.popups.retain(|p| &p.popup_id != popup_id)
        }
    }
}

pub struct Offset {
    pub x: i32,
    pub y: i32,
}

pub enum Position {
    AtCursor,
    Absolute {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    Widget(String),
}

impl Position {
    pub fn anchor_rect_for(&self, surface: &mut SnowcapSurface) -> Option<iced::Rectangle<i32>> {
        match self {
            Position::AtCursor => surface.pointer_location.map(|(x, y)| iced::Rectangle {
                x: x as i32,
                y: y as i32,
                width: 1,
                height: 1,
            }),
            &Position::Absolute {
                x,
                y,
                width,
                height,
            } => {
                let size = surface.widgets.size();
                let bounds = iced::Rectangle::with_size(size);
                let anchor = iced::Rectangle {
                    x,
                    y,
                    width,
                    height,
                };

                if anchor.is_within(&bounds.into()) {
                    Some(iced::Rectangle {
                        x: x as i32,
                        y: y as i32,
                        width: width as i32,
                        height: height as i32,
                    })
                } else {
                    None
                }
            }
            Position::Widget(id) => {
                let mut ope = get_bounds(id.clone().into());

                {
                    let mut black_box = operation::black_box(&mut ope);

                    surface.operate(&mut black_box);
                }

                match ope.finish() {
                    Outcome::Some(bounds) => Some(bounds),
                    Outcome::None => None,
                    _ => unreachable!(),
                }
                .map(|b| {
                    let iced::Rectangle {
                        x,
                        y,
                        width,
                        height,
                    } = b;
                    iced::Rectangle {
                        x: x as i32,
                        y: y as i32,
                        width: width as i32,
                        height: height as i32,
                    }
                })
            }
        }
    }
}

pub fn get_bounds(target: widget::Id) -> impl Operation<iced::Rectangle> {
    struct GetBounds {
        target: widget::Id,
        bounds: Option<iced::Rectangle>,
    }

    impl Operation<iced::Rectangle> for GetBounds {
        fn container(&mut self, id: Option<&widget::Id>, bounds: iced::Rectangle) {
            if id.is_some_and(|id| *id == self.target) {
                self.bounds = Some(bounds);
            }
        }

        fn text(&mut self, id: Option<&widget::Id>, bounds: iced::Rectangle, _text: &str) {
            if id.is_some_and(|id| *id == self.target) {
                self.bounds = Some(bounds);
            }
        }

        fn custom(
            &mut self,
            id: Option<&widget::Id>,
            bounds: iced::Rectangle,
            _state: &mut dyn std::any::Any,
        ) {
            if id.is_some_and(|id| *id == self.target) {
                self.bounds = Some(bounds);
            }
        }

        fn focusable(
            &mut self,
            id: Option<&widget::Id>,
            bounds: iced::Rectangle,
            _state: &mut dyn widget::operation::Focusable,
        ) {
            if id.is_some_and(|id| *id == self.target) {
                self.bounds = Some(bounds);
            }
        }

        fn scrollable(
            &mut self,
            id: Option<&widget::Id>,
            bounds: iced::Rectangle,
            _content_bounds: iced::Rectangle,
            _translation: iced::Vector,
            _state: &mut dyn widget::operation::Scrollable,
        ) {
            if id.is_some_and(|id| *id == self.target) {
                self.bounds = Some(bounds);
            }
        }

        fn text_input(
            &mut self,
            id: Option<&widget::Id>,
            bounds: iced::Rectangle,
            _state: &mut dyn widget::operation::TextInput,
        ) {
            if id.is_some_and(|id| *id == self.target) {
                self.bounds = Some(bounds);
            }
        }

        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<iced::Rectangle>)) {
            if self.bounds.is_some() {
                return;
            }

            operate(self);
        }

        fn finish(&self) -> Outcome<iced::Rectangle> {
            self.bounds.map_or(Outcome::None, Outcome::Some)
        }
    }

    GetBounds {
        target,
        bounds: None,
    }
}

pub enum Error {
    Positioner,
    InvalidPosition,
    ParentNotFound,
    ToplevelNotFound,
    CreateFailed,
}

pub struct SnowcapPopup {
    pub surface: SnowcapSurface,
    pub popup: Popup,

    pub popup_id: PopupId,
    pub parent_id: ParentId,
    pub toplevel_id: ParentId,

    pub initial_configure_received: bool,

    positioner: XdgPositioner,

    pub wl_output: Option<WlOutput>,
    output_size: iced::Size<u32>,
    pending_output_size: Option<iced::Size<u32>>,

    reposition_token: u32,
    pending_reposition: Option<u32>,

    recompute_size: bool,
    current_size: iced::Size<u32>,
    pending_size: Option<iced::Size<u32>>,

    pub keyboard_key_sender: Option<UnboundedSender<KeyboardKey>>,
}

impl SnowcapPopup {
    pub fn new(
        state: &mut State,
        parent_id: ParentId,
        position: Position,
        anchor: Option<xdg_positioner::Anchor>,
        gravity: Option<xdg_positioner::Gravity>,
        offset: Option<Offset>,
        constraints_adjustment: Option<ConstraintAdjustment>,
        grab_keyboard: bool,
        widgets: ViewFn,
    ) -> Result<Self, Error> {
        let mut surface = SnowcapSurface::new(state, widgets, false);

        let Ok(positioner) = XdgPositioner::new(&state.xdg_shell) else {
            return Err(Error::Positioner);
        };

        if let Some(anchor) = anchor {
            positioner.set_anchor(anchor);
        }

        if let Some(gravity) = gravity {
            positioner.set_gravity(gravity);
        }

        if let Some(Offset { x, y }) = offset {
            positioner.set_offset(x, y);
        }

        if let Some(adjustment) = constraints_adjustment {
            positioner.set_constraint_adjustment(adjustment);
        }
        positioner.set_size(1, 1);
        positioner.set_reactive();

        let (popup, toplevel_id, focus_serial) = match parent_id {
            ParentId::Popup(id) => {
                let p = state
                    .popups
                    .iter_mut()
                    .find(|p| p.popup_id == id)
                    .ok_or(Error::ParentNotFound)?;

                let iced::Rectangle {
                    x,
                    y,
                    width,
                    height,
                } = position
                    .anchor_rect_for(&mut p.surface)
                    .ok_or(Error::InvalidPosition)?;

                positioner.set_anchor_rect(x, y, width, height);

                let Ok(popup) = Popup::from_surface(
                    Some(p.popup.xdg_surface()),
                    &positioner,
                    &state.queue_handle,
                    surface.wl_surface.clone(),
                    &state.xdg_shell,
                ) else {
                    return Err(Error::CreateFailed);
                };

                (popup, p.toplevel_id, p.surface.focus_serial)
            }
            ParentId::Layer(id) => {
                let l = state
                    .layers
                    .iter_mut()
                    .find(|l| l.layer_id == id)
                    .ok_or(Error::ParentNotFound)?;

                let iced::Rectangle {
                    x,
                    y,
                    width,
                    height,
                } = position
                    .anchor_rect_for(&mut l.surface)
                    .ok_or(Error::InvalidPosition)?;

                positioner.set_anchor_rect(x, y, width, height);

                let Ok(popup) = Popup::from_surface(
                    None,
                    &positioner,
                    &state.queue_handle,
                    surface.wl_surface.clone(),
                    &state.xdg_shell,
                ) else {
                    return Err(Error::CreateFailed);
                };

                l.layer.get_popup(popup.xdg_popup());

                (popup, parent_id, l.surface.focus_serial)
            }
            ParentId::Decoration(id) => {
                let deco = state
                    .decorations
                    .iter_mut()
                    .find(|deco| deco.decoration_id == id)
                    .ok_or(Error::ParentNotFound)?;

                let iced::Rectangle {
                    x,
                    y,
                    width,
                    height,
                } = position
                    .anchor_rect_for(&mut deco.surface)
                    .ok_or(Error::InvalidPosition)?;

                positioner.set_anchor_rect(x, y, width, height);

                let Ok(popup) = Popup::from_surface(
                    None,
                    &positioner,
                    &state.queue_handle,
                    surface.wl_surface.clone(),
                    &state.xdg_shell,
                ) else {
                    return Err(Error::CreateFailed);
                };

                deco.decoration.get_popup(popup.xdg_popup());

                (popup, parent_id, deco.surface.focus_serial)
            }
        };

        if grab_keyboard {
            if let Some(serial) = focus_serial
                && let Some(seat) = state.seat.as_ref()
            {
                popup.xdg_popup().grab(seat, serial);
            } else {
                tracing::error!(
                    "Could not grab keyboard for popup with parent {:?}",
                    parent_id
                );
            }
        }
        popup.wl_surface().commit();

        match toplevel_id {
            ParentId::Layer(id) => {
                let layer = state
                    .layers
                    .iter()
                    .find(|l| l.layer_id == id)
                    .ok_or(Error::ToplevelNotFound)?;
                surface.toplevel_wl_surface = Some(layer.surface.wl_surface.clone());

                // Popup don't receive frames unless the toplevel does.
                layer.surface.request_frame();
            }
            ParentId::Decoration(id) => {
                let deco = state
                    .decorations
                    .iter()
                    .find(|deco| deco.decoration_id == id)
                    .ok_or(Error::ToplevelNotFound)?;

                surface.toplevel_wl_surface = Some(deco.surface.wl_surface.clone());

                deco.surface.request_frame();
            }
            _ => unreachable!(),
        };

        let next_id = state.popup_id_counter.next();

        Ok(Self {
            surface,
            popup,
            popup_id: next_id,
            parent_id,
            toplevel_id,
            initial_configure_received: false,

            positioner,
            wl_output: None,
            output_size: iced::Size {
                width: 1,
                height: 1,
            },
            pending_output_size: None,

            reposition_token: 0,
            pending_reposition: None,

            recompute_size: false,
            current_size: iced::Size {
                width: 1,
                height: 1,
            },
            pending_size: None,

            keyboard_key_sender: None,
        })
    }

    fn send_reposition(&mut self) {
        let token = self.reposition_token;
        self.reposition_token += 1;
        self.pending_reposition = Some(token);
        self.popup.reposition(&self.positioner, token);
    }

    pub fn request_view(&mut self) {
        self.surface.request_view();
    }

    pub fn schedule_redraw(&mut self) {
        self.surface.schedule_redraw();
    }

    pub fn update_properties(
        &mut self,
        anchor_rect: Option<iced::Rectangle<i32>>,
        anchor: Option<xdg_positioner::Anchor>,
        gravity: Option<xdg_positioner::Gravity>,
        offset: Option<Offset>,
        constraints_adjustment: Option<ConstraintAdjustment>,
        widgets: Option<ViewFn>,
    ) {
        let mut update_pos = false;

        if let Some(iced::Rectangle {
            x,
            y,
            width,
            height,
        }) = anchor_rect
        {
            self.positioner.set_anchor_rect(x, y, width, height);
            update_pos = true;
        }

        if let Some(anchor) = anchor {
            self.positioner.set_anchor(anchor);
            update_pos = true;
        }

        if let Some(gravity) = gravity {
            self.positioner.set_gravity(gravity);
            update_pos = true;
        }

        if let Some(Offset { x, y }) = offset {
            self.positioner.set_offset(x, y);
            update_pos = true;
        }

        if let Some(adjustment) = constraints_adjustment {
            self.positioner.set_constraint_adjustment(adjustment);
            update_pos = true;
        }

        if let Some(widgets) = widgets {
            self.surface.view_changed(widgets);
        }

        if update_pos {
            self.send_reposition();
        }

        self.recompute_size = true;
        self.surface.request_frame();
    }

    pub fn draw_if_scheduled(&mut self) {
        if self.pending_reposition.is_none() {
            self.surface.draw_if_scheduled();
        }
    }

    pub fn operate(&mut self, operation: &mut dyn widget::Operation) {
        self.surface.operate(operation);
    }

    pub fn update(
        &mut self,
        runtime: &mut crate::runtime::Runtime,
        compositor: &mut crate::compositor::Compositor,
    ) {
        if let Some(pending_size) = self.pending_size.take() {
            self.current_size = pending_size;
        }

        if let Some(pending_output_size) = self.pending_output_size.take() {
            self.output_size = pending_output_size;
        }

        self.surface.bounds_changed(self.widget_bounds());

        let resized = self.surface.update(runtime, compositor);

        if resized {
            let iced::Size { width, height } = self.surface.widgets.size();
            self.current_size = iced::Size::new(width, height);

            // INFO: Our size may have increased. Let's inform the server.
            if self.recompute_size {
                self.positioner.set_size(width as i32, height as i32);

                self.send_reposition();
            }
        }

        self.recompute_size = false;
    }

    pub fn widget_bounds(&self) -> iced::Size<u32> {
        if self.recompute_size {
            self.output_size
        } else {
            self.current_size
        }
    }

    pub fn size_changed(&mut self, new_size: iced::Size<u32>) {
        self.pending_size = Some(new_size);
    }

    pub fn output_size_changed(&mut self, new_size: iced::Size<u32>) {
        self.pending_output_size = Some(new_size);
        self.recompute_size = true;
    }

    pub fn repositioned(&mut self, token: Option<u32>) {
        if self.pending_reposition == token {
            self.pending_reposition = None;
            self.schedule_redraw();
        }
    }
}
