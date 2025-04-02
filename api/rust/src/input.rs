// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Input management.
//!
//! This module provides ways to manage bindings, input devices, and other input settings.

use num_enum::{FromPrimitive, IntoPrimitive};
use pinnacle_api_defs::pinnacle::input::{
    self,
    v1::{
        switch_xkb_layout_request, BindProperties, BindRequest, EnterBindLayerRequest,
        GetBindInfosRequest, KeybindOnPressRequest, KeybindStreamRequest, MousebindOnPressRequest,
        MousebindStreamRequest, SetBindPropertiesRequest, SetRepeatRateRequest, SetXcursorRequest,
        SetXkbConfigRequest, SetXkbKeymapRequest, SwitchXkbLayoutRequest,
    },
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio_stream::StreamExt;

use crate::{
    client::Client,
    signal::{InputSignal, SignalHandle},
    BlockOnTokio,
};

pub mod libinput;

pub use xkbcommon::xkb::Keysym;

/// A mouse button.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, FromPrimitive, IntoPrimitive)]
#[repr(u32)]
pub enum MouseButton {
    /// The left mouse button
    Left = 0x110,
    /// The right mouse button
    Right = 0x111,
    /// The middle mouse button
    Middle = 0x112,
    /// The side mouse button
    Side = 0x113,
    /// The extra mouse button
    Extra = 0x114,
    /// The forward mouse button
    Forward = 0x115,
    /// The backward mouse button
    Back = 0x116,
    /// Some other mouse button
    #[num_enum(catch_all)]
    Other(u32),
}

bitflags::bitflags! {
    /// A keyboard modifier for use in binds.
    ///
    /// Binds can be configured to require certain keyboard modifiers to be held down to trigger.
    /// For example, a bind with `Mod::SUPER | Mod::CTRL` requires both the super and control keys
    /// to be held down.
    ///
    /// Normally, modifiers must be in the exact same state as passed in to trigger a bind.
    /// This means if you use `Mod::SUPER` in a bind, *only* super must be held down; holding
    /// down any other modifier will invalidate the bind.
    ///
    /// To circumvent this, you can ignore certain modifiers by OR-ing with the respective
    /// `Mod::IGNORE_*`.
    #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Default)]
    pub struct Mod: u16 {
        /// The shift key
        const SHIFT = 1;
        /// The ctrl key
        const CTRL = 1 << 1;
        /// The alt key
        const ALT = 1 << 2;
        /// The super key, aka meta, win, mod4
        const SUPER = 1 << 3;
        /// The IsoLevel3Shift modifier
        const ISO_LEVEL3_SHIFT = 1 << 4;
        /// The IsoLevel5Shift modifer
        const ISO_LEVEL5_SHIFT = 1 << 5;

        /// Ignore the shift key
        const IGNORE_SHIFT = 1 << 6;
        /// Ignore the ctrl key
        const IGNORE_CTRL = 1 << 7;
        /// Ignore the alt key
        const IGNORE_ALT = 1 << 8;
        /// Ignore the super key
        const IGNORE_SUPER = 1 << 9;
        /// Ignore the IsoLevel3Shift modifier
        const IGNORE_ISO_LEVEL3_SHIFT = 1 << 10;
        /// Ignore the IsoLevel5Shift modifier
        const IGNORE_ISO_LEVEL5_SHIFT = 1 << 11;
    }
}

impl Mod {
    fn api_mods(&self) -> Vec<input::v1::Modifier> {
        let mut mods = Vec::new();
        if self.contains(Mod::SHIFT) {
            mods.push(input::v1::Modifier::Shift);
        }
        if self.contains(Mod::CTRL) {
            mods.push(input::v1::Modifier::Ctrl);
        }
        if self.contains(Mod::ALT) {
            mods.push(input::v1::Modifier::Alt);
        }
        if self.contains(Mod::SUPER) {
            mods.push(input::v1::Modifier::Super);
        }
        if self.contains(Mod::ISO_LEVEL3_SHIFT) {
            mods.push(input::v1::Modifier::IsoLevel3Shift);
        }
        if self.contains(Mod::ISO_LEVEL5_SHIFT) {
            mods.push(input::v1::Modifier::IsoLevel5Shift);
        }
        mods
    }

    fn api_ignore_mods(&self) -> Vec<input::v1::Modifier> {
        let mut mods = Vec::new();
        if self.contains(Mod::IGNORE_SHIFT) {
            mods.push(input::v1::Modifier::Shift);
        }
        if self.contains(Mod::IGNORE_CTRL) {
            mods.push(input::v1::Modifier::Ctrl);
        }
        if self.contains(Mod::IGNORE_ALT) {
            mods.push(input::v1::Modifier::Alt);
        }
        if self.contains(Mod::IGNORE_SUPER) {
            mods.push(input::v1::Modifier::Super);
        }
        if self.contains(Mod::IGNORE_ISO_LEVEL3_SHIFT) {
            mods.push(input::v1::Modifier::IsoLevel3Shift);
        }
        if self.contains(Mod::IGNORE_ISO_LEVEL5_SHIFT) {
            mods.push(input::v1::Modifier::IsoLevel5Shift);
        }
        mods
    }
}

/// A bind layer, also known as a bind mode.
///
/// Normally all binds belong to the [`DEFAULT`][Self::DEFAULT] mode.
/// You can bind binding to different layers and switch between them to enable modal binds.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct BindLayer {
    name: Option<String>,
}

