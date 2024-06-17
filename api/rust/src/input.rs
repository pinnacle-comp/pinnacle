// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Input management.
//!
//! This module provides [`Input`], a struct that gives you several different
//! methods for setting key- and mousebinds, changing xkeyboard settings, and more.
//! View the struct's documentation for more information.

use futures::{future::BoxFuture, FutureExt, StreamExt};
use num_enum::TryFromPrimitive;
use pinnacle_api_defs::pinnacle::input::{
    self,
    v0alpha1::{
        input_service_client::InputServiceClient,
        set_libinput_setting_request::{CalibrationMatrix, Setting},
        KeybindDescriptionsRequest, SetKeybindRequest, SetLibinputSettingRequest,
        SetMousebindRequest, SetRepeatRateRequest, SetXkbConfigRequest,
    },
};
use tokio::sync::mpsc::UnboundedSender;
use tonic::transport::Channel;
use xkbcommon::xkb::Keysym;

use crate::block_on_tokio;

use self::libinput::LibinputSetting;

pub mod libinput;

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

/// Keyboard modifiers.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, TryFromPrimitive)]
pub enum Mod {
    /// The shift key
    Shift = 1,
    /// The ctrl key
    Ctrl,
    /// The alt key
    Alt,
    /// The super key, aka meta, win, mod4
    Super,
}

/// Press or release.
#[repr(i32)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, TryFromPrimitive)]
pub enum MouseEdge {
    /// Perform actions on button press
    Press = 1,
    /// Perform actions on button release
    Release,
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

/// The `Input` struct.
///
/// This struct contains methods that allow you to set key- and mousebinds,
/// change xkeyboard and libinput settings, and change the keyboard's repeat rate.
#[derive(Debug, Clone)]
pub struct Input {
    channel: Channel,
    fut_sender: UnboundedSender<BoxFuture<'static, ()>>,
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

impl Input {
    pub(crate) fn new(
        channel: Channel,
        fut_sender: UnboundedSender<BoxFuture<'static, ()>>,
    ) -> Self {
        Self {
            channel,
            fut_sender,
        }
    }

    fn create_input_client(&self) -> InputServiceClient<Channel> {
        InputServiceClient::new(self.channel.clone())
    }

