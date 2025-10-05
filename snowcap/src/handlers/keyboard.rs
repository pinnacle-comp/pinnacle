use iced::keyboard::key::{NativeCode, Physical};
use smithay_client_toolkit::{
    delegate_keyboard,
    reexports::client::{
        Connection, QueueHandle,
        protocol::{wl_keyboard::WlKeyboard, wl_surface::WlSurface},
    },
    seat::keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
    shell::{WaylandSurface, wlr_layer::LayerSurface},
};

use crate::{input::keyboard::keysym_to_iced_key_and_loc, state::State};

#[derive(Clone, Copy, Debug)]
pub struct KeyboardKey {
    pub key: Keysym,
    pub modifiers: Modifiers,
    pub pressed: bool,
}

impl State {
    pub(crate) fn on_key_repeat(&mut self, keyboard: &WlKeyboard, event: KeyEvent) {
        self.on_key_press(keyboard, event, true);
    }

    pub(crate) fn on_key_press(&mut self, _keyboard: &WlKeyboard, event: KeyEvent, repeat: bool) {
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
                text: event.utf8.map(Into::into),
                modified_key: key, // TODO:
                physical_key: Physical::Unidentified(NativeCode::Xkb(event.keysym.raw())),
                repeat,
            }));
    }
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
        keyboard: &WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        self.on_key_press(keyboard, event, false)
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
        _raw_modifiers: RawModifiers,
        _layout: u32,
    ) {
        self.keyboard_modifiers = modifiers;

        // TODO: Should we check if the modifiers actually changed ?
        let Some(KeyboardFocus::Layer(layer)) = self.keyboard_focus.as_ref() else {
            return;
        };

        let Some(snowcap_layer) = self.layers.iter_mut().find(|sn_l| &sn_l.layer == layer) else {
            return;
        };

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
            .queue_event(iced::Event::Keyboard(
                iced::keyboard::Event::ModifiersChanged(modifiers),
            ));
    }

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
        // TODO: Smithay does not support wl_keyboard v10. Until that happen, this will not be
        // called.
        // I'm leaving this commented for now because I don't know whether only one or both
        // function will get called when support is added.
        //
        // self.on_key_repeat(keyboard, event, true)
    }
}
delegate_keyboard!(State);

pub enum KeyboardFocus {
    Layer(LayerSurface),
}