impl BindLayer {
    /// The default bind layer.
    ///
    /// This is the layer [`input::keybind`][self::keybind] uses.
    pub const DEFAULT: Self = Self { name: None };

    /// Gets the bind layer with the given `name`.
    pub fn get(name: impl ToString) -> Self {
        Self {
            name: Some(name.to_string()),
        }
    }

    /// Creates a keybind on this layer.
    pub fn keybind(&self, mods: Mod, key: impl ToKeysym) -> Keybind {
        new_keybind(mods, key, self).block_on_tokio()
    }

    /// Creates a mousebind on this layer.
    pub fn mousebind(&self, mods: Mod, button: MouseButton) -> Mousebind {
        new_mousebind(mods, button, self).block_on_tokio()
    }

    /// Enters this layer, causing only its binds to be in effect.
    pub fn enter(&self) {
        Client::input()
            .enter_bind_layer(EnterBindLayerRequest {
                layer_name: self.name.clone(),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Returns this bind layer's name, or `None` if this is the default bind layer.
    pub fn name(&self) -> Option<String> {
        self.name.clone()
    }
}

/// Functionality common to all bind types.
pub trait Bind {
    /// Sets this bind's group.
    fn group(&mut self, group: impl ToString) -> &mut Self;
    /// Sets this bind's description.
    fn description(&mut self, desc: impl ToString) -> &mut Self;
    /// Sets this bind as a quit bind.
    fn set_as_quit(&mut self) -> &mut Self;
    /// Sets this bind as a reload config bind.
    fn set_as_reload_config(&mut self) -> &mut Self;
    /// Allows this bind to trigger when the session is locked.
    fn allow_when_locked(&mut self) -> &mut Self;
}

macro_rules! bind_impl {
    ($ty:ty) => {
        impl Bind for $ty {
            fn group(&mut self, group: impl ToString) -> &mut Self {
                Client::input()
                    .set_bind_properties(SetBindPropertiesRequest {
                        bind_id: self.bind_id,
                        properties: Some(BindProperties {
                            group: Some(group.to_string()),
                            ..Default::default()
                        }),
                    })
                    .block_on_tokio()
                    .unwrap();
                self
            }

            fn description(&mut self, desc: impl ToString) -> &mut Self {
                Client::input()
                    .set_bind_properties(SetBindPropertiesRequest {
                        bind_id: self.bind_id,
                        properties: Some(BindProperties {
                            description: Some(desc.to_string()),
                            ..Default::default()
                        }),
                    })
                    .block_on_tokio()
                    .unwrap();
                self
            }

            fn set_as_quit(&mut self) -> &mut Self {
                Client::input()
                    .set_bind_properties(SetBindPropertiesRequest {
                        bind_id: self.bind_id,
                        properties: Some(BindProperties {
                            quit: Some(true),
                            ..Default::default()
                        }),
                    })
                    .block_on_tokio()
                    .unwrap();
                self
            }

            fn set_as_reload_config(&mut self) -> &mut Self {
                Client::input()
                    .set_bind_properties(SetBindPropertiesRequest {
                        bind_id: self.bind_id,
                        properties: Some(BindProperties {
                            reload_config: Some(true),
                            ..Default::default()
                        }),
                    })
                    .block_on_tokio()
                    .unwrap();
                self
            }

            fn allow_when_locked(&mut self) -> &mut Self {
                Client::input()
                    .set_bind_properties(SetBindPropertiesRequest {
                        bind_id: self.bind_id,
                        properties: Some(BindProperties {
                            allow_when_locked: Some(true),
                            ..Default::default()
                        }),
                    })
                    .block_on_tokio()
                    .unwrap();
                self
            }
        }
    };
}

enum Edge {
    Press,
    Release,
}

type KeybindCallback = (Box<dyn FnMut() + Send + 'static>, Edge);

/// A keybind.
pub struct Keybind {
    bind_id: u32,
    callback_sender: Option<UnboundedSender<KeybindCallback>>,
}

bind_impl!(Keybind);

/// Creates a keybind on the [`DEFAULT`][BindLayer::DEFAULT] bind layer.
pub fn keybind(mods: Mod, key: impl ToKeysym) -> Keybind {
    BindLayer::DEFAULT.keybind(mods, key)
}

impl Keybind {
    /// Runs a closure whenever this keybind is pressed.
    pub fn on_press<F: FnMut() + Send + 'static>(&mut self, on_press: F) -> &mut Self {
        let sender = self
            .callback_sender
            .get_or_insert_with(|| new_keybind_stream(self.bind_id).block_on_tokio());
        let _ = sender.send((Box::new(on_press), Edge::Press));

        Client::input()
            .keybind_on_press(KeybindOnPressRequest {
                bind_id: self.bind_id,
            })
            .block_on_tokio()
            .unwrap();

        self
    }

    /// Runs a closure whenever this keybind is released.
    pub fn on_release<F: FnMut() + Send + 'static>(&mut self, on_release: F) -> &mut Self {
        let sender = self
            .callback_sender
            .get_or_insert_with(|| new_keybind_stream(self.bind_id).block_on_tokio());
        let _ = sender.send((Box::new(on_release), Edge::Release));

        self
    }
}

async fn new_keybind(mods: Mod, key: impl ToKeysym, layer: &BindLayer) -> Keybind {
    let ignore_mods = mods.api_ignore_mods();
    let mods = mods.api_mods();

    let bind_id = Client::input()
        .bind(BindRequest {
            bind: Some(input::v1::Bind {
                mods: mods.into_iter().map(|m| m.into()).collect(),
                ignore_mods: ignore_mods.into_iter().map(|m| m.into()).collect(),
                layer_name: layer.name.clone(),
                properties: Some(BindProperties::default()),
                bind: Some(input::v1::bind::Bind::Key(input::v1::Keybind {
                    key_code: Some(key.to_keysym().raw()),
                    xkb_name: None,
                })),
            }),
        })
        .await
        .unwrap()
        .into_inner()
        .bind_id;

    Keybind {
        bind_id,
        callback_sender: None,
    }
}

async fn new_keybind_stream(
    bind_id: u32,
) -> UnboundedSender<(Box<dyn FnMut() + Send + 'static>, Edge)> {
    let mut from_server = Client::input()
        .keybind_stream(KeybindStreamRequest { bind_id })
        .await
        .unwrap()
        .into_inner();

    let (send, mut recv) = unbounded_channel();

    tokio::spawn(async move {
        let mut on_presses = Vec::<Box<dyn FnMut() + Send + 'static>>::new();
        let mut on_releases = Vec::<Box<dyn FnMut() + Send + 'static>>::new();

        loop {
            tokio::select! {
                Some(Ok(response)) = from_server.next() => {
                    match response.edge() {
                        input::v1::Edge::Unspecified => (),
                        input::v1::Edge::Press => {
                            for on_press in on_presses.iter_mut() {
                                on_press();
                            }
                        }
                        input::v1::Edge::Release => {
                            for on_release in on_releases.iter_mut() {
                                on_release();
                            }
                        }
                    }
                }
                Some((cb, edge)) = recv.recv() => {
                    match edge {
                        Edge::Press => on_presses.push(cb),
                        Edge::Release => on_releases.push(cb),
                    }
                }
                else => break,
            }
        }
    });

    send
}

// Mousebinds

type MousebindCallback = (Box<dyn FnMut() + Send + 'static>, Edge);

/// A mousebind.
pub struct Mousebind {
    bind_id: u32,
    callback_sender: Option<UnboundedSender<MousebindCallback>>,
}

bind_impl!(Mousebind);

/// Creates a mousebind on the [`DEFAULT`][BindLayer::DEFAULT] bind layer.
pub fn mousebind(mods: Mod, button: MouseButton) -> Mousebind {
    BindLayer::DEFAULT.mousebind(mods, button)
}

impl Mousebind {
    /// Runs a closure whenever this mousebind is pressed.
    pub fn on_press<F: FnMut() + Send + 'static>(&mut self, on_press: F) -> &mut Self {
        let sender = self
            .callback_sender
            .get_or_insert_with(|| new_mousebind_stream(self.bind_id).block_on_tokio());
        let _ = sender.send((Box::new(on_press), Edge::Press));

        Client::input()
            .mousebind_on_press(MousebindOnPressRequest {
                bind_id: self.bind_id,
            })
            .block_on_tokio()
            .unwrap();

        self
    }

    /// Runs a closure whenever this mousebind is released.
    pub fn on_release<F: FnMut() + Send + 'static>(&mut self, on_release: F) -> &mut Self {
        let sender = self
            .callback_sender
            .get_or_insert_with(|| new_mousebind_stream(self.bind_id).block_on_tokio());
        let _ = sender.send((Box::new(on_release), Edge::Release));

        self
    }
}

async fn new_mousebind(mods: Mod, button: MouseButton, layer: &BindLayer) -> Mousebind {
    let ignore_mods = mods.api_ignore_mods();
    let mods = mods.api_mods();

    let bind_id = Client::input()
        .bind(BindRequest {
            bind: Some(input::v1::Bind {
                mods: mods.into_iter().map(|m| m.into()).collect(),
                ignore_mods: ignore_mods.into_iter().map(|m| m.into()).collect(),
                layer_name: layer.name.clone(),
                properties: Some(BindProperties::default()),
                bind: Some(input::v1::bind::Bind::Mouse(input::v1::Mousebind {
                    button: button.into(),
                })),
            }),
        })
        .await
        .unwrap()
        .into_inner()
        .bind_id;

    Mousebind {
        bind_id,
        callback_sender: None,
    }
}

async fn new_mousebind_stream(
    bind_id: u32,
) -> UnboundedSender<(Box<dyn FnMut() + Send + 'static>, Edge)> {
    let mut from_server = Client::input()
        .mousebind_stream(MousebindStreamRequest { bind_id })
        .await
        .unwrap()
        .into_inner();

    let (send, mut recv) = unbounded_channel();

    tokio::spawn(async move {
        let mut on_presses = Vec::<Box<dyn FnMut() + Send + 'static>>::new();
        let mut on_releases = Vec::<Box<dyn FnMut() + Send + 'static>>::new();

        loop {
            tokio::select! {
                Some(Ok(response)) = from_server.next() => {
                    match response.edge() {
                        input::v1::Edge::Unspecified => (),
                        input::v1::Edge::Press => {
                            for on_press in on_presses.iter_mut() {
                                on_press();
                            }
                        }
                        input::v1::Edge::Release => {
                            for on_release in on_releases.iter_mut() {
                                on_release();
                            }
                        }
                    }
                }
                Some((cb, edge)) = recv.recv() => {
                    match edge {
                        Edge::Press => on_presses.push(cb),
                        Edge::Release => on_releases.push(cb),
                    }
                }
                else => break,
            }
        }
    });

    send
}

/// A struct that lets you define xkeyboard config options.
///
/// See `xkeyboard-config(7)` for more information.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Default)]
pub struct XkbConfig {
    /// Files of rules to be used for keyboard mapping composition
    pub rules: Option<String>,
    /// Name of the model of your keyboard type
    pub model: Option<String>,
    /// Layout(s) you intend to use
    pub layout: Option<String>,
    /// Variant(s) of the layout you intend to use
    pub variant: Option<String>,
    /// Extra xkb configuration options
    pub options: Option<String>,
}

impl XkbConfig {
    /// Creates a new, empty [`XkbConfig`].
    pub fn new() -> Self {
        Default::default()
    }

