// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Output management.
//!
//! An output is Pinnacle's terminology for a monitor.
//!
//! This module provides [`Output`], which allows you to get [`OutputHandle`]s for different
//! connected monitors and set them up.

use std::str::FromStr;

use futures::FutureExt;
use pinnacle_api_defs::pinnacle::{
    output::{
        self,
        v1::{
            GetEnabledRequest, GetFocusStackWindowIdsRequest, GetFocusedRequest, GetInfoRequest,
            GetLocRequest, GetLogicalSizeRequest, GetModesRequest, GetPhysicalSizeRequest,
            GetPoweredRequest, GetRequest, GetScaleRequest, GetTagIdsRequest, GetTransformRequest,
            SetLocRequest, SetModeRequest, SetModelineRequest, SetPoweredRequest, SetScaleRequest,
            SetTransformRequest,
        },
    },
    util::v1::{AbsOrRel, SetOrToggle},
};

use crate::{
    client::Client,
    signal::{OutputSignal, SignalHandle},
    tag::TagHandle,
    util::{Batch, Point, Size},
    window::WindowHandle,
    BlockOnTokio,
};

pub fn get_all() -> impl Iterator<Item = OutputHandle> {
    get_all_async().block_on_tokio()
}

/// Get handles to all connected outputs.
///
/// # Examples
///
/// ```
///
///
/// let outputs = output.get_all();
/// ```
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

pub fn get_all_enabled() -> impl Iterator<Item = OutputHandle> {
    get_all_enabled_async().block_on_tokio()
}

/// Get handles to all outputs that are connected and enabled.
///
/// # Examples
///
/// ```
///
///
/// let enabled = output.get_all_enabled();
/// ```
pub async fn get_all_enabled_async() -> impl Iterator<Item = OutputHandle> {
    get_all_async()
        .await
        .batch_filter(|op| op.enabled_async().boxed(), |enabled| *enabled)
}

pub fn get_by_name(name: impl ToString) -> Option<OutputHandle> {
    get_by_name_async(name).block_on_tokio()
}

/// Get a handle to the output with the given name.
///
/// By "name", we mean the name of the connector the output is connected to.
///
/// # Examples
///
/// ```
/// let op = output.get_by_name("eDP-1")?;
///
///
/// let op2 = output.get_by_name("HDMI-2")?;
/// ```
pub async fn get_by_name_async(name: impl ToString) -> Option<OutputHandle> {
    get_all_async().await.find(|op| op.name == name.to_string())
}

pub fn get_focused() -> Option<OutputHandle> {
    get_focused_async().block_on_tokio()
}

/// Get a handle to the focused output.
///
/// This is currently implemented as the one that has had the most recent pointer movement.
///
/// # Examples
///
/// ```
///
///
/// let op = output.get_focused()?;
/// ```
pub async fn get_focused_async() -> Option<OutputHandle> {
    get_all_async()
        .await
        .batch_find(|op| op.focused_async().boxed(), |focused| *focused)
}

/// Connect a closure to be run on all current and future outputs.
///
/// When called, `connect_for_all` will do two things:
/// 1. Immediately run `for_all` with all currently connected outputs.
/// 2. Create a future that will call `for_all` with any newly connected outputs.
///
/// Note that `for_all` will *not* run with outputs that have been unplugged and replugged.
/// This is to prevent duplicate setup. Instead, the compositor keeps track of any tags and
/// state the output had when unplugged and restores them on replug.
///
/// # Examples
///
/// ```
/// // Add tags 1-3 to all outputs and set tag "1" to active
/// output.connect_for_all(|op| {
///     let tags = tag.add(&op, ["1", "2", "3"]);
///     tags.first()?.set_active(true);
/// });
/// ```
pub fn for_all_outputs(mut for_all: impl FnMut(&OutputHandle) + Send + 'static) {
    for output in get_all() {
        for_all(&output);
    }

    Client::signal_state()
        .output_connect
        .add_callback(Box::new(for_all));
}

