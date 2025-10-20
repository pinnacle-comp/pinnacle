// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Output management.
//!
//! An output is the Wayland term for a monitor. It presents windows, your cursor, and other UI elements.
//!
//! Outputs are uniquely identified by their name, a.k.a. the name of the connector they're plugged in to.

use std::str::FromStr;

use futures::FutureExt;
use pinnacle_api_defs::pinnacle::{
    output::{
        self,
        v1::{
            FocusRequest, GetEnabledRequest, GetFocusStackWindowIdsRequest, GetFocusedRequest,
            GetInfoRequest, GetLocRequest, GetLogicalSizeRequest, GetModesRequest,
            GetOutputsInDirRequest, GetPhysicalSizeRequest, GetPoweredRequest, GetRequest,
            GetScaleRequest, GetTagIdsRequest, GetTransformRequest, SetLocRequest, SetModeRequest,
            SetModelineRequest, SetPoweredRequest, SetScaleRequest, SetTransformRequest,
            SetVrrRequest,
        },
    },
    util::v1::{AbsOrRel, SetOrToggle},
};

use crate::{
    BlockOnTokio,
    client::Client,
    signal::{OutputSignal, SignalHandle},
    tag::TagHandle,
    util::{Batch, Direction, Point, Size},
    window::WindowHandle,
};

/// Gets handles to all currently plugged-in outputs.
///
/// # Examples
///
/// ```no_run
/// # use pinnacle_api::output;
/// for output in output::get_all() {
///     println!("{} {} {}", output.make(), output.model(), output.serial());
/// }
/// ```
pub fn get_all() -> impl Iterator<Item = OutputHandle> {
    get_all_async().block_on_tokio()
}

/// Async impl for [`get_all`].
pub async fn get_all_async() -> impl Iterator<Item = OutputHandle> {
    Client::output()
        .get(GetRequest {})
        .await
        .unwrap()
        .into_inner()
        .output_names
        .into_iter()
        .map(|name| OutputHandle { name })
}

/// Gets handles to all currently plugged-in *and enabled* outputs.
///
/// This ignores outputs you have explicitly disabled.
///
/// # Examples
///
/// ```no_run
/// # use pinnacle_api::output;
/// for output in output::get_all_enabled() {
///     println!("{} {} {}", output.make(), output.model(), output.serial());
/// }
/// ```
pub fn get_all_enabled() -> impl Iterator<Item = OutputHandle> {
    get_all_enabled_async().block_on_tokio()
}

/// Async impl for [`get_all_enabled`].
pub async fn get_all_enabled_async() -> impl Iterator<Item = OutputHandle> {
    get_all_async()
        .await
        .batch_filter(|op| op.enabled_async().boxed(), |enabled| enabled)
}

/// Gets a handle to the output with the given name.
///
/// By "name", we mean the name of the connector the output is connected to.
pub fn get_by_name(name: impl ToString) -> Option<OutputHandle> {
    get_by_name_async(name).block_on_tokio()
}

/// Async impl for [`get_by_name`].
pub async fn get_by_name_async(name: impl ToString) -> Option<OutputHandle> {
    get_all_async().await.find(|op| op.name == name.to_string())
}

/// Gets a handle to the currently focused output.
///
/// This is currently implemented as the one that has had the most recent pointer movement.
pub fn get_focused() -> Option<OutputHandle> {
    get_focused_async().block_on_tokio()
}

/// Async impl for [`get_focused`].
pub async fn get_focused_async() -> Option<OutputHandle> {
    get_all_async()
        .await
        .batch_find(|op| op.focused_async().boxed(), |focused| *focused)
}

/// Runs a closure on all current and future outputs.
///
/// When called, this will do two things:
/// 1. Immediately run `for_each` with all currently connected outputs.
/// 2. Call `for_each` with any newly connected outputs.
///
/// Note that `for_each` will *not* run with outputs that have been unplugged and replugged.
/// This is to prevent duplicate setup. Instead, the compositor keeps track of any tags and
/// state the output had when unplugged and restores them on replug. This may change in the future.
///
/// # Examples
///
/// ```no_run
/// # use pinnacle_api::output;
/// # use pinnacle_api::tag;
/// // Add tags 1-3 to all outputs and set tag "1" to active
/// output::for_each_output(|op| {
///     let mut tags = tag::add(op, ["1", "2", "3"]);
///     tags.next().unwrap().set_active(true);
/// });
/// ```
pub fn for_each_output(mut for_each: impl FnMut(&OutputHandle) + Send + 'static) {
    for output in get_all() {
        for_each(&output);
    }

    Client::signal_state()
        .output_connect
        .add_callback(Box::new(for_each));
}

