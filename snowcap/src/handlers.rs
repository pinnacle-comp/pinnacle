pub mod keyboard;
pub mod pointer;

use std::time::Instant;

use iced_wgpu::graphics::Viewport;
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

use crate::state::State;

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
        let layer = self.layers.iter_mut().find(|l| &l.layer == layer);

        if let Some(layer) = layer {
            if !layer.initial_configure {
                layer.initial_configure = true;
                layer.update(qh, &mut self.runtime);
                layer.widgets.queue_event(iced::Event::Window(
                    iced::window::Event::RedrawRequested(Instant::now()),
                ));
                layer.redraw_requested = true;
            }
        }
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
        if let Some(layer) = self
            .layers
            .iter_mut()
            .find(|sn_layer| sn_layer.layer.wl_surface() == surface)
        {
            layer.viewport = Viewport::with_physical_size(
                iced::Size::new(
                    layer.width * new_factor as u32,
                    layer.height * new_factor as u32,
                ),
                new_factor as f64,
            );
            layer.set_scale(
                new_factor,
                self.compositor.as_mut().expect("should be initialized"),
            );
            layer
                .layer
                .wl_surface()
                .frame(qh, layer.layer.wl_surface().clone());
            layer.layer.wl_surface().commit();
        }
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_transform: wl_output::Transform,
    ) {
        // TODO:
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
            if !layer.redraw_requested {
                layer.widgets.queue_event(iced::Event::Window(
                    iced::window::Event::RedrawRequested(Instant::now()),
                ));
                layer.redraw_requested = true;
            }
        }
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        // TODO:
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        // TODO:
    }
}
delegate_compositor!(State);
