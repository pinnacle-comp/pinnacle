pub mod decoration;
pub mod foreign_toplevel_list;
pub mod foreign_toplevel_management;
pub mod keyboard;
pub mod pointer;

use smithay_client_toolkit::{
    compositor::CompositorHandler,
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_seat,
    output::{OutputHandler, OutputState},
    reexports::{
        client::{
            Connection, Dispatch, QueueHandle, delegate_noop,
            protocol::{
                wl_output::{self, WlOutput},
                wl_region::WlRegion,
                wl_seat::WlSeat,
                wl_surface::WlSurface,
            },
        },
        protocols::wp::{
            fractional_scale::v1::client::{
                wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
                wp_fractional_scale_v1::{self, WpFractionalScaleV1},
            },
            viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter},
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
            // When Smithay gets support for wl_keyboard v10, we can switch to get_keyboard().
            let keyboard = self
                .seat_state
                .get_keyboard_with_repeat(
                    qh,
                    &seat,
                    None,
                    self.loop_handle.clone(),
                    Box::new(State::on_key_repeat),
                )
                .unwrap();

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
        if capability == Capability::Keyboard
            && let Some(keyboard) = self.keyboard.take()
        {
            keyboard.release();
        }

        if capability == Capability::Pointer
            && let Some(pointer) = self.pointer.take()
        {
            pointer.release();
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

    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {}

    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, output: WlOutput) {
        let Some(output_info) = self.output_state.info(&output) else {
            return;
        };

        let Some(size) = output_info.logical_size else {
            return;
        };

        for layer in self
            .layers
            .iter_mut()
            .filter(|layer| layer.wl_output.as_ref() == Some(&output))
        {
            layer.output_size_changed(iced::Size::new(size.0 as u32, size.1 as u32));
            layer.surface.request_frame();
        }
    }

    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, output: WlOutput) {
        self.layers
            .retain(|layer| layer.wl_output.as_ref() != Some(&output));
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
        _qh: &QueueHandle<Self>,
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
            layer.output_size_changed(size);
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
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_factor: i32,
    ) {
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
            return;
        }

        let deco = self
            .decorations
            .iter_mut()
            .find(|deco| &deco.surface.wl_surface == surface);

        if let Some(deco) = deco {
            deco.schedule_redraw();
        }
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
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

        layer.wl_output = Some(output.clone());

        let Some(output_info) = self.output_state.info(output) else {
            return;
        };

        let Some(size) = output_info.logical_size else {
            return;
        };

        let size = iced::Size::new(size.0 as u32, size.1 as u32);

        if let InitialConfigureState::PreConfigure(pending) = &mut layer.initial_configure {
            *pending = Some(size);
            return;
        }

        if layer.initial_configure == InitialConfigureState::PostConfigure {
            return;
        };

        layer.initial_configure = InitialConfigureState::PostOutputSize;

        layer.output_size_changed(size);
        layer.surface.request_frame();
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

delegate_noop!(State: WpFractionalScaleManagerV1);
delegate_noop!(State: WpViewporter);
delegate_noop!(State: WpViewport);
delegate_noop!(State: WlRegion);

impl Dispatch<WpFractionalScaleV1, WlSurface> for State {
    fn event(
        state: &mut Self,
        _proxy: &WpFractionalScaleV1,
        event: <WpFractionalScaleV1 as smithay_client_toolkit::reexports::client::Proxy>::Event,
        surface: &WlSurface,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let Some(layer) = state
            .layers
            .iter_mut()
            .find(|layer| layer.layer.wl_surface() == surface)
        {
            match event {
                wp_fractional_scale_v1::Event::PreferredScale { scale } => {
                    layer.surface.scale_changed(scale as f32 / 120.0);
                    layer.surface.request_frame();
                }
                _ => unreachable!(),
            }
        } else if let Some(deco) = state
            .decorations
            .iter_mut()
            .find(|deco| &deco.surface.wl_surface == surface)
        {
            match event {
                wp_fractional_scale_v1::Event::PreferredScale { scale } => {
                    deco.surface.scale_changed(scale as f32 / 120.0);
                    deco.surface.request_frame();
                }
                _ => unreachable!(),
            }
        }
    }
}