/// Connect to an output signal.
///
/// The compositor will fire off signals that your config can listen for and act upon.
/// You can pass in an [`OutputSignal`] along with a callback and it will get run
/// with the necessary arguments every time a signal of that type is received.
pub fn connect_signal(signal: OutputSignal) -> SignalHandle {
    let mut signal_state = Client::signal_state();

    match signal {
        OutputSignal::Connect(f) => signal_state.output_connect.add_callback(f),
        OutputSignal::Disconnect(f) => signal_state.output_disconnect.add_callback(f),
        OutputSignal::Resize(f) => signal_state.output_resize.add_callback(f),
        OutputSignal::Move(f) => signal_state.output_move.add_callback(f),
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
/// This determines what orientation outputs will render at.
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

impl OutputHandle {
    /// Set the location of this output in the global space.
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
    /// ```
    /// // Assume two monitors in order, "DP-1" and "HDMI-1", with the following dimensions:
    /// //  - "DP-1":   ┌─────┐
    /// //              │     │1920x1080
    /// //              └─────┘
    /// //  - "HDMI-1": ┌───────┐
    /// //              │ 2560x │
    /// //              │ 1440  │
    /// //              └───────┘
    ///
    /// output.get_by_name("DP-1")?.set_location(0, 0);
    /// output.get_by_name("HDMI-1")?.set_location(1920, -360);
    ///
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

    /// Set this output adjacent to another one.
    ///
    /// This is a helper method over [`OutputHandle::set_location`] to make laying out outputs
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
    /// ```
    /// use pinnacle_api::output::Alignment;
    ///
    /// // Assume two monitors in order, "DP-1" and "HDMI-1", with the following dimensions:
    /// //  - "DP-1":   ┌─────┐
    /// //              │     │1920x1080
    /// //              └─────┘
    /// //  - "HDMI-1": ┌───────┐
    /// //              │ 2560x │
    /// //              │ 1440  │
    /// //              └───────┘
    ///
    /// output.get_by_name("DP-1")?.set_loc_adj_to(output.get_by_name("HDMI-1")?, Alignment::BottomAlignRight);
    ///
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

    /// Set this output's mode.
    ///
    /// If `refresh_rate_millihertz` is provided, Pinnacle will attempt to use the mode with that
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
    /// ```
    /// output.get_focused()?.set_mode(2560, 1440, 144000);
    /// ```
    pub fn set_mode(&self, width: u32, height: u32, refresh_rate_mhz: impl Into<Option<u32>>) {
        Client::output()
            .set_mode(SetModeRequest {
                output_name: self.name(),
                size: Some(pinnacle_api_defs::pinnacle::util::v1::Size { width, height }),
                refresh_rate_mhz: refresh_rate_mhz.into(),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Set a custom modeline for this output.
    ///
    /// See `xorg.conf(5)` for more information.
    ///
    /// You can parse a modeline from a string of the form
    /// "<clock> <hdisplay> <hsync_start> <hsync_end> <htotal> <vdisplay> <vsync_start> <vsync_end> <hsync> <vsync>".
    ///
    /// # Examples
    ///
    /// ```
    /// output.set_modeline("173.00 1920 2048 2248 2576 1080 1083 1088 1120 -hsync +vsync".parse()?);
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

    /// Set this output's scaling factor.
    ///
    /// # Examples
    ///
    /// ```
    /// output.get_focused()?.set_scale(1.5);
    /// ```
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

    /// Increase this output's scaling factor by `increase_by`.
    ///
    /// # Examples
    ///
    /// ```
    /// output.get_focused()?.increase_scale(0.25);
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

    /// Set this output's transform.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::output::Transform;
    ///
    /// // Rotate 90 degrees counter-clockwise
    /// output.set_transform(Transform::_90);
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

    /// Power on or off this output.
    ///
    /// This will not remove it from the space and your tags and windows
    /// will still be interactable; only the monitor is turned off.
    ///
    /// # Examples
    ///
    /// ```
    /// // Power off `output`
    /// output.set_powered(false);
    /// ```
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

    pub fn toggle_powered(&self) {
        Client::output()
            .set_powered(SetPoweredRequest {
                output_name: self.name(),
                set_or_toggle: SetOrToggle::Toggle.into(),
            })
            .block_on_tokio()
            .unwrap();
    }

    pub fn make(&self) -> String {
        self.make_async().block_on_tokio()
    }

    /// Get this output's make.
    ///
    ///
    ///
    /// Shorthand for `self.props().make`.
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

    pub fn model(&self) -> String {
        self.model_async().block_on_tokio()
    }

    /// Get this output's model.
    ///
    ///
    ///
    /// Shorthand for `self.props().make`.
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

    pub fn serial_async(&self) -> String {
        self.serial_async_async().block_on_tokio()
    }

    pub async fn serial_async_async(&self) -> String {
        Client::output()
            .get_info(GetInfoRequest {
                output_name: self.name(),
            })
            .await
            .unwrap()
            .into_inner()
            .serial
    }

    pub fn loc(&self) -> Option<Point> {
        self.loc_async().block_on_tokio()
    }

    /// Get this output's x position in the global space.
    ///
    ///
    ///
    /// Shorthand for `self.props().x`.
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

    pub fn logical_size(&self) -> Option<Size> {
        self.logical_size_async().block_on_tokio()
    }

    /// Get this output's logical width in pixels.
    ///
    /// If the output is disabled, this returns None.
    ///
    ///
    ///
    /// Shorthand for `self.props().logical_width`.
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

    pub fn current_mode(&self) -> Option<Mode> {
        self.current_mode_async().block_on_tokio()
    }

    /// Get this output's current mode.
    ///
    ///
    ///
    /// Shorthand for `self.props().current_mode`.
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

    pub fn preferred_mode(&self) -> Option<Mode> {
        self.preferred_mode_async().block_on_tokio()
    }

    /// Get this output's preferred mode.
    ///
    ///
    ///
    /// Shorthand for `self.props().preferred_mode`.
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

    pub fn modes(&self) -> impl Iterator<Item = Mode> {
        self.modes_async().block_on_tokio()
    }

    /// Get all available modes this output supports.
    ///
    ///
    ///
    /// Shorthand for `self.props().modes`.
    pub async fn modes_async(&self) -> impl Iterator<Item = Mode> {
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

    pub fn physical_size(&self) -> Size {
        self.physical_size_async().block_on_tokio()
    }

    /// Get this output's physical width in millimeters.
    ///
    ///
    ///
    /// Shorthand for `self.props().physical_width`.
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

    pub fn focused(&self) -> bool {
        self.focused_async().block_on_tokio()
    }

    /// Get whether this output is focused or not.
    ///
    /// This is currently implemented as the output with the most recent pointer motion.
    ///
    ///
    ///
    /// Shorthand for `self.props().focused`.
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

    pub fn tags(&self) -> impl Iterator<Item = TagHandle> {
        self.tags_async().block_on_tokio()
    }

    /// Get the tags this output has.
    ///
    ///
    ///
    /// Shorthand for `self.props().tags`
    pub async fn tags_async(&self) -> impl Iterator<Item = TagHandle> {
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

    pub fn scale(&self) -> f32 {
        self.scale_async().block_on_tokio()
    }

    /// Get this output's scaling factor.
    ///
    ///
    ///
    /// Shorthand for `self.props().scale`
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

    pub fn transform(&self) -> Transform {
        self.transform_async().block_on_tokio()
    }

    /// Get this output's transform.
    ///
    ///
    ///
    /// Shorthand for `self.props().transform`
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

    pub fn keyboard_focus_stack(&self) -> impl Iterator<Item = WindowHandle> {
        self.keyboard_focus_stack_async().block_on_tokio()
    }

    /// Get this output's keyboard focus stack.
    ///
    /// This will return the focus stack containing *all* windows on this output.
    /// If you only want windows on active tags, see
    /// [`OutputHandle::keyboard_focus_stack_visible`].
    ///
    ///
    ///
    /// Shorthand for `self.props().keyboard_focus_stack`
    pub async fn keyboard_focus_stack_async(&self) -> impl Iterator<Item = WindowHandle> {
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

    pub fn keyboard_focus_stack_visible(&self) -> impl Iterator<Item = WindowHandle> {
        self.keyboard_focus_stack_visible_async().block_on_tokio()
    }

    /// Get this output's keyboard focus stack with only visible windows.
    ///
    ///
    ///
    /// If you only want a focus stack containing all windows on this output, see
    /// [`OutputHandle::keyboard_focus_stack`].
    pub async fn keyboard_focus_stack_visible_async(&self) -> impl Iterator<Item = WindowHandle> {
        self.keyboard_focus_stack_async()
            .await
            .batch_filter(|win| win.is_on_active_tag_async().boxed(), |is_on| *is_on)
    }

    /// Get whether this output is enabled.
    ///
    ///
    pub fn enabled(&self) -> bool {
        self.enabled_async().block_on_tokio()
    }
    ///
    /// Disabled outputs act as if you unplugged them.
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

    /// Get whether this output is powered.
    ///
    /// Unpowered outputs will be turned off but you can still interact with them.
    ///
    /// Outputs can be disabled but still powered; this just means
    ///
    ///
    pub fn powered(&self) -> bool {
        self.powered_async().block_on_tokio()
    }
    /// they will turn on when powered. Disabled and unpowered outputs
    /// will not power on when enabled, but will still be interactable.
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

    /// Get this output's unique name (the name of its connector).
    pub fn name(&self) -> String {
        self.name.to_string()
    }
}

/// A possible output pixel dimension and refresh rate configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Mode {
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