/// Connects to an [`OutputSignal`].
///
/// # Examples
///
/// ```no_run
/// # use pinnacle_api::output;
/// # use pinnacle_api::signal::OutputSignal;
/// output::connect_signal(OutputSignal::Connect(Box::new(|output| {
///     println!("New output: {}", output.name());
/// })));
/// ```
pub fn connect_signal(signal: OutputSignal) -> SignalHandle {
    let mut signal_state = Client::signal_state();

    match signal {
        OutputSignal::Connect(f) => signal_state.output_connect.add_callback(f),
        OutputSignal::Disconnect(f) => signal_state.output_disconnect.add_callback(f),
        OutputSignal::Setup(f) => signal_state.output_setup.add_callback(f),
        OutputSignal::Resize(f) => signal_state.output_resize.add_callback(f),
        OutputSignal::Move(f) => signal_state.output_move.add_callback(f),
        OutputSignal::PointerEnter(f) => signal_state.output_pointer_enter.add_callback(f),
        OutputSignal::PointerLeave(f) => signal_state.output_pointer_leave.add_callback(f),
        OutputSignal::Focused(f) => signal_state.output_focused.add_callback(f),
    }
}

/// A handle to an output.
///
/// This allows you to manipulate outputs and get their properties.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OutputHandle {
    pub(crate) name: String,
}

/// The alignment to use for [`OutputHandle::set_loc_adj_to`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Alignment {
    /// Set above, align left borders
    TopAlignLeft,
    /// Set above, align centers
    TopAlignCenter,
    /// Set above, align right borders
    TopAlignRight,
    /// Set below, align left borders
    BottomAlignLeft,
    /// Set below, align centers
    BottomAlignCenter,
    /// Set below, align right borders
    BottomAlignRight,
    /// Set to left, align top borders
    LeftAlignTop,
    /// Set to left, align centers
    LeftAlignCenter,
    /// Set to left, align bottom borders
    LeftAlignBottom,
    /// Set to right, align top borders
    RightAlignTop,
    /// Set to right, align centers
    RightAlignCenter,
    /// Set to right, align bottom borders
    RightAlignBottom,
}

/// An output transform.
///
/// This determines what orientation outputs will render with.
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Transform {
    /// No transform.
    #[default]
    Normal,
    /// 90 degrees counter-clockwise.
    _90,
    /// 180 degrees counter-clockwise.
    _180,
    /// 270 degrees counter-clockwise.
    _270,
    /// Flipped vertically (across the horizontal axis).
    Flipped,
    /// Flipped vertically and rotated 90 degrees counter-clockwise
    Flipped90,
    /// Flipped vertically and rotated 180 degrees counter-clockwise
    Flipped180,
    /// Flipped vertically and rotated 270 degrees counter-clockwise
    Flipped270,
}

impl TryFrom<output::v1::Transform> for Transform {
    type Error = ();

    fn try_from(value: output::v1::Transform) -> Result<Self, Self::Error> {
        match value {
            output::v1::Transform::Unspecified => Err(()),
            output::v1::Transform::Normal => Ok(Transform::Normal),
            output::v1::Transform::Transform90 => Ok(Transform::_90),
            output::v1::Transform::Transform180 => Ok(Transform::_180),
            output::v1::Transform::Transform270 => Ok(Transform::_270),
            output::v1::Transform::Flipped => Ok(Transform::Flipped),
            output::v1::Transform::Flipped90 => Ok(Transform::Flipped90),
            output::v1::Transform::Flipped180 => Ok(Transform::Flipped180),
            output::v1::Transform::Flipped270 => Ok(Transform::Flipped270),
        }
    }
}

impl From<Transform> for output::v1::Transform {
    fn from(value: Transform) -> Self {
        match value {
            Transform::Normal => output::v1::Transform::Normal,
            Transform::_90 => output::v1::Transform::Transform90,
            Transform::_180 => output::v1::Transform::Transform180,
            Transform::_270 => output::v1::Transform::Transform270,
            Transform::Flipped => output::v1::Transform::Flipped,
            Transform::Flipped90 => output::v1::Transform::Flipped90,
            Transform::Flipped180 => output::v1::Transform::Flipped180,
            Transform::Flipped270 => output::v1::Transform::Flipped270,
        }
    }
}

/// The state of variable refresh rate on an output.
#[doc(alias = "AdaptiveSync")]
#[doc(alias = "VariableRefreshRate")]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Vrr {
    /// Variable refresh rate should be off.
    #[default]
    Off,
    /// Variable refresh rate should be on at all times.
    AlwaysOn,
    /// Variable refresh rate should be on when a window with an
    /// active [`VrrDemand`](crate::window::VrrDemand) is visible.
    OnDemand,
}

