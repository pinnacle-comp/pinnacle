pub mod keyboard;
pub mod pointer;

use smithay_client_toolkit::{
    compositor::CompositorHandler,
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_seat,
    output::{OutputHandler, OutputState},
    reexports::client::{
        Connection, QueueHandle,
        protocol::{
            wl_output::{self, WlOutput},
            wl_seat::WlSeat,
            wl_surface::WlSurface,
        },
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{Capability, SeatHandler, SeatState},
    shell::{
        WaylandSurface,
        wlr_layer::{LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
    },
};

use crate::{layer::InitialConfigureState, state::State};

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers!(OutputState, SeatState);
}
delegate_registry!(State);

impl SeatHandler for State {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat) {
        // TODO:
    }

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            let keyboard = self.seat_state.get_keyboard(qh, &seat, None).unwrap();
            self.keyboard = Some(keyboard);
        }

        if capability == Capability::Pointer && self.pointer.is_none() {
            let pointer = self.seat_state.get_pointer(qh, &seat).unwrap();
            self.pointer = Some(pointer);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard {
            if let Some(keyboard) = self.keyboard.take() {
                keyboard.release();
            }
        }

        if capability == Capability::Pointer {
            if let Some(pointer) = self.pointer.take() {
                pointer.release();
            }
        }
    }

    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat) {
        // TODO:
    }
}
delegate_seat!(State);

impl OutputHandler for State {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {
        // TODO:
    }

    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {
        // TODO:
    }

    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {
        // TODO:
    }
}
delegate_output!(State);

impl LayerShellHandler for State {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, layer: &LayerSurface) {
        self.layers.retain(|sn_layer| &sn_layer.layer != layer);
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        _configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let Some(layer) = self.layers.iter_mut().find(|l| &l.layer == layer) else {
            return;
        };

        let InitialConfigureState::PreConfigure(size) = layer.initial_configure else {
            return;
        };

        if let Some(size) = size {
            layer.pending_size = Some((size.0 as u32, size.1 as u32));
            layer.initial_configure = InitialConfigureState::PostOutputSize;
        } else {
            layer.initial_configure = InitialConfigureState::PostConfigure;
        }

        layer.schedule_redraw();
    }
}
delegate_layer!(State);

impl CompositorHandler for State {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: &WlSurface,
        new_factor: i32,
    ) {
        let Some(layer) = self
            .layers
            .iter_mut()
            .find(|sn_layer| sn_layer.layer.wl_surface() == surface)
        else {
            return;
        };

        layer.output_size_changed(layer.output_width, layer.output_height, new_factor);
        layer.request_frame(qh);
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface: &WlSurface,
        _time: u32,
    ) {
        let layer = self
            .layers
            .iter_mut()
            .find(|layer| layer.layer.wl_surface() == surface);

        if let Some(layer) = layer {
            layer.schedule_redraw();
        }
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: &WlSurface,
        output: &wl_output::WlOutput,
    ) {
        let Some(layer) = self
            .layers
            .iter_mut()
            .find(|layer| layer.layer.wl_surface() == surface)
        else {
            return;
        };

        let Some(output_info) = self.output_state.info(output) else {
            return;
        };

        let Some(size) = output_info.logical_size else {
            return;
        };

        if let InitialConfigureState::PreConfigure(pending) = &mut layer.initial_configure {
            *pending = Some(size);
            return;
        }

        if layer.initial_configure == InitialConfigureState::PostConfigure {
            return;
        };

        layer.initial_configure = InitialConfigureState::PostOutputSize;

        layer.output_size_changed(size.0 as u32, size.1 as u32, layer.output_scale);

        layer.request_frame(qh);
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}
delegate_compositor!(State);
