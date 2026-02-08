use iced::keyboard::key::{NativeCode, Physical};
use smithay_client_toolkit::{
    delegate_keyboard,
    reexports::client::{
        Connection, QueueHandle,
        protocol::{wl_keyboard::WlKeyboard, wl_surface::WlSurface},
    },
    seat::keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
    shell::{WaylandSurface, wlr_layer::LayerSurface, xdg::popup::Popup},
};

use crate::{input::keyboard::keysym_to_iced_key_and_loc, state::State};

#[derive(Clone, Debug)]
pub struct KeyboardKey {
    pub key: Keysym,
    pub modifiers: Modifiers,
    pub pressed: bool,
    pub captured: bool,
    pub text: Option<String>,
}

impl State {
    pub(crate) fn on_key_repeat(&mut self, keyboard: &WlKeyboard, event: KeyEvent) {
        self.on_key_press(keyboard, event, true, None);
    }

    pub(crate) fn on_key_press(
        &mut self,
        _keyboard: &WlKeyboard,
        event: KeyEvent,
        repeat: bool,
        serial: Option<u32>,
    ) {
        let surface = match self.keyboard_focus.as_ref() {
            Some(KeyboardFocus::Layer(layer)) => self
                .layers
                .iter_mut()
                .find(|l| &l.layer == layer)
                .map(|l| &mut l.surface),
            Some(KeyboardFocus::Popup(popup)) => self
                .popups
                .iter_mut()
                .find(|l| &l.popup == popup)
                .map(|l| &mut l.surface),
            _ => None,
        };

        let Some(surface) = surface else {
            return;
        };

        if let Some(serial) = serial {
            surface.focus_serial = Some(serial);
        }

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

        surface
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
        serial: u32,
        _raw: &[u32],
        _keysyms: &[Keysym],
    ) {
        if let Some(layer) = self
            .layers
            .iter_mut()
            .find(|sn_layer| sn_layer.layer.wl_surface() == surface)
        {
            layer.surface.focus_serial = Some(serial);
            self.keyboard_focus = Some(KeyboardFocus::Layer(layer.layer.clone()));
        } else if let Some(popup) = self
            .popups
            .iter_mut()
            .find(|p| p.popup.wl_surface() == surface)
        {
            popup.surface.focus_serial = Some(serial);
            self.keyboard_focus = Some(KeyboardFocus::Popup(popup.popup.clone()))
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
        match self.keyboard_focus.as_ref() {
            Some(KeyboardFocus::Layer(layer)) if layer.wl_surface() == surface => {
                self.keyboard_focus = None;
            }
            Some(KeyboardFocus::Popup(popup)) if popup.wl_surface() == surface => {
                self.keyboard_focus = None
            }
            _ => (),
        };
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        keyboard: &WlKeyboard,
        serial: u32,
        event: KeyEvent,
    ) {
        self.on_key_press(keyboard, event, false, Some(serial))
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        serial: u32,
        event: KeyEvent,
    ) {
        let surface = match self.keyboard_focus.as_ref() {
            Some(KeyboardFocus::Layer(layer)) => self
                .layers
                .iter_mut()
                .find(|l| &l.layer == layer)
                .map(|l| &mut l.surface),
            Some(KeyboardFocus::Popup(popup)) => self
                .popups
                .iter_mut()
                .find(|l| &l.popup == popup)
                .map(|l| &mut l.surface),
            _ => None,
        };

        let Some(surface) = surface else {
            return;
        };

        surface.focus_serial = Some(serial);

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

        surface
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
        serial: u32,
        modifiers: Modifiers,
        _raw_modifiers: RawModifiers,
        _layout: u32,
    ) {
        self.keyboard_modifiers = modifiers;

        let surface = match self.keyboard_focus.as_ref() {
            Some(KeyboardFocus::Layer(layer)) => self
                .layers
                .iter_mut()
                .find(|l| &l.layer == layer)
                .map(|l| &mut l.surface),
            Some(KeyboardFocus::Popup(popup)) => self
                .popups
                .iter_mut()
                .find(|l| &l.popup == popup)
                .map(|l| &mut l.surface),
            _ => None,
        };

        let Some(surface) = surface else {
            return;
        };

        surface.focus_serial = Some(serial);

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

        surface.widgets.queue_event(iced::Event::Keyboard(
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
        // self.on_key_repeat(keyboard, event, true, Some(serial))
    }
}
delegate_keyboard!(State);

pub enum KeyboardFocus {
    Layer(LayerSurface),
    Popup(Popup),
}
