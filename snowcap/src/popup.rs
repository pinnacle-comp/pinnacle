use iced_runtime::core::widget;
use smithay_client_toolkit::{
    reexports::{
        client::protocol::wl_output::WlOutput,
        protocols::xdg::shell::client::xdg_positioner::ConstraintAdjustment,
    },
    shell::xdg::{XdgPositioner, popup::Popup},
};

use crate::{
    decoration::DecorationId, layer::LayerId, state::State, surface::SnowcapSurface, widget::ViewFn,
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

    _current_size: iced::Size<u32>,
    pending_size: Option<iced::Size<u32>>,
}

impl SnowcapPopup {
    pub fn new(
        state: &mut State,
        parent_id: ParentId,
        positioner: XdgPositioner,
        widgets: ViewFn,
    ) -> Option<Self> {
        let mut surface = SnowcapSurface::new(state, widgets, false);

        positioner.set_size(150, 1);
        positioner
            .set_constraint_adjustment(ConstraintAdjustment::SlideY | ConstraintAdjustment::SlideX);
        positioner.set_reactive();

        let (popup, toplevel_id) = match parent_id {
            ParentId::Popup(id) => {
                let p = state.popups.iter().find(|p| p.popup_id == id)?;

                let popup = Popup::from_surface(
                    Some(p.popup.xdg_surface()),
                    &positioner,
                    &state.queue_handle,
                    surface.wl_surface.clone(),
                    &state.xdg_shell,
                )
                .ok()?;

                (popup, p.toplevel_id)
            }
            ParentId::Layer(id) => {
                let l = state.layers.iter().find(|l| l.layer_id == id)?;
                let popup = Popup::from_surface(
                    None,
                    &positioner,
                    &state.queue_handle,
                    surface.wl_surface.clone(),
                    &state.xdg_shell,
                )
                .ok()?;

                l.layer.get_popup(popup.xdg_popup());

                (popup, parent_id)
            }
            _ => unreachable!(),
        };

        popup.wl_surface().commit();

        match toplevel_id {
            ParentId::Layer(id) => {
                let layer = state.layers.iter().find(|l| l.layer_id == id)?;
                surface.toplevel_wl_surface = Some(layer.surface.wl_surface.clone());

                // Popup don't receive frames unless the toplevel does.
                layer.surface.request_frame();
            }
            _ => unreachable!(),
        };

        let next_id = state.popup_id_counter.next();

        Some(Self {
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

            _current_size: iced::Size {
                width: 1,
                height: 1,
            },
            pending_size: None,
        })
    }

    pub fn request_view(&mut self) {
        self.surface.request_view();
    }

    pub fn schedule_redraw(&mut self) {
        self.surface.schedule_redraw();
    }

    pub fn update_properties(&mut self, widgets: Option<ViewFn>) {
        if let Some(widgets) = widgets {
            self.surface.view_changed(widgets);
        }

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
        if let Some(pending_output_size) = self.pending_output_size.take() {
            self.output_size = pending_output_size;
        }

        self.surface.bounds_changed(self.widget_bounds());

        let resized = self.surface.update(runtime, compositor);

        if resized {
            self.positioner.set_size(
                self.surface.widgets.size().width as i32,
                self.surface.widgets.size().height as i32,
            );

            let token = self.reposition_token;
            self.reposition_token += 1;
            self.pending_reposition = Some(token);
            self.popup.reposition(&self.positioner, token);
        }
    }

    pub fn widget_bounds(&self) -> iced::Size<u32> {
        self.output_size
    }

    pub fn size_changed(&mut self, new_size: iced::Size<u32>) {
        self.pending_size = Some(new_size);
    }

    pub fn output_size_changed(&mut self, new_size: iced::Size<u32>) {
        self.pending_output_size = Some(new_size);
    }

    pub fn repositioned(&mut self, token: Option<u32>) {
        if self.pending_reposition == token {
            self.pending_reposition = None;
            self.schedule_redraw();
        }
    }
}