    /// Sets this config's `rules`.
    pub fn with_rules(mut self, rules: impl ToString) -> Self {
        self.rules = Some(rules.to_string());
        self
    }

    /// Sets this config's `model`.
    pub fn with_model(mut self, model: impl ToString) -> Self {
        self.model = Some(model.to_string());
        self
    }

    /// Sets this config's `layout`.
    pub fn with_layout(mut self, layout: impl ToString) -> Self {
        self.layout = Some(layout.to_string());
        self
    }

    /// Sets this config's `variant`.
    pub fn with_variant(mut self, variant: impl ToString) -> Self {
        self.variant = Some(variant.to_string());
        self
    }

    /// Sets this config's `options`.
    pub fn with_options(mut self, options: impl ToString) -> Self {
        self.options = Some(options.to_string());
        self
    }
}

/// Sets the xkeyboard config.
///
/// This allows you to set several xkeyboard options like `layout` and `rules`.
///
/// See `xkeyboard-config(7)` for more information.
///
/// # Examples
///
/// ```no_run
/// # use pinnacle_api::input;
/// # use pinnacle_api::input::XkbConfig;
/// input::set_xkb_config(XkbConfig::new()
///     .with_layout("us,fr,ge")
///     .with_options("ctrl:swapcaps,caps:shift"));
/// ```
pub fn set_xkb_config(xkb_config: XkbConfig) {
    Client::input()
        .set_xkb_config(SetXkbConfigRequest {
            rules: xkb_config.rules,
            variant: xkb_config.variant,
            layout: xkb_config.layout,
            model: xkb_config.model,
            options: xkb_config.options,
        })
        .block_on_tokio()
        .unwrap();
}

/// Sets the XKB keymap.
///
/// # Examples
///
/// ```no_run
/// # use pinnacle_api::input;
/// input::set_xkb_keymap("keymap here...");
///
/// // From a file
/// # || {
/// input::set_xkb_keymap(std::fs::read_to_string("/path/to/keymap.xkb")?);
/// # Ok::<_, std::io::Error>(())
/// # };
/// ```
pub fn set_xkb_keymap(keymap: impl ToString) {
    Client::input()
        .set_xkb_keymap(SetXkbKeymapRequest {
            keymap: keymap.to_string(),
        })
        .block_on_tokio()
        .unwrap();
}

/// Cycles the current XKB layout forward.
pub fn cycle_xkb_layout_forward() {
    Client::input()
        .switch_xkb_layout(SwitchXkbLayoutRequest {
            action: Some(switch_xkb_layout_request::Action::Next(())),
        })
        .block_on_tokio()
        .unwrap();
}

/// Cycles the current XKB layout backward.
pub fn cycle_xkb_layout_backward() {
    Client::input()
        .switch_xkb_layout(SwitchXkbLayoutRequest {
            action: Some(switch_xkb_layout_request::Action::Prev(())),
        })
        .block_on_tokio()
        .unwrap();
}

/// Switches the current XKB layout to the one at the provided `index`.
///
/// Fails if the index is out of bounds.
pub fn switch_xkb_layout(index: u32) {
    Client::input()
        .switch_xkb_layout(SwitchXkbLayoutRequest {
            action: Some(switch_xkb_layout_request::Action::Index(index)),
        })
        .block_on_tokio()
        .unwrap();
}

/// Bind information.
///
/// Mainly used for the bind overlay.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BindInfo {
    /// The group to place this bind in. Empty if it is not in one.
    pub group: String,
    /// The description of this bind. Empty if it does not have one.
    pub description: String,
    /// The bind's modifiers.
    pub mods: Mod,
    /// The bind's layer.
    pub layer: BindLayer,
    /// Whether this bind is a quit bind.
    pub quit: bool,
    /// Whether this bind is a reload config bind.
    pub reload_config: bool,
    /// Whether this bind is allowed when the session is locked.
    pub allow_when_locked: bool,
    /// What kind of bind this is.
    pub kind: BindInfoKind,
}

