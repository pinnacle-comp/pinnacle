use std::num::NonZeroU32;

use crate::{
    output::OutputName,
    tag::{Layout, TagId},
    window::{FloatingOrTiled, FullscreenOrMaximized, WindowId},
    Modifier, MouseEdge,
};

#[derive(Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize, Clone, Copy)]
pub struct CallbackId(pub u32);

#[derive(Debug, PartialEq, Copy, Clone, serde::Serialize)]
pub enum AccelProfile {
    Flat,
    Adaptive,
}

#[derive(Debug, PartialEq, Copy, Clone, serde::Serialize)]
pub enum ClickMethod {
    ButtonAreas,
    Clickfinger,
}

#[derive(Debug, PartialEq, Copy, Clone, serde::Serialize)]
pub enum ScrollMethod {
    NoScroll,
    TwoFinger,
    Edge,
    OnButtonDown,
}

#[derive(Debug, PartialEq, Copy, Clone, serde::Serialize)]
pub enum TapButtonMap {
    LeftRightMiddle,
    LeftMiddleRight,
}

#[derive(Debug, PartialEq, Copy, Clone, serde::Serialize)]
pub enum LibinputSetting {
    AccelProfile(AccelProfile),
    AccelSpeed(f64),
    CalibrationMatrix([f32; 6]),
    ClickMethod(ClickMethod),
    DisableWhileTypingEnabled(bool),
    LeftHanded(bool),
    MiddleEmulationEnabled(bool),
    RotationAngle(u32),
    ScrollMethod(ScrollMethod),
    NaturalScrollEnabled(bool),
    ScrollButton(u32),
    TapButtonMap(TapButtonMap),
    TapDragEnabled(bool),
    TapDragLockEnabled(bool),
    TapEnabled(bool),
}

#[derive(Debug, Hash, Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RequestId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WindowRuleCondition {
    /// This condition is met when any of the conditions provided is met.
    #[serde(default)]
    cond_any: Option<Vec<WindowRuleCondition>>,
    /// This condition is met when all of the conditions provided are met.
    #[serde(default)]
    cond_all: Option<Vec<WindowRuleCondition>>,
    /// This condition is met when the class matches.
    #[serde(default)]
    class: Option<Vec<String>>,
    /// This condition is met when the title matches.
    #[serde(default)]
    title: Option<Vec<String>>,
    /// This condition is met when the tag matches.
    #[serde(default)]
    tag: Option<Vec<TagId>>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct WindowRule {
    /// Set the output the window will open on.
    #[serde(default)]
    pub output: Option<OutputName>,
    /// Set the tags the output will have on open.
    #[serde(default)]
    pub tags: Option<Vec<TagId>>,
    /// Set the window to floating or tiled on open.
    #[serde(default)]
    pub floating_or_tiled: Option<FloatingOrTiled>,
    /// Set the window to fullscreen, maximized, or force it to neither.
    #[serde(default)]
    pub fullscreen_or_maximized: Option<FullscreenOrMaximized>,
    /// Set the window's initial size.
    #[serde(default)]
    pub size: Option<(NonZeroU32, NonZeroU32)>,
    /// Set the window's initial location. If the window is tiled, it will snap to this position
    /// when set to floating.
    #[serde(default)]
    pub location: Option<(i32, i32)>,
}

#[derive(Debug, serde::Serialize)]
pub(crate) enum Msg {
    // Input
    SetKeybind {
        key: KeyIntOrString,
        modifiers: Vec<Modifier>,
        callback_id: CallbackId,
    },
    SetMousebind {
        modifiers: Vec<Modifier>,
        button: u32,
        edge: MouseEdge,
        callback_id: CallbackId,
    },

    // Window management
    CloseWindow {
        window_id: WindowId,
    },
    SetWindowSize {
        window_id: WindowId,
        #[serde(default)]
        width: Option<i32>,
        #[serde(default)]
        height: Option<i32>,
    },
    MoveWindowToTag {
        window_id: WindowId,
        tag_id: TagId,
    },
    ToggleTagOnWindow {
        window_id: WindowId,
        tag_id: TagId,
    },
    ToggleFloating {
        window_id: WindowId,
    },
    ToggleFullscreen {
        window_id: WindowId,
    },
    ToggleMaximized {
        window_id: WindowId,
    },
    AddWindowRule {
        cond: WindowRuleCondition,
        rule: WindowRule,
    },
    WindowMoveGrab {
        button: u32,
    },
    WindowResizeGrab {
        button: u32,
    },

