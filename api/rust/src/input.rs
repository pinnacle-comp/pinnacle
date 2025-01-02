// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Input management.
//!
//! This module provides [`Input`], a struct that gives you several different
//! methods for setting key- and mousebinds, changing xkeyboard settings, and more.
//! View the struct's documentation for more information.

use pinnacle_api_defs::pinnacle::input::{
    self,
    v1::{
        set_libinput_setting_request::{CalibrationMatrix, Setting},
        BindRequest, EnterBindLayerRequest, KeybindStreamRequest, MousebindStreamRequest,
        SetLibinputSettingRequest, SetRepeatRateRequest, SetXcursorRequest, SetXkbConfigRequest,
    },
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio_stream::StreamExt;

use crate::{client::Client, BlockOnTokio};

use self::libinput::LibinputSetting;

pub mod libinput;

pub use xkbcommon::xkb::Keysym;

/// A mouse button.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
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
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
    pub struct Mod: u16 {
        /// The shift key
        const SHIFT = 1;
        /// The ctrl key
        const CTRL = 1 << 1;
        /// The alt key
        const ALT = 1 << 2;
        /// The super key, aka meta, win, mod4
        const SUPER = 1 << 3;
        const ISO_LEVEL3_SHIFT = 1 << 4;
        const ISO_LEVEL5_SHIFT = 1 << 5;

        const IGNORE_SHIFT = 1 << 6;
        const IGNORE_CTRL = 1 << 7;
        const IGNORE_ALT = 1 << 8;
        const IGNORE_SUPER = 1 << 9;
        const IGNORE_ISO_LEVEL3_SHIFT = 1 << 10;
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

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct BindLayer {
    name: Option<String>,
}

impl BindLayer {
    pub const DEFAULT: Self = Self { name: None };

    pub fn get(name: impl ToString) -> Self {
        Self {
            name: Some(name.to_string()),
        }
    }

    pub fn keybind(&self, mods: Mod, key: impl ToKeysym) -> Keybind {
        new_keybind(mods, key, self).block_on_tokio()
    }

    pub fn mousebind(&self, mods: Mod, button: MouseButton) -> Mousebind {
        new_mousebind(mods, button, self).block_on_tokio()
    }

    pub fn enter(&self) {
        Client::input()
            .enter_bind_layer(EnterBindLayerRequest {
                layer_name: self.name.clone(),
            })
            .block_on_tokio()
            .unwrap();
    }
}

enum Edge {
    Press,
    Release,
}

pub struct Keybind {
    bind_id: u32,
    callback_sender: Option<UnboundedSender<(Box<dyn FnMut() + Send + 'static>, Edge)>>,
}

pub fn keybind(mods: Mod, key: impl ToKeysym) -> Keybind {
    BindLayer::DEFAULT.keybind(mods, key)
}

impl Keybind {
    pub fn on_press<F: FnMut() + Send + 'static>(&mut self, on_press: F) -> &mut Self {
        let sender = self
            .callback_sender
            .get_or_insert_with(|| new_keybind_stream(self.bind_id).block_on_tokio());
        let _ = sender.send((Box::new(on_press), Edge::Press));

        self
    }

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
                group: None,       // TODO:
                description: None, // TODO:
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

pub struct Mousebind {
    bind_id: u32,
    callback_sender: Option<UnboundedSender<(Box<dyn FnMut() + Send + 'static>, Edge)>>,
}

pub fn mousebind(mods: Mod, button: MouseButton) -> Mousebind {
    BindLayer::DEFAULT.mousebind(mods, button)
}

impl Mousebind {
    pub fn on_press<F: FnMut() + Send + 'static>(&mut self, on_press: F) -> &mut Self {
        let sender = self
            .callback_sender
            .get_or_insert_with(|| new_mousebind_stream(self.bind_id).block_on_tokio());
        let _ = sender.send((Box::new(on_press), Edge::Press));

        self
    }

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
                group: None,       // TODO:
                description: None, // TODO:
                bind: Some(input::v1::bind::Bind::Mouse(input::v1::Mousebind {
                    button: button as u32,
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
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Default)]
pub struct XkbConfig {
    /// Files of rules to be used for keyboard mapping composition
    pub rules: Option<&'static str>,
    /// Name of the model of your keyboard type
    pub model: Option<&'static str>,
    /// Layout(s) you intend to use
    pub layout: Option<&'static str>,
    /// Variant(s) of the layout you intend to use
    pub variant: Option<&'static str>,
    /// Extra xkb configuration options
    pub options: Option<&'static str>,
}

/// Set the xkeyboard config.
///
/// This allows you to set several xkeyboard options like `layout` and `rules`.
///
/// See `xkeyboard-config(7)` for more information.
///
/// # Examples
///
/// ```
/// use pinnacle_api::input::XkbConfig;
///
/// input.set_xkb_config(XkbConfig {
///     layout: Some("us,fr,ge"),
///     options: Some("ctrl:swapcaps,caps:shift"),
///     ..Default::default()
/// });
/// ```
pub fn set_xkb_config(xkb_config: XkbConfig) {
    Client::input()
        .set_xkb_config(SetXkbConfigRequest {
            rules: xkb_config.rules.map(String::from),
            variant: xkb_config.variant.map(String::from),
            layout: xkb_config.layout.map(String::from),
            model: xkb_config.model.map(String::from),
            options: xkb_config.options.map(String::from),
        })
        .block_on_tokio()
        .unwrap();
}

/// Keybind information.
///
/// Mainly used for the keybind list.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct KeybindInfo {
    /// The group to place this keybind in.
    pub group: Option<String>,
    /// The description of this keybind.
    pub description: Option<String>,
}

/// The description of a keybind.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeybindDescription {
    /// The keybind's modifiers.
    pub modifiers: Vec<Mod>,
    /// The keysym code.
    pub key_code: u32,
    /// The name of the key.
    pub xkb_name: String,
    /// The group.
    pub group: Option<String>,
    /// The description of the keybind.
    pub description: Option<String>,
}

/// Set a mousebind.
///
/// If called with an already set mousebind, it gets replaced.
///
/// You must supply:
/// - `mods`: A list of [`Mod`]s. These must be held down for the keybind to trigger.
/// - `button`: A [`MouseButton`].
/// - `edge`: A [`MouseEdge`]. This allows you to trigger the bind on either mouse press or release.
/// - `action`: A closure that will be run when the mousebind is triggered.
///     - Currently, any captures must be both `Send` and `'static`. If you want to mutate
///       something, consider using channels or [`Box::leak`].
///
/// # Examples
///
/// ```
/// use pinnacle_api::input::{Mod, MouseButton, MouseEdge};
///
/// // Set `Super + left click` to start moving a window
/// input.mousebind([Mod::Super], MouseButton::Left, MouseEdge::Press, || {
///     window.begin_move(MouseButton::Press);
/// });
/// ```
// pub fn mousebind(
//     &self,
//     mods: impl IntoIterator<Item = Mod>,
//     button: MouseButton,
//     edge: MouseEdge,
//     mut action: impl FnMut() + 'static + Send,
// ) {
//     let modifiers = mods.into_iter().map(|modif| modif as i32).collect();
//     let mut stream = match block_on_tokio(crate::input().set_mousebind(SetMousebindRequest {
//         modifiers,
//         button: Some(button as u32),
//         edge: Some(edge as i32),
//     })) {
//         Ok(stream) => stream.into_inner(),
//         Err(err) => {
//             error!("Failed to set keybind: {err}");
//             return;
//         }
//     };
//
//     tokio::spawn(async move {
//         while let Some(Ok(_response)) = stream.next().await {
//             action();
//             tokio::task::yield_now().await;
//         }
//     });
// }

/// Get all keybinds and their information.
// pub fn keybind_descriptions(&self) -> impl Iterator<Item = KeybindDescription> {
//     let descriptions = match block_on_tokio(
//         crate::input().keybind_descriptions(KeybindDescriptionsRequest {}),
//     ) {
//         Ok(descs) => descs.into_inner().descriptions,
//         Err(err) => {
//             error!("Failed to get keybind descriptions: {err}");
//             Vec::new()
//         }
//     };
//
//     descriptions.into_iter().map(|desc| {
//         let mods = desc.modifiers().flat_map(|m| match m {
//             input::v0alpha1::Modifier::Unspecified => None,
//             input::v0alpha1::Modifier::Shift => Some(Mod::Shift),
//             input::v0alpha1::Modifier::Ctrl => Some(Mod::Ctrl),
//             input::v0alpha1::Modifier::Alt => Some(Mod::Alt),
//             input::v0alpha1::Modifier::Super => Some(Mod::Super),
//         });
//         KeybindDescription {
//             modifiers: mods.collect(),
//             key_code: desc.raw_code(),
//             xkb_name: desc.xkb_name().to_string(),
//             group: desc.group,
//             description: desc.description,
//         }
//     })
// }

/// Set the keyboard's repeat rate.
///
/// This allows you to set the time between holding down a key and it repeating
/// as well as the time between each repeat.
///
/// Units are in milliseconds.
///
/// # Examples
///
/// ```
/// // Set keyboard to repeat after holding down for half a second,
/// // and repeat once every 25ms (40 times a second)
/// input.set_repeat_rate(25, 500);
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

/// Set a libinput setting.
///
/// From [freedesktop.org](https://www.freedesktop.org/wiki/Software/libinput/):
/// > libinput is a library to handle input devices in Wayland compositors
///
/// As such, this method allows you to set various settings related to input devices.
/// This includes things like pointer acceleration and natural scrolling.
///
/// See [`LibinputSetting`] for all the settings you can change.
///
/// Note: currently Pinnacle applies anything set here to *every* device, regardless of what it
/// actually is. This will be fixed in the future.
///
/// # Examples
///
/// ```
/// use pinnacle_api::input::libinput::*;
///
/// // Set pointer acceleration to flat
/// input.set_libinput_setting(LibinputSetting::AccelProfile(AccelProfile::Flat));
///
/// // Enable natural scrolling (reverses scroll direction; usually used with trackpads)
/// input.set_libinput_setting(LibinputSetting::NaturalScroll(true));
/// ```
pub fn set_libinput_setting(setting: LibinputSetting) {
    let setting = match setting {
        LibinputSetting::AccelProfile(profile) => Setting::AccelProfile(profile as i32),
        LibinputSetting::AccelSpeed(speed) => Setting::AccelSpeed(speed),
        LibinputSetting::CalibrationMatrix(matrix) => {
            Setting::CalibrationMatrix(CalibrationMatrix {
                matrix: matrix.to_vec(),
            })
        }
        LibinputSetting::ClickMethod(method) => Setting::ClickMethod(method as i32),
        LibinputSetting::DisableWhileTyping(disable) => Setting::DisableWhileTyping(disable),
        LibinputSetting::LeftHanded(enable) => Setting::LeftHanded(enable),
        LibinputSetting::MiddleEmulation(enable) => Setting::MiddleEmulation(enable),
        LibinputSetting::RotationAngle(angle) => Setting::RotationAngle(angle),
        LibinputSetting::ScrollButton(button) => Setting::RotationAngle(button),
        LibinputSetting::ScrollButtonLock(enable) => Setting::ScrollButtonLock(enable),
        LibinputSetting::ScrollMethod(method) => Setting::ScrollMethod(method as i32),
        LibinputSetting::NaturalScroll(enable) => Setting::NaturalScroll(enable),
        LibinputSetting::TapButtonMap(map) => Setting::TapButtonMap(map as i32),
        LibinputSetting::TapDrag(enable) => Setting::TapDrag(enable),
        LibinputSetting::TapDragLock(enable) => Setting::TapDragLock(enable),
        LibinputSetting::Tap(enable) => Setting::Tap(enable),
    };

    Client::input()
        .set_libinput_setting(SetLibinputSettingRequest {
            setting: Some(setting),
        })
        .block_on_tokio()
        .unwrap();
}

/// Set the xcursor theme.
///
/// Pinnacle reads `$XCURSOR_THEME` on startup to determine the theme.
/// This allows you to set it at runtime.
///
/// # Examples
///
/// ```
/// input.set_xcursor_theme("Adwaita");
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

/// Set the xcursor size.
///
/// Pinnacle reads `$XCURSOR_SIZE` on startup to determine the cursor size.
/// This allows you to set it at runtime.
///
/// # Examples
///
/// ```
/// input.set_xcursor_size(64);
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
    /// Convert this into a [`Keysym`].
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
