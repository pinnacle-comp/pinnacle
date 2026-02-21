pub mod decoration;
pub mod foreign_toplevel_list;
pub mod foreign_toplevel_management;
pub mod keyboard;
pub mod pointer;

use smithay_client_toolkit::{
    compositor::CompositorHandler,
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_seat,
    delegate_xdg_popup, delegate_xdg_shell,
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
        xdg::{
            popup::{self, PopupHandler},
            window::WindowHandler,
        },
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

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, seat: WlSeat) {
        // TODO: For now we only support one seat. This is good enough as most compositor only
        // support one seat as well, but could be improved either by picking the best seat (the one
        // with the most desirable capabilities), or having the user pick a seat by name.
        if self.seat.is_none() {
            self.seat = Some(seat);
        }
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
            let cursor_shape_device = self
                .cursor_shape_manager
                .get_shape_device(&pointer, &self.queue_handle);

            self.pointer = Some(pointer);
            self.cursor_shape_device = Some(cursor_shape_device);
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

        if capability == Capability::Pointer {
            if let Some(pointer) = self.pointer.take() {
                pointer.release();
            }
            if let Some(device) = self.cursor_shape_device.take() {
                device.destroy();
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

        for popup in self
            .popups
            .iter_mut()
            .filter(|p| p.wl_output.as_ref() == Some(&output))
        {
            popup.output_size_changed(iced::Size::new(size.0 as u32, size.1 as u32));
            popup.surface.request_frame();
        }
    }

    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, output: WlOutput) {
        let to_delete: Vec<_> = self
            .layers
            .iter()
            .filter_map(|layer| {
                if layer.wl_output.as_ref() == Some(&output) {
                    Some(layer.layer_id)
                } else {
                    None
                }
            })
            .collect();

        for layer_id in to_delete {
            self.layer_destroy(layer_id);
        }
    }
}
delegate_output!(State);

impl LayerShellHandler for State {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, layer: &LayerSurface) {
        if let Some(layer_id) = self
            .layers
            .iter()
            .find(|l| &l.layer == layer)
            .map(|l| l.layer_id)
        {
            self.layer_destroy(layer_id);
        }
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
            return;
        }

        let popup = self
            .popups
            .iter_mut()
            .find(|popup| popup.popup.wl_surface() == surface);

        if let Some(popup) = popup {
            popup.schedule_redraw();
        }
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface: &WlSurface,
        output: &wl_output::WlOutput,
    ) {
        let Some(output_info) = self.output_state.info(output) else {
            return;
        };

        let Some(size) = output_info.logical_size else {
            return;
        };

        let size = iced::Size::new(size.0 as u32, size.1 as u32);

        if let Some(layer) = self
            .layers
            .iter_mut()
            .find(|layer| layer.layer.wl_surface() == surface)
        {
            layer.wl_output = Some(output.clone());

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
        } else if let Some(popup) = self
            .popups
            .iter_mut()
            .find(|p| p.popup.wl_surface() == surface)
        {
            popup.wl_output = Some(output.clone());

            popup.output_size_changed(size);

            if popup.initial_configure_received {
                popup.surface.request_frame()
            }
        }
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
        let surface = if let Some(layer) = state
            .layers
            .iter_mut()
            .find(|layer| layer.layer.wl_surface() == surface)
        {
            &mut layer.surface
        } else if let Some(deco) = state
            .decorations
            .iter_mut()
            .find(|deco| &deco.surface.wl_surface == surface)
        {
            &mut deco.surface
        } else if let Some(popup) = state
            .popups
            .iter_mut()
            .find(|popup| &popup.surface.wl_surface == surface)
        {
            &mut popup.surface
        } else {
            return;
        };

        match event {
            wp_fractional_scale_v1::Event::PreferredScale { scale } => {
                surface.scale_changed(scale as f32 / 120.0);
                surface.request_frame();
            }
            _ => unreachable!(),
        }
    }
}

impl WindowHandler for State {
    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _window: &smithay_client_toolkit::shell::xdg::window::Window,
        _configure: smithay_client_toolkit::shell::xdg::window::WindowConfigure,
        _serial: u32,
    ) {
        unimplemented!()
    }

    fn request_close(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _window: &smithay_client_toolkit::shell::xdg::window::Window,
    ) {
        unimplemented!()
    }
}

delegate_xdg_shell!(State);

impl PopupHandler for State {
    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        popup: &smithay_client_toolkit::shell::xdg::popup::Popup,
        config: smithay_client_toolkit::shell::xdg::popup::PopupConfigure,
    ) {
        let Some(popup) = self.popups.iter_mut().find(|p| &p.popup == popup) else {
            return;
        };

        popup.size_changed(iced::Size::new(config.width as u32, config.height as u32));

        match config.kind {
            popup::ConfigureKind::Initial => {
                popup.initial_configure_received = true;
                popup.schedule_redraw();
            }
            popup::ConfigureKind::Reposition { token } => {
                popup.repositioned(Some(token));
            }
            popup::ConfigureKind::Reactive => {
                popup.repositioned(None);
            }
            _ => unreachable!(),
        };
    }

    fn done(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        popup: &smithay_client_toolkit::shell::xdg::popup::Popup,
    ) {
        if let Some(popup_id) = self
            .popups
            .iter()
            .find(|p| &p.popup == popup)
            .map(|p| p.popup_id)
        {
            self.popup_destroy(popup_id);
        };
    }
}
delegate_xdg_popup!(State);