/// The kind of a bind (hey that rhymes).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BindInfoKind {
    /// This is a keybind.
    Key {
        /// The numeric key code.
        key_code: u32,
        /// The xkeyboard name of this key.
        xkb_name: String,
    },
    /// This is a mousebind.
    Mouse {
        /// Which mouse button this bind uses.
        button: MouseButton,
    },
}

/// Sets the keyboard's repeat rate.
///
/// This allows you to set the time between holding down a key and it repeating
/// as well as the time between each repeat.
///
/// Units are in milliseconds.
///
/// # Examples
///
/// ```no_run
/// # use pinnacle_api::input;
/// // Set keyboard to repeat after holding down for half a second,
/// // and repeat once every 25ms (40 times a second)
/// input::set_repeat_rate(25, 500);
/// ```
pub fn set_repeat_rate(rate: i32, delay: i32) {
    Client::input()
        .set_repeat_rate(SetRepeatRateRequest {
            rate: Some(rate),
            delay: Some(delay),
        })
        .block_on_tokio()
        .unwrap();
}

/// Sets the xcursor theme.
///
/// Pinnacle reads `$XCURSOR_THEME` on startup to determine the theme.
/// This allows you to set it at runtime.
///
/// # Examples
///
/// ```no_run
/// # use pinnacle_api::input;
/// input::set_xcursor_theme("Adwaita");
/// ```
pub fn set_xcursor_theme(theme: impl ToString) {
    Client::input()
        .set_xcursor(SetXcursorRequest {
            theme: Some(theme.to_string()),
            size: None,
        })
        .block_on_tokio()
        .unwrap();
}

