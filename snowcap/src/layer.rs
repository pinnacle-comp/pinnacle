use std::num::NonZeroU32;

use smithay_client_toolkit::{
    reexports::client::protocol::wl_output::WlOutput,
    shell::{
        WaylandSurface,
        wlr_layer::{self, Anchor, LayerSurface},
    },
};
use snowcap_api_defs::snowcap::input::v0alpha1::PointerButtonResponse;
use tokio::sync::mpsc::UnboundedSender;
use tonic::Status;

use crate::{
    handlers::keyboard::KeyboardKey, state::State, surface::SnowcapSurface, widget::ViewFn,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct LayerId(pub u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct LayerIdCounter(LayerId);

impl LayerIdCounter {
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> LayerId {
        let ret = self.0;
        self.0.0 += 1;
        ret
    }
}

impl State {
    pub fn layer_for_id(&mut self, id: LayerId) -> Option<&mut SnowcapLayer> {
        self.layers.iter_mut().find(|layer| layer.layer_id == id)
    }
}

pub struct SnowcapLayer {
    // Drop order: `surface` needs to be dropped first as
    // `layer` will also attempt to destroy the wl_surface.
    pub surface: SnowcapSurface,
    pub layer: LayerSurface,

    /// The logical size of the output this layer is on.
    output_size: iced::Size<u32>,
    pending_output_size: Option<iced::Size<u32>>,
    // COMPAT: 0.1
    max_size: Option<iced::Size<u32>>,

    pub layer_id: LayerId,

    pub wl_output: Option<WlOutput>,

    pub keyboard_key_sender: Option<UnboundedSender<KeyboardKey>>,
    pub pointer_button_sender: Option<UnboundedSender<Result<PointerButtonResponse, Status>>>,

    pub initial_configure: InitialConfigureState,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum InitialConfigureState {
    PreConfigure(Option<iced::Size<u32>>),
    PostConfigure,
    PostOutputSize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ExclusiveZone {
    /// This layer surface wants an exclusive zone of the given size.
    Exclusive(NonZeroU32),
    /// This layer surface does not have an exclusive zone but wants to be placed respecting any.
    Respect,
    /// This layer surface does not have an exclusive zone and wants to be placed ignoring any.
    Ignore,
}

impl SnowcapLayer {
    pub fn new(
        state: &mut State,
        // COMPAT: 0.1
        max_size: Option<(u32, u32)>,
        layer: wlr_layer::Layer,
        anchor: Anchor,
        exclusive_zone: ExclusiveZone,
        keyboard_interactivity: wlr_layer::KeyboardInteractivity,
        widgets: ViewFn,
    ) -> Self {
        let surface = SnowcapSurface::new(state, widgets, false);

        let layer = state.layer_shell_state.create_layer_surface(
            &state.queue_handle,
            surface.wl_surface.clone(),
            layer,
            Some("snowcap"),
            None,
        );

        layer.set_size(1, 1);
        layer.set_anchor(anchor);
        layer.set_keyboard_interactivity(keyboard_interactivity);
        layer.set_exclusive_zone(match exclusive_zone {
            ExclusiveZone::Exclusive(size) => size.get() as i32,
            ExclusiveZone::Respect => 0,
            ExclusiveZone::Ignore => -1,
        });

        layer.commit();

        let next_id = state.layer_id_counter.next();

        Self {
            surface,
            layer,
            max_size: max_size.map(|(w, h)| iced::Size::new(w, h)),
            output_size: iced::Size::new(1, 1),
            pending_output_size: None,
            wl_output: None,
            layer_id: next_id,
            keyboard_key_sender: None,
            pointer_button_sender: None,
            initial_configure: InitialConfigureState::PreConfigure(None),
        }
    }

    pub fn request_view(&mut self) {
        self.surface.request_view();
    }

    pub fn schedule_redraw(&mut self) {
        self.surface.schedule_redraw();
    }

    pub fn update_properties(
        &mut self,
        layer: Option<wlr_layer::Layer>,
        anchor: Option<Anchor>,
        exclusive_zone: Option<ExclusiveZone>,
        keyboard_interactivity: Option<wlr_layer::KeyboardInteractivity>,
        widgets: Option<ViewFn>,
    ) {
        if let Some(widgets) = widgets {
            self.surface.view_changed(widgets);
        }

        if let Some(layer) = layer {
            self.layer.set_layer(layer);
        }

        if let Some(anchor) = anchor {
            self.layer.set_anchor(anchor);
        }

        if let Some(zone) = exclusive_zone {
            self.layer.set_exclusive_zone(match zone {
                ExclusiveZone::Exclusive(size) => size.get() as i32,
                ExclusiveZone::Respect => 0,
                ExclusiveZone::Ignore => -1,
            });
        }

        if let Some(keyboard_interactivity) = keyboard_interactivity {
            self.layer
                .set_keyboard_interactivity(keyboard_interactivity);
        }

        self.surface.request_frame();
    }

    pub fn draw_if_scheduled(&mut self) {
        self.surface.draw_if_scheduled();
    }

    pub fn update(
        &mut self,
        runtime: &mut crate::runtime::Runtime,
        compositor: &mut crate::compositor::Compositor,
    ) {
        if let Some(pending_size) = self.pending_output_size.take() {
            self.output_size = pending_size;
        }

        self.surface.bounds_changed(self.widget_bounds());

        let resized = self.surface.update(runtime, compositor);

        if resized {
            self.layer.set_size(
                self.surface.widgets.size().width,
                self.surface.widgets.size().height,
            );
        }
    }

    pub fn widget_bounds(&self) -> iced::Size<u32> {
        if let Some(max_size) = self.max_size {
            iced::Size::new(
                self.output_size.width.min(max_size.width),
                self.output_size.height.min(max_size.height),
            )
        } else {
            self.output_size
        }
    }

    pub fn output_size_changed(&mut self, new_size: iced::Size<u32>) {
        self.pending_output_size = Some(new_size);
    }
}