    /// Set a keybind.
    ///
    /// If called with an already set keybind, it gets replaced.
    ///
    /// You must supply:
    /// - `mods`: A list of [`Mod`]s. These must be held down for the keybind to trigger.
    /// - `key`: The key that needs to be pressed. This can be anything that implements the [Key] trait:
    ///     - `char`
    ///     - `&str` and `String`: This is any name from
    ///       [xkbcommon-keysyms.h](https://xkbcommon.org/doc/current/xkbcommon-keysyms_8h.html)
    ///       without the `XKB_KEY_` prefix.
    ///     - `u32`: The numerical key code from the website above.
    ///     - A [`keysym`][Keysym] from the [`xkbcommon`] re-export.
    /// - `action`: A closure that will be run when the keybind is triggered.
    ///     - Currently, any captures must be both `Send` and `'static`. If you want to mutate
    ///       something, consider using channels or [`Box::leak`].
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::input::Mod;
    ///
    /// // Set `Super + Shift + c` to close the focused window
    /// input.keybind([Mod::Super, Mod::Shift], 'c', || {
    ///     if let Some(win) = window.get_focused() {
    ///         win.close();
    ///     }
    /// });
    ///
    /// // With a string key
    /// input.keybind([], "BackSpace", || { /* ... */ });
    ///
    /// // With a numeric key
    /// input.keybind([], 65, || { /* ... */ });    // 65 = 'A'
    ///
    /// // With a `Keysym`
    /// input.keybind([], pinnacle_api::xkbcommon::xkb::Keysym::Return, || { /* ... */ });
    /// ```
    pub fn keybind(
        &self,
        mods: impl IntoIterator<Item = Mod>,
        key: impl Key + Send + 'static,
        mut action: impl FnMut() + Send + 'static,
        keybind_info: impl Into<Option<KeybindInfo>>,
    ) {
        let mut client = self.create_input_client();

        let modifiers = mods.into_iter().map(|modif| modif as i32).collect();

        let keybind_info: Option<KeybindInfo> = keybind_info.into();

        let mut stream = block_on_tokio(client.set_keybind(SetKeybindRequest {
            modifiers,
            key: Some(input::v0alpha1::set_keybind_request::Key::RawCode(
                key.into_keysym().raw(),
            )),
            group: keybind_info.clone().and_then(|info| info.group),
            description: keybind_info.clone().and_then(|info| info.description),
        }))
        .unwrap()
        .into_inner();

        self.fut_sender
            .send(
                async move {
                    while let Some(Ok(_response)) = stream.next().await {
                        action();
                        tokio::task::yield_now().await;
                    }
                }
                .boxed(),
            )
            .unwrap();
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
    pub fn mousebind(
        &self,
        mods: impl IntoIterator<Item = Mod>,
        button: MouseButton,
        edge: MouseEdge,
        mut action: impl FnMut() + 'static + Send,
    ) {
        let mut client = self.create_input_client();

        let modifiers = mods.into_iter().map(|modif| modif as i32).collect();
        let mut stream = block_on_tokio(client.set_mousebind(SetMousebindRequest {
            modifiers,
            button: Some(button as u32),
            edge: Some(edge as i32),
        }))
        .unwrap()
        .into_inner();

        self.fut_sender
            .send(
                async move {
                    while let Some(Ok(_response)) = stream.next().await {
                        action();
                        tokio::task::yield_now().await;
                    }
                }
                .boxed(),
            )
            .unwrap();
    }

    /// Get all keybinds and their information.
    pub fn keybind_descriptions(&self) -> impl Iterator<Item = KeybindDescription> {
        let mut client = self.create_input_client();
        let descriptions =
            block_on_tokio(client.keybind_descriptions(KeybindDescriptionsRequest {})).unwrap();
        let descriptions = descriptions.into_inner();

        descriptions.descriptions.into_iter().map(|desc| {
            let mods = desc.modifiers().flat_map(|m| match m {
                input::v0alpha1::Modifier::Unspecified => None,
                input::v0alpha1::Modifier::Shift => Some(Mod::Shift),
                input::v0alpha1::Modifier::Ctrl => Some(Mod::Ctrl),
                input::v0alpha1::Modifier::Alt => Some(Mod::Alt),
                input::v0alpha1::Modifier::Super => Some(Mod::Super),
            });
            KeybindDescription {
                modifiers: mods.collect(),
                key_code: desc.raw_code(),
                xkb_name: desc.xkb_name().to_string(),
                group: desc.group,
                description: desc.description,
            }
        })
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
    pub fn set_xkb_config(&self, xkb_config: XkbConfig) {
        let mut client = self.create_input_client();

        block_on_tokio(client.set_xkb_config(SetXkbConfigRequest {
            rules: xkb_config.rules.map(String::from),
            variant: xkb_config.variant.map(String::from),
            layout: xkb_config.layout.map(String::from),
            model: xkb_config.model.map(String::from),
            options: xkb_config.options.map(String::from),
        }))
        .unwrap();
    }

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
    pub fn set_repeat_rate(&self, rate: i32, delay: i32) {
        let mut client = self.create_input_client();

        block_on_tokio(client.set_repeat_rate(SetRepeatRateRequest {
            rate: Some(rate),
            delay: Some(delay),
        }))
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
    pub fn set_libinput_setting(&self, setting: LibinputSetting) {
        let mut client = self.create_input_client();

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

        block_on_tokio(client.set_libinput_setting(SetLibinputSettingRequest {
            setting: Some(setting),
        }))
        .unwrap();
    }
}

/// A trait that designates anything that can be converted into a [`Keysym`].
pub trait Key {
    /// Convert this into a [`Keysym`].
    fn into_keysym(self) -> Keysym;
}

impl Key for Keysym {
    fn into_keysym(self) -> Keysym {
        self
    }
}

impl Key for char {
    fn into_keysym(self) -> Keysym {
        Keysym::from_char(self)
    }
}

impl Key for &str {
    fn into_keysym(self) -> Keysym {
        xkbcommon::xkb::keysym_from_name(self, xkbcommon::xkb::KEYSYM_NO_FLAGS)
    }
}

impl Key for String {
    fn into_keysym(self) -> Keysym {
        xkbcommon::xkb::keysym_from_name(&self, xkbcommon::xkb::KEYSYM_NO_FLAGS)
    }
}

impl Key for u32 {
    fn into_keysym(self) -> Keysym {
        Keysym::from(self)
    }
}