/// Sets the xcursor size.
///
/// Pinnacle reads `$XCURSOR_SIZE` on startup to determine the cursor size.
/// This allows you to set it at runtime.
///
/// # Examples
///
/// ```no_run
/// # use pinnacle_api::input;
/// input::set_xcursor_size(64);
/// ```
pub fn set_xcursor_size(size: u32) {
    Client::input()
        .set_xcursor(SetXcursorRequest {
            theme: None,
            size: Some(size),
        })
        .block_on_tokio()
        .unwrap();
}

/// A trait that designates anything that can be converted into a [`Keysym`].
pub trait ToKeysym {
    /// Converts this into a [`Keysym`].
    fn to_keysym(&self) -> Keysym;
}

impl ToKeysym for Keysym {
    fn to_keysym(&self) -> Keysym {
        *self
    }
}

impl ToKeysym for char {
    fn to_keysym(&self) -> Keysym {
        Keysym::from_char(*self)
    }
}

impl ToKeysym for &str {
    fn to_keysym(&self) -> Keysym {
        xkbcommon::xkb::keysym_from_name(self, xkbcommon::xkb::KEYSYM_NO_FLAGS)
    }
}

impl ToKeysym for String {
    fn to_keysym(&self) -> Keysym {
        xkbcommon::xkb::keysym_from_name(self, xkbcommon::xkb::KEYSYM_NO_FLAGS)
    }
}

