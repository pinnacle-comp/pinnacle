use iced::keyboard::key::{NativeCode, Physical};
use smithay_client_toolkit::{
    delegate_keyboard,
    reexports::client::{
        Connection, QueueHandle,
        protocol::{wl_keyboard::WlKeyboard, wl_surface::WlSurface},
    },
    seat::keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers},
    shell::{WaylandSurface, wlr_layer::LayerSurface},
};

use crate::{input::keyboard::keysym_to_iced_key_and_loc, state::State};

#[derive(Clone, Copy, Debug)]
pub struct KeyboardKey {
    pub key: Keysym,
    pub modifiers: Modifiers,
    pub pressed: bool,
}

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
        if let Some(KeyboardFocus::Layer(layer)) = self.keyboard_focus.as_ref()
            && layer.wl_surface() == surface
        {
            self.keyboard_focus = None;
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

        snowcap_layer
            .surface
            .widgets
            .queue_event(iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key: key.clone(),
                location,
                modifiers,
                text: None,
                modified_key: key, // TODO:
                physical_key: Physical::Unidentified(NativeCode::Xkb(event.keysym.raw())),
            }));
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

        snowcap_layer
            .surface
            .widgets
            .queue_event(iced::Event::Keyboard(iced::keyboard::Event::KeyReleased {
                key: key.clone(),
                location,
                modifiers,
                // TODO:
                modified_key: key,
                physical_key: Physical::Unidentified(NativeCode::Xkb(event.keysym.raw())),
            }));
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