    // Tag management
    ToggleTag {
        tag_id: TagId,
    },
    SwitchToTag {
        tag_id: TagId,
    },
    AddTags {
        /// The name of the output you want these tags on.
        output_name: OutputName,
        tag_names: Vec<String>,
    },
    RemoveTags {
        /// The name of the output you want these tags removed from.
        tag_ids: Vec<TagId>,
    },
    SetLayout {
        tag_id: TagId,
        layout: Layout,
    },

    // Output management
    ConnectForAllOutputs {
        callback_id: CallbackId,
    },
    SetOutputLocation {
        output_name: OutputName,
        #[serde(default)]
        x: Option<i32>,
        #[serde(default)]
        y: Option<i32>,
    },

    // Process management
    /// Spawn a program with an optional callback.
    Spawn {
        command: Vec<String>,
        #[serde(default)]
        callback_id: Option<CallbackId>,
    },
    SetEnv {
        key: String,
        value: String,
    },

    // Pinnacle management
    /// Quit the compositor.
    Quit,

    // Input management
    SetXkbConfig {
        #[serde(default)]
        rules: Option<String>,
        #[serde(default)]
        variant: Option<String>,
        #[serde(default)]
        layout: Option<String>,
        #[serde(default)]
        model: Option<String>,
        #[serde(default)]
        options: Option<String>,
    },

    SetLibinputSetting(LibinputSetting),

    Request {
        request_id: RequestId,
        request: Request,
    },
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
/// Messages that require a server response, usually to provide some data.
pub enum Request {
    // Windows
    GetWindows,
    GetWindowProps { window_id: WindowId },
    // Outputs
    GetOutputs,
    GetOutputProps { output_name: String },
    // Tags
    GetTags,
    GetTagProps { tag_id: TagId },
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub enum KeyIntOrString {
    Int(u32),
    String(String),
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub enum Args {
    /// Send a message with lines from the spawned process.
    Spawn {
        #[serde(default)]
        stdout: Option<String>,
        #[serde(default)]
        stderr: Option<String>,
        #[serde(default)]
        exit_code: Option<i32>,
        #[serde(default)]
        exit_msg: Option<String>,
    },
    ConnectForAllOutputs {
        output_name: String,
    },
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum IncomingMsg {
    CallCallback {
        callback_id: CallbackId,
        #[serde(default)]
        args: Option<Args>,
    },
    RequestResponse {
        request_id: RequestId,
        response: RequestResponse,
    },
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum RequestResponse {
    Window {
        window_id: Option<WindowId>,
    },
    Windows {
        window_ids: Vec<WindowId>,
    },
    WindowProps {
        size: Option<(i32, i32)>,
        loc: Option<(i32, i32)>,
        class: Option<String>,
        title: Option<String>,
        focused: Option<bool>,
        floating: Option<bool>,
        fullscreen_or_maximized: Option<FullscreenOrMaximized>,
    },
    Output {
        output_name: Option<String>,
    },
    Outputs {
        output_names: Vec<String>,
    },
    OutputProps {
        /// The make of the output.
        make: Option<String>,
        /// The model of the output.
        model: Option<String>,
        /// The location of the output in the space.
        loc: Option<(i32, i32)>,
        /// The resolution of the output.
        res: Option<(i32, i32)>,
        /// The refresh rate of the output.
        refresh_rate: Option<i32>,
        /// The size of the output, in millimeters.
        physical_size: Option<(i32, i32)>,
        /// Whether the output is focused or not.
        focused: Option<bool>,
        tag_ids: Option<Vec<TagId>>,
    },
    Tags {
        tag_ids: Vec<TagId>,
    },
    TagProps {
        active: Option<bool>,
        name: Option<String>,
        output_name: Option<String>,
    },
}
