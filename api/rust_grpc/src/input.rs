use futures::executor::block_on;
use num_enum::TryFromPrimitive;
use pinnacle_api_defs::pinnacle::input::{
    self,
    v0alpha1::{
        input_service_client::InputServiceClient,
        set_libinput_setting_request::{CalibrationMatrix, Setting},
        SetKeybindRequest, SetLibinputSettingRequest, SetMousebindRequest, SetRepeatRateRequest,
    },
};
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use xkbcommon::xkb::Keysym;

pub use pinnacle_api_defs::pinnacle::input::v0alpha1::SetXkbConfigRequest as XkbConfig;

use crate::FutSender;

use self::libinput::LibinputSetting;
pub mod libinput;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum MouseButton {
    Left = 0x110,
    Right = 0x111,
    Middle = 0x112,
    Side = 0x113,
    Extra = 0x114,
    Forward = 0x115,
    Back = 0x116,
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, TryFromPrimitive)]
pub enum Mod {
    Shift = 1,
    Ctrl,
    Alt,
    Super,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, TryFromPrimitive)]
pub enum MouseEdge {
    Press = 1,
    Release,
}

#[derive(Debug, Clone)]
pub struct Input {
    client: InputServiceClient<Channel>,
    fut_sender: FutSender,
}

impl Input {
    pub fn new(client: InputServiceClient<Channel>, fut_sender: FutSender) -> Self {
        Self { client, fut_sender }
    }

    pub fn keybind(
        &self,
        mods: impl IntoIterator<Item = Mod>,
        key: impl Key + Send + 'static,
        mut action: impl FnMut() + Send + 'static,
    ) {
        let mut client = self.client.clone();

        let modifiers = mods.into_iter().map(|modif| modif as i32).collect();

        self.fut_sender
            .send(Box::pin(async move {
                let mut stream = client
                    .set_keybind(SetKeybindRequest {
                        modifiers,
                        key: Some(input::v0alpha1::set_keybind_request::Key::RawCode(
                            key.into_keysym().raw(),
                        )),
                    })
                    .await
                    .unwrap()
                    .into_inner();

                while let Some(Ok(_response)) = stream.next().await {
                    action();
                }
            }))
            .unwrap();
    }

    pub fn mousebind(
        &self,
        mods: impl IntoIterator<Item = Mod>,
        button: MouseButton,
        edge: MouseEdge,
        mut action: impl FnMut() + 'static + Send,
    ) {
        let mut client = self.client.clone();

        let modifiers = mods.into_iter().map(|modif| modif as i32).collect();

        tokio::spawn(async move {
            let mut stream = client
                .set_mousebind(SetMousebindRequest {
                    modifiers,
                    button: Some(button as u32),
                    edge: Some(edge as i32),
                })
                .await
                .unwrap()
                .into_inner();

            while let Some(Ok(_response)) = stream.next().await {
                action();
            }
        });
    }

    pub fn set_xkb_config(&self, xkb_config: XkbConfig) {
        let mut client = self.client.clone();

        block_on(client.set_xkb_config(xkb_config)).unwrap();
    }

    pub fn set_repeat_rate(&self, rate: i32, delay: i32) {
        let mut client = self.client.clone();

        block_on(client.set_repeat_rate(SetRepeatRateRequest {
            rate: Some(rate),
            delay: Some(delay),
        }))
        .unwrap();
    }

    pub fn set_libinput_setting(&self, setting: LibinputSetting) {
        let mut client = self.client.clone();

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

        block_on(client.set_libinput_setting(SetLibinputSettingRequest {
            setting: Some(setting),
        }))
        .unwrap();
    }
}

pub trait Key {
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