impl OutputHandle {
    /// Creates an output handle from a name.
    pub fn from_name(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    /// Sets the location of this output in the global space.
    ///
    /// On startup, Pinnacle will lay out all connected outputs starting at (0, 0)
    /// and going to the right, with their top borders aligned.
    ///
    /// This method allows you to move outputs where necessary.
    ///
    /// Note: If you leave space between two outputs when setting their locations,
    /// the pointer will not be able to move between them.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pinnacle_api::output;
    /// // Assume two monitors in order, "DP-1" and "HDMI-1", with the following dimensions:
    /// //  - "DP-1":   ┌─────┐
    /// //              │     │1920x1080
    /// //              └─────┘
    /// //  - "HDMI-1": ┌───────┐
    /// //              │ 2560x │
    /// //              │ 1440  │
    /// //              └───────┘
    /// # || {
    /// output::get_by_name("DP-1")?.set_loc(0, 0);
    /// output::get_by_name("HDMI-1")?.set_loc(1920, -360);
    /// # Some(())
    /// # };
    /// // Results in:
    /// //   x=0    ┌───────┐y=-360
    /// // y=0┌─────┤       │
    /// //    │DP-1 │HDMI-1 │
    /// //    └─────┴───────┘
    /// //          ^x=1920
    /// ```
    pub fn set_loc(&self, x: i32, y: i32) {
        Client::output()
            .set_loc(SetLocRequest {
                output_name: self.name(),
                x,
                y,
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Sets this output adjacent to another one.
    ///
    /// This is a helper method over [`OutputHandle::set_loc`] to make laying out outputs
    /// easier.
    ///
    /// `alignment` is an [`Alignment`] of how you want this output to be placed.
    /// For example, [`TopAlignLeft`][Alignment::TopAlignLeft] will place this output
    /// above `other` and align the left borders.
    /// Similarly, [`RightAlignCenter`][Alignment::RightAlignCenter] will place this output
    /// to the right of `other` and align their centers.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pinnacle_api::output;
    /// # use pinnacle_api::output::Alignment;
    /// // Assume two monitors in order, "DP-1" and "HDMI-1", with the following dimensions:
    /// //  - "DP-1":   ┌─────┐
    /// //              │     │1920x1080
    /// //              └─────┘
    /// //  - "HDMI-1": ┌───────┐
    /// //              │ 2560x │
    /// //              │ 1440  │
    /// //              └───────┘
    /// # || {
    /// let dp_1 = output::get_by_name("DP-1")?;
    /// let hdmi_1 = output::get_by_name("HDMI-1")?;
    /// dp_1.set_loc_adj_to(&hdmi_1, Alignment::BottomAlignRight);
    /// # Some(())
    /// # };
    /// // Results in:
    /// // ┌───────┐
    /// // │       │
    /// // │HDMI-1 │
    /// // └──┬────┤
    /// //    │DP-1│
    /// //    └────┘
    /// // Notice that "DP-1" now has the coordinates (2280, 1440) because "DP-1" is getting moved, not "HDMI-1".
    /// // "HDMI-1" was placed at (1920, 0) during the compositor's initial output layout.
    /// ```
    pub fn set_loc_adj_to(&self, other: &OutputHandle, alignment: Alignment) {
        let (self_size, other_loc, other_size) = async {
            tokio::join!(
                self.logical_size_async(),
                other.loc_async(),
                other.logical_size_async()
            )
        }
        .block_on_tokio();

        // poor man's try {}
        let attempt_set_loc = || -> Option<()> {
            let other_x = other_loc?.x;
            let other_y = other_loc?.y;
            let other_width = other_size?.w as i32;
            let other_height = other_size?.h as i32;

            let self_width = self_size?.w as i32;
            let self_height = self_size?.h as i32;

            use Alignment::*;

            let x: i32;
            let y: i32;

            if let TopAlignLeft | TopAlignCenter | TopAlignRight | BottomAlignLeft
            | BottomAlignCenter | BottomAlignRight = alignment
            {
                if let TopAlignLeft | TopAlignCenter | TopAlignRight = alignment {
                    y = other_y - self_height;
                } else {
                    // bottom
                    y = other_y + other_height;
                }

                match alignment {
                    TopAlignLeft | BottomAlignLeft => x = other_x,
                    TopAlignCenter | BottomAlignCenter => {
                        x = other_x + (other_width - self_width) / 2;
                    }
                    TopAlignRight | BottomAlignRight => x = other_x + (other_width - self_width),
                    _ => unreachable!(),
                }
            } else {
                if let LeftAlignTop | LeftAlignCenter | LeftAlignBottom = alignment {
                    x = other_x - self_width;
                } else {
                    x = other_x + other_width;
                }

                match alignment {
                    LeftAlignTop | RightAlignTop => y = other_y,
                    LeftAlignCenter | RightAlignCenter => {
                        y = other_y + (other_height - self_height) / 2;
                    }
                    LeftAlignBottom | RightAlignBottom => {
                        y = other_y + (other_height - self_height);
                    }
                    _ => unreachable!(),
                }
            }

            self.set_loc(x, y);

            Some(())
        };

        attempt_set_loc();
    }

    /// Sets this output's mode.
    ///
    /// If `refresh_rate_mhz` is provided, Pinnacle will attempt to use the mode with that
    /// refresh rate. If it is not, Pinnacle will attempt to use the mode with the
    /// highest refresh rate that matches the given size.
    ///
    /// The refresh rate should be given in millihertz. For example, if you want a refresh rate of
    /// 60Hz, use 60000.
    ///
    /// If this output doesn't support the given mode, it will be ignored.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pinnacle_api::output;
    /// # || {
    /// // Sets the focused output to 2560x1440 at 144Hz
    /// output::get_focused()?.set_mode(2560, 1440, 144000);
    /// # Some(())
    /// # };
    /// ```
    pub fn set_mode(&self, width: u32, height: u32, refresh_rate_mhz: impl Into<Option<u32>>) {
        Client::output()
            .set_mode(SetModeRequest {
                output_name: self.name(),
                size: Some(pinnacle_api_defs::pinnacle::util::v1::Size { width, height }),
                refresh_rate_mhz: refresh_rate_mhz.into(),
                custom: false,
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Sets this output's mode to a custom one.
    ///
    /// If `refresh_rate_mhz` is provided, Pinnacle will create a new mode with that refresh rate.
    /// If it is not, it will default to 60Hz.
    ///
    /// The refresh rate should be given in millihertz. For example, if you want a refresh rate of
    /// 60Hz, use 60000.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pinnacle_api::output;
    /// # || {
    /// // Sets the focused output to 2560x1440 at 75Hz
    /// output::get_focused()?.set_custom_mode(2560, 1440, 75000);
    /// # Some(())
    /// # };
    /// ```
    pub fn set_custom_mode(
        &self,
        width: u32,
        height: u32,
        refresh_rate_mhz: impl Into<Option<u32>>,
    ) {
        Client::output()
            .set_mode(SetModeRequest {
                output_name: self.name(),
                size: Some(pinnacle_api_defs::pinnacle::util::v1::Size { width, height }),
                refresh_rate_mhz: refresh_rate_mhz.into(),
                custom: true,
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Sets a custom modeline for this output.
    ///
    /// See `xorg.conf(5)` for more information.
    ///
    /// You can parse a modeline from a string of the form
    /// "\<clock> \<hdisplay> \<hsync_start> \<hsync_end> \<htotal> \<vdisplay> \<vsync_start> \<vsync_end> \<hsync> \<vsync>".
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pinnacle_api::output;
    /// # || {
    /// let output = output::get_focused()?;
    /// output.set_modeline("173.00 1920 2048 2248 2576 1080 1083 1088 1120 -hsync +vsync".parse().unwrap());
    /// # Some(())
    /// # };
    /// ```
    pub fn set_modeline(&self, modeline: Modeline) {
        Client::output()
            .set_modeline(SetModelineRequest {
                output_name: self.name(),
                modeline: Some(modeline.into()),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Sets this output's scaling factor.
    pub fn set_scale(&self, scale: f32) {
        Client::output()
            .set_scale(SetScaleRequest {
                output_name: self.name(),
                scale,
                abs_or_rel: AbsOrRel::Absolute.into(),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Changes this output's scaling factor by a relative amount.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pinnacle_api::output;
    /// # || {
    /// output::get_focused()?.change_scale(0.25);
    /// output::get_focused()?.change_scale(-0.25);
    /// # Some(())
    /// # };
    /// ```
    pub fn change_scale(&self, change_by: f32) {
        Client::output()
            .set_scale(SetScaleRequest {
                output_name: self.name(),
                scale: change_by,
                abs_or_rel: AbsOrRel::Relative.into(),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Sets this output's [`Transform`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pinnacle_api::output;
    /// # use pinnacle_api::output::Transform;
    /// // Rotate 90 degrees counter-clockwise
    /// # || {
    /// output::get_focused()?.set_transform(Transform::_90);
    /// # Some(())
    /// # };
    /// ```
    pub fn set_transform(&self, transform: Transform) {
        Client::output()
            .set_transform(SetTransformRequest {
                output_name: self.name(),
                transform: output::v1::Transform::from(transform).into(),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Powers on or off this output.
    ///
    /// This will not remove it from the space and your tags and windows
    /// will still be interactable; only the monitor is turned off.
    pub fn set_powered(&self, powered: bool) {
        Client::output()
            .set_powered(SetPoweredRequest {
                output_name: self.name(),
                set_or_toggle: match powered {
                    true => SetOrToggle::Set,
                    false => SetOrToggle::Unset,
                }
                .into(),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Toggles the power on this output.
    ///
    /// This will not remove it from the space and your tags and windows
    /// will still be interactable; only the monitor is turned off.
    pub fn toggle_powered(&self) {
        Client::output()
            .set_powered(SetPoweredRequest {
                output_name: self.name(),
                set_or_toggle: SetOrToggle::Toggle.into(),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Sets the variable refresh rate state of this output.
    ///
    /// See [`Vrr`] for possible states and their behaviors.
    #[doc(alias = "set_adaptive_sync")]
    #[doc(alias = "set_variable_refresh_rate")]
    pub fn set_vrr(&self, vrr: Vrr) {
        Client::output()
            .set_vrr(SetVrrRequest {
                output_name: self.name(),
                vrr: match vrr {
                    Vrr::Off => output::v1::Vrr::Off,
                    Vrr::AlwaysOn => output::v1::Vrr::AlwaysOn,
                    Vrr::OnDemand => output::v1::Vrr::OnDemand,
                } as i32,
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Focuses this output.
    pub fn focus(&self) {
        Client::output()
            .focus(FocusRequest {
                output_name: self.name(),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Gets this output's make.
    pub fn make(&self) -> String {
        self.make_async().block_on_tokio()
    }

    /// Async impl for [`Self::make`].
    pub async fn make_async(&self) -> String {
        Client::output()
            .get_info(GetInfoRequest {
                output_name: self.name(),
            })
            .await
            .unwrap()
            .into_inner()
            .make
    }

    /// Gets this output's model.
    pub fn model(&self) -> String {
        self.model_async().block_on_tokio()
    }

    /// Async impl for [`Self::model`].
    pub async fn model_async(&self) -> String {
        Client::output()
            .get_info(GetInfoRequest {
                output_name: self.name(),
            })
            .await
            .unwrap()
            .into_inner()
            .model
    }

    /// Gets this output's serial.
    pub fn serial(&self) -> String {
        self.serial_async().block_on_tokio()
    }

    /// Async impl for [`Self::serial`].
    pub async fn serial_async(&self) -> String {
        Client::output()
            .get_info(GetInfoRequest {
                output_name: self.name(),
            })
            .await
            .unwrap()
            .into_inner()
            .serial
    }

    /// Gets this output's location in the global space.
    ///
    /// May return `None` if it is disabled.
    pub fn loc(&self) -> Option<Point> {
        self.loc_async().block_on_tokio()
    }

    /// Async impl for [`Self::loc`].
    pub async fn loc_async(&self) -> Option<Point> {
        Client::output()
            .get_loc(GetLocRequest {
                output_name: self.name(),
            })
            .await
            .unwrap()
            .into_inner()
            .loc
            .map(|loc| Point { x: loc.x, y: loc.y })
    }

    /// Gets this output's logical size in logical pixels.
    ///
    /// If this output has a scale of 1, this will equal the output's
    /// actual pixel size. If it has a scale of 2, it will have half the
    /// logical pixel width and height. Similarly, if it has a scale of 0.5,
    /// it will have double the logical pixel width and height.
    ///
    /// May return `None` if it is disabled.
    pub fn logical_size(&self) -> Option<Size> {
        self.logical_size_async().block_on_tokio()
    }

    /// Async impl for [`Self::logical_size`].
    pub async fn logical_size_async(&self) -> Option<Size> {
        Client::output()
            .get_logical_size(GetLogicalSizeRequest {
                output_name: self.name(),
            })
            .await
            .unwrap()
            .into_inner()
            .logical_size
            .map(|size| Size {
                w: size.width,
                h: size.height,
            })
    }

    /// Gets this output's current mode.
    ///
    /// May return `None` if it is disabled.
    pub fn current_mode(&self) -> Option<Mode> {
        self.current_mode_async().block_on_tokio()
    }

    /// Async impl for [`Self::current_mode`].
    pub async fn current_mode_async(&self) -> Option<Mode> {
        Client::output()
            .get_modes(GetModesRequest {
                output_name: self.name(),
            })
            .await
            .unwrap()
            .into_inner()
            .current_mode
            .map(|mode| Mode {
                size: Size {
                    w: mode.size.expect("mode should have a size").width,
                    h: mode.size.expect("mode should have a size").height,
                },
                refresh_rate_mhz: mode.refresh_rate_mhz,
            })
    }

    /// Gets this output's preferred mode.
    ///
    /// May return `None` if it is disabled.
    pub fn preferred_mode(&self) -> Option<Mode> {
        self.preferred_mode_async().block_on_tokio()
    }

    /// Async impl for [`Self::preferred_mode`].
    pub async fn preferred_mode_async(&self) -> Option<Mode> {
        Client::output()
            .get_modes(GetModesRequest {
                output_name: self.name(),
            })
            .await
            .unwrap()
            .into_inner()
            .preferred_mode
            .map(|mode| Mode {
                size: Size {
                    w: mode.size.expect("mode should have a size").width,
                    h: mode.size.expect("mode should have a size").height,
                },
                refresh_rate_mhz: mode.refresh_rate_mhz,
            })
    }

    /// Gets all modes currently known to this output.
    pub fn modes(&self) -> impl Iterator<Item = Mode> + use<> {
        self.modes_async().block_on_tokio()
    }

    /// Async impl for [`Self::modes`].
    pub async fn modes_async(&self) -> impl Iterator<Item = Mode> + use<> {
        Client::output()
            .get_modes(GetModesRequest {
                output_name: self.name(),
            })
            .await
            .unwrap()
            .into_inner()
            .modes
            .into_iter()
            .map(|mode| Mode {
                size: Size {
                    w: mode.size.expect("mode should have a size").width,
                    h: mode.size.expect("mode should have a size").height,
                },
                refresh_rate_mhz: mode.refresh_rate_mhz,
            })
    }

    /// Gets this output's physical size in millimeters.
    ///
    /// Returns a size of (0, 0) if unknown.
    pub fn physical_size(&self) -> Size {
        self.physical_size_async().block_on_tokio()
    }

    /// Async impl for [`Self::physical_size`].
    pub async fn physical_size_async(&self) -> Size {
        Client::output()
            .get_physical_size(GetPhysicalSizeRequest {
                output_name: self.name(),
            })
            .await
            .unwrap()
            .into_inner()
            .physical_size
            .map(|size| Size {
                w: size.width,
                h: size.height,
            })
            .unwrap_or_default()
    }

    /// Gets whether or not this output is focused.
    ///
    /// This is currently implemented as the output with the most recent pointer motion.
    pub fn focused(&self) -> bool {
        self.focused_async().block_on_tokio()
    }

    /// Async impl for [`Self::focused`].
    pub async fn focused_async(&self) -> bool {
        Client::output()
            .get_focused(GetFocusedRequest {
                output_name: self.name(),
            })
            .await
            .unwrap()
            .into_inner()
            .focused
    }

    /// Gets handles to all tags on this output.
    pub fn tags(&self) -> impl Iterator<Item = TagHandle> + use<> {
        self.tags_async().block_on_tokio()
    }

    /// Async impl for [`Self::tags`].
    pub async fn tags_async(&self) -> impl Iterator<Item = TagHandle> + use<> {
        Client::output()
            .get_tag_ids(GetTagIdsRequest {
                output_name: self.name(),
            })
            .await
            .unwrap()
            .into_inner()
            .tag_ids
            .into_iter()
            .map(|id| TagHandle { id })
    }

    /// Gets handles to all active tags on this output.
    pub fn active_tags(&self) -> impl Iterator<Item = TagHandle> + use<> {
        self.active_tags_async().block_on_tokio()
    }

    /// Async impl for [`Self::active_tags`].
    pub async fn active_tags_async(&self) -> impl Iterator<Item = TagHandle> + use<> {
        self.tags_async()
            .await
            .batch_filter(|tag| tag.active_async().boxed(), |is_active| is_active)
    }

    /// Gets handles to all inactive tags on this output.
    pub fn inactive_tags(&self) -> impl Iterator<Item = TagHandle> + use<> {
        self.inactive_tags_async().block_on_tokio()
    }

    /// Async impl for [`Self::active_tags`].
    pub async fn inactive_tags_async(&self) -> impl Iterator<Item = TagHandle> + use<> {
        self.tags_async()
            .await
            .batch_filter(|tag| tag.active_async().boxed(), |is_active| !is_active)
    }

    /// Gets this output's current scale.
    pub fn scale(&self) -> f32 {
        self.scale_async().block_on_tokio()
    }

    /// Async impl for [`Self::scale`].
    pub async fn scale_async(&self) -> f32 {
        Client::output()
            .get_scale(GetScaleRequest {
                output_name: self.name(),
            })
            .await
            .unwrap()
            .into_inner()
            .scale
    }

    /// Gets this output's current transform.
    pub fn transform(&self) -> Transform {
        self.transform_async().block_on_tokio()
    }

    /// Async impl for [`Self::transform`].
    pub async fn transform_async(&self) -> Transform {
        Client::output()
            .get_transform(GetTransformRequest {
                output_name: self.name(),
            })
            .await
            .unwrap()
            .into_inner()
            .transform()
            .try_into()
            .unwrap_or_default()
    }

    /// Gets this window's keyboard focus stack.
    ///
    /// Pinnacle keeps a stack of the windows that get keyboard focus.
    /// This can be used, for example, for an `Alt + Tab`-style keybind
    /// that focuses the previously focused window.
    ///
    /// This will return the focus stack containing *all* windows on this output.
    /// If you only want windows on active tags, see
    /// [`OutputHandle::keyboard_focus_stack_visible`].
    pub fn keyboard_focus_stack(&self) -> impl Iterator<Item = WindowHandle> + use<> {
        self.keyboard_focus_stack_async().block_on_tokio()
    }

    /// Async impl for [`Self::keyboard_focus_stack`].
    pub async fn keyboard_focus_stack_async(&self) -> impl Iterator<Item = WindowHandle> + use<> {
        Client::output()
            .get_focus_stack_window_ids(GetFocusStackWindowIdsRequest {
                output_name: self.name(),
            })
            .await
            .unwrap()
            .into_inner()
            .window_ids
            .into_iter()
            .map(|id| WindowHandle { id })
    }

    /// Gets this window's keyboard focus stack with only windows on active tags.
    ///
    /// Pinnacle keeps a stack of the windows that get keyboard focus.
    /// This can be used, for example, for an `Alt + Tab`-style keybind
    /// that focuses the previously focused window.
    ///
    /// This will return the focus stack containing only windows on active tags on this output.
    /// If you want *all* windows on this output, see [`OutputHandle::keyboard_focus_stack`].
    pub fn keyboard_focus_stack_visible(&self) -> impl Iterator<Item = WindowHandle> + use<> {
        self.keyboard_focus_stack_visible_async().block_on_tokio()
    }

    /// Async impl for [`Self::keyboard_focus_stack_visible`].
    pub async fn keyboard_focus_stack_visible_async(
        &self,
    ) -> impl Iterator<Item = WindowHandle> + use<> {
        self.keyboard_focus_stack_async()
            .await
            .batch_filter(|win| win.is_on_active_tag_async().boxed(), |is_on| is_on)
    }

    /// Gets whether this output is enabled.
    pub fn enabled(&self) -> bool {
        self.enabled_async().block_on_tokio()
    }

    /// Async impl for [`Self::enabled`].
    pub async fn enabled_async(&self) -> bool {
        Client::output()
            .get_enabled(GetEnabledRequest {
                output_name: self.name(),
            })
            .await
            .unwrap()
            .into_inner()
            .enabled
    }

    /// Gets whether or not this output is powered.
    ///
    /// Unpowered outputs are turned off but you can still interact with them.
    pub fn powered(&self) -> bool {
        self.powered_async().block_on_tokio()
    }

    /// Async impl for [`Self::powered`].
    pub async fn powered_async(&self) -> bool {
        Client::output()
            .get_powered(GetPoweredRequest {
                output_name: self.name(),
            })
            .await
            .unwrap()
            .into_inner()
            .powered
    }

    /// Gets all outputs in the provided direction, sorted closest to farthest.
    pub fn in_direction(&self, direction: Direction) -> impl Iterator<Item = OutputHandle> + use<> {
        self.in_direction_async(direction).block_on_tokio()
    }

    /// Async impl for [`Self::in_direction`].
    pub async fn in_direction_async(
        &self,
        direction: Direction,
    ) -> impl Iterator<Item = OutputHandle> + use<> {
        let output_name = self.name();

        let mut request = GetOutputsInDirRequest {
            output_name,
            dir: Default::default(),
        };

        request.set_dir(match direction {
            Direction::Left => pinnacle_api_defs::pinnacle::util::v1::Dir::Left,
            Direction::Right => pinnacle_api_defs::pinnacle::util::v1::Dir::Right,
            Direction::Up => pinnacle_api_defs::pinnacle::util::v1::Dir::Up,
            Direction::Down => pinnacle_api_defs::pinnacle::util::v1::Dir::Down,
        });

        let response = Client::output()
            .get_outputs_in_dir(request)
            .await
            .unwrap()
            .into_inner();

        response
            .output_names
            .into_iter()
            .map(OutputHandle::from_name)
    }

    /// Returns this output's unique name (the name of its connector).
    pub fn name(&self) -> String {
        self.name.to_string()
    }
}

/// A possible output pixel dimension and refresh rate configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Mode {
    /// The size of the mode, in pixels.
    pub size: Size,
    /// The output's refresh rate, in millihertz.
    ///
    /// For example, 60Hz is returned as 60000.
    pub refresh_rate_mhz: u32,
}

/// A custom modeline.
#[allow(missing_docs)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct Modeline {
    pub clock: f32,
    pub hdisplay: u32,
    pub hsync_start: u32,
    pub hsync_end: u32,
    pub htotal: u32,
    pub vdisplay: u32,
    pub vsync_start: u32,
    pub vsync_end: u32,
    pub vtotal: u32,
    pub hsync: bool,
    pub vsync: bool,
}

impl From<Modeline> for output::v1::Modeline {
    fn from(modeline: Modeline) -> Self {
        output::v1::Modeline {
            clock: modeline.clock,
            hdisplay: modeline.hdisplay,
            hsync_start: modeline.hsync_start,
            hsync_end: modeline.hsync_end,
            htotal: modeline.htotal,
            vdisplay: modeline.vdisplay,
            vsync_start: modeline.vsync_start,
            vsync_end: modeline.vsync_end,
            vtotal: modeline.vtotal,
            hsync: modeline.hsync,
            vsync: modeline.vsync,
        }
    }
}

/// Error for the `FromStr` implementation for [`Modeline`].
#[derive(Debug)]
pub struct ParseModelineError(ParseModelineErrorKind);

#[derive(Debug)]
enum ParseModelineErrorKind {
    NoClock,
    InvalidClock,
    NoHdisplay,
    InvalidHdisplay,
    NoHsyncStart,
    InvalidHsyncStart,
    NoHsyncEnd,
    InvalidHsyncEnd,
    NoHtotal,
    InvalidHtotal,
    NoVdisplay,
    InvalidVdisplay,
    NoVsyncStart,
    InvalidVsyncStart,
    NoVsyncEnd,
    InvalidVsyncEnd,
    NoVtotal,
    InvalidVtotal,
    NoHsync,
    InvalidHsync,
    NoVsync,
    InvalidVsync,
}

impl std::fmt::Display for ParseModelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

impl From<ParseModelineErrorKind> for ParseModelineError {
    fn from(value: ParseModelineErrorKind) -> Self {
        Self(value)
    }
}

impl FromStr for Modeline {
    type Err = ParseModelineError;

    /// Tries to convert the provided modeline string to a [`Modeline`].
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut args = s.split_whitespace();

        let clock = args
            .next()
            .ok_or(ParseModelineErrorKind::NoClock)?
            .parse()
            .map_err(|_| ParseModelineErrorKind::InvalidClock)?;
        let hdisplay = args
            .next()
            .ok_or(ParseModelineErrorKind::NoHdisplay)?
            .parse()
            .map_err(|_| ParseModelineErrorKind::InvalidHdisplay)?;
        let hsync_start = args
            .next()
            .ok_or(ParseModelineErrorKind::NoHsyncStart)?
            .parse()
            .map_err(|_| ParseModelineErrorKind::InvalidHsyncStart)?;
        let hsync_end = args
            .next()
            .ok_or(ParseModelineErrorKind::NoHsyncEnd)?
            .parse()
            .map_err(|_| ParseModelineErrorKind::InvalidHsyncEnd)?;
        let htotal = args
            .next()
            .ok_or(ParseModelineErrorKind::NoHtotal)?
            .parse()
            .map_err(|_| ParseModelineErrorKind::InvalidHtotal)?;
        let vdisplay = args
            .next()
            .ok_or(ParseModelineErrorKind::NoVdisplay)?
            .parse()
            .map_err(|_| ParseModelineErrorKind::InvalidVdisplay)?;
        let vsync_start = args
            .next()
            .ok_or(ParseModelineErrorKind::NoVsyncStart)?
            .parse()
            .map_err(|_| ParseModelineErrorKind::InvalidVsyncStart)?;
        let vsync_end = args
            .next()
            .ok_or(ParseModelineErrorKind::NoVsyncEnd)?
            .parse()
            .map_err(|_| ParseModelineErrorKind::InvalidVsyncEnd)?;
        let vtotal = args
            .next()
            .ok_or(ParseModelineErrorKind::NoVtotal)?
            .parse()
            .map_err(|_| ParseModelineErrorKind::InvalidVtotal)?;

        let hsync = match args
            .next()
            .ok_or(ParseModelineErrorKind::NoHsync)?
            .to_lowercase()
            .as_str()
        {
            "+hsync" => true,
            "-hsync" => false,
            _ => Err(ParseModelineErrorKind::InvalidHsync)?,
        };
        let vsync = match args
            .next()
            .ok_or(ParseModelineErrorKind::NoVsync)?
            .to_lowercase()
            .as_str()
        {
            "+vsync" => true,
            "-vsync" => false,
            _ => Err(ParseModelineErrorKind::InvalidVsync)?,
        };

        Ok(Modeline {
            clock,
            hdisplay,
            hsync_start,
            hsync_end,
            htotal,
            vdisplay,
            vsync_start,
            vsync_end,
            vtotal,
            hsync,
            vsync,
        })
    }
}
