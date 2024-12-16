use smithay_client_toolkit::{
    delegate_keyboard,
    reexports::client::{
        protocol::{wl_keyboard::WlKeyboard, wl_surface::WlSurface},
        Connection, QueueHandle,
    },
    seat::keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers},
    shell::{wlr_layer::LayerSurface, WaylandSurface},
};
use snowcap_api_defs::snowcap::input::{self, v0alpha1::KeyboardKeyResponse};

use crate::{input::keyboard::keysym_to_iced_key_and_loc, state::State};

impl KeyboardHandler for State {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        surface: &WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[Keysym],
    ) {
        if let Some(layer) = self
            .layers
            .iter()
            .find(|sn_layer| sn_layer.layer.wl_surface() == surface)
        {
            self.keyboard_focus = Some(KeyboardFocus::Layer(layer.layer.clone()));
        }
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        surface: &WlSurface,
        _serial: u32,
    ) {
        if let Some(KeyboardFocus::Layer(layer)) = self.keyboard_focus.as_ref() {
            if layer.wl_surface() == surface {
                self.keyboard_focus = None;
            }
        }
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        let Some(KeyboardFocus::Layer(layer)) = self.keyboard_focus.as_ref() else {
            return;
        };

        let Some(snowcap_layer) = self.layers.iter_mut().find(|sn_l| &sn_l.layer == layer) else {
            return;
        };

        let (key, location) = keysym_to_iced_key_and_loc(event.keysym);

        let mut modifiers = iced::keyboard::Modifiers::empty();
        if self.keyboard_modifiers.ctrl {
            modifiers |= iced::keyboard::Modifiers::CTRL;
        }
        if self.keyboard_modifiers.alt {
            modifiers |= iced::keyboard::Modifiers::ALT;
        }
        if self.keyboard_modifiers.shift {
            modifiers |= iced::keyboard::Modifiers::SHIFT;
        }
        if self.keyboard_modifiers.logo {
            modifiers |= iced::keyboard::Modifiers::LOGO;
        }

        snowcap_layer.widgets.queue_event(iced::Event::Keyboard(
            iced::keyboard::Event::KeyPressed {
                key,
                location,
                modifiers,
                text: None,
            },
        ));

        if let Some(sender) = snowcap_layer.keyboard_key_sender.as_ref() {
            let api_modifiers = input::v0alpha1::Modifiers {
                shift: Some(self.keyboard_modifiers.shift),
                ctrl: Some(self.keyboard_modifiers.ctrl),
                alt: Some(self.keyboard_modifiers.alt),
                super_: Some(self.keyboard_modifiers.logo),
            };
            let _ = sender.send(Ok(KeyboardKeyResponse {
                key: Some(event.keysym.raw()),
                modifiers: Some(api_modifiers),
                pressed: Some(true),
            }));
        }
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        let Some(KeyboardFocus::Layer(layer)) = self.keyboard_focus.as_ref() else {
            return;
        };

        let Some(snowcap_layer) = self.layers.iter_mut().find(|sn_l| &sn_l.layer == layer) else {
            return;
        };

        let (key, location) = keysym_to_iced_key_and_loc(event.keysym);

        let mut modifiers = iced::keyboard::Modifiers::empty();
        if self.keyboard_modifiers.ctrl {
            modifiers |= iced::keyboard::Modifiers::CTRL;
        }
        if self.keyboard_modifiers.alt {
            modifiers |= iced::keyboard::Modifiers::ALT;
        }
        if self.keyboard_modifiers.shift {
            modifiers |= iced::keyboard::Modifiers::SHIFT;
        }
        if self.keyboard_modifiers.logo {
            modifiers |= iced::keyboard::Modifiers::LOGO;
        }

        snowcap_layer.widgets.queue_event(iced::Event::Keyboard(
            iced::keyboard::Event::KeyReleased {
                key,
                location,
                modifiers,
            },
        ));
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
        _layout: u32,
    ) {
        // TODO: per wl_keyboard
        self.keyboard_modifiers = modifiers;
    }
}
delegate_keyboard!(State);

pub enum KeyboardFocus {
    Layer(LayerSurface),
}