impl ToKeysym for u32 {
    fn to_keysym(&self) -> Keysym {
        Keysym::from(*self)
    }
}

/// Gets all bind information.
pub fn bind_infos() -> impl Iterator<Item = BindInfo> {
    let infos = Client::input()
        .get_bind_infos(GetBindInfosRequest {})
        .block_on_tokio()
        .unwrap()
        .into_inner()
        .bind_infos;

    infos.into_iter().filter_map(|info| {
        let info = info.bind?;
        let mut mods = info.mods().fold(Mod::empty(), |acc, m| match m {
            input::v1::Modifier::Unspecified => acc,
            input::v1::Modifier::Shift => acc | Mod::SHIFT,
            input::v1::Modifier::Ctrl => acc | Mod::CTRL,
            input::v1::Modifier::Alt => acc | Mod::ALT,
            input::v1::Modifier::Super => acc | Mod::SUPER,
            input::v1::Modifier::IsoLevel3Shift => acc | Mod::ISO_LEVEL3_SHIFT,
            input::v1::Modifier::IsoLevel5Shift => acc | Mod::ISO_LEVEL5_SHIFT,
        });

        for ignore_mod in info.ignore_mods() {
            match ignore_mod {
                input::v1::Modifier::Unspecified => (),
                input::v1::Modifier::Shift => mods |= Mod::IGNORE_SHIFT,
                input::v1::Modifier::Ctrl => mods |= Mod::IGNORE_CTRL,
                input::v1::Modifier::Alt => mods |= Mod::IGNORE_ALT,
                input::v1::Modifier::Super => mods |= Mod::IGNORE_SUPER,
                input::v1::Modifier::IsoLevel3Shift => mods |= Mod::ISO_LEVEL3_SHIFT,
                input::v1::Modifier::IsoLevel5Shift => mods |= Mod::ISO_LEVEL5_SHIFT,
            }
        }

        let bind_kind = match info.bind? {
            input::v1::bind::Bind::Key(keybind) => BindInfoKind::Key {
                key_code: keybind.key_code(),
                xkb_name: keybind.xkb_name().to_string(),
            },
            input::v1::bind::Bind::Mouse(mousebind) => BindInfoKind::Mouse {
                button: MouseButton::from(mousebind.button),
            },
        };

        let layer = BindLayer {
            name: info.layer_name,
        };
        let group = info
            .properties
            .as_ref()
            .and_then(|props| props.group.clone())
            .unwrap_or_default();
        let description = info
            .properties
            .as_ref()
            .and_then(|props| props.description.clone())
            .unwrap_or_default();
        let quit = info
            .properties
            .as_ref()
            .and_then(|props| props.quit)
            .unwrap_or_default();
        let reload_config = info
            .properties
            .as_ref()
            .and_then(|props| props.reload_config)
            .unwrap_or_default();
        let allow_when_locked = info
            .properties
            .as_ref()
            .and_then(|props| props.allow_when_locked)
            .unwrap_or_default();

        Some(BindInfo {
            group,
            description,
            mods,
            layer,
            quit,
            reload_config,
            allow_when_locked,
            kind: bind_kind,
        })
    })
}

/// Connects to an [`InputSignal`].
///
/// # Examples
///
/// ```no_run
/// # use pinnacle_api::input;
/// # use pinnacle_api::signal::InputSignal;
/// input::connect_signal(InputSignal::DeviceAdded(Box::new(|device| {
///     println!("New device: {}", device.name());
/// })));
/// ```
pub fn connect_signal(signal: InputSignal) -> SignalHandle {
    let mut signal_state = Client::signal_state();

    match signal {
        InputSignal::DeviceAdded(f) => signal_state.input_device_added.add_callback(f),
    }
}
