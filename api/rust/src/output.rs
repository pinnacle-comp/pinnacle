// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Output management.
//!
//! An output is Pinnacle's terminology for a monitor.
//!
//! This module provides [`Output`], which allows you to get [`OutputHandle`]s for different
//! connected monitors and set them up.

use futures::FutureExt;
use pinnacle_api_defs::pinnacle::output::{
    self,
    v0alpha1::{
        output_service_client::OutputServiceClient, set_scale_request::AbsoluteOrRelative,
        SetLocationRequest, SetModeRequest, SetScaleRequest,
    },
};
use tonic::transport::Channel;

use crate::{
    block_on_tokio,
    signal::{OutputSignal, SignalHandle},
    tag::TagHandle,
    util::Batch,
    SIGNAL, TAG,
};

/// A struct that allows you to get handles to connected outputs and set them up.
///
/// See [`OutputHandle`] for more information.
#[derive(Debug, Clone)]
pub struct Output {
    output_client: OutputServiceClient<Channel>,
}

impl Output {
    pub(crate) fn new(channel: Channel) -> Self {
        Self {
            output_client: OutputServiceClient::new(channel.clone()),
        }
    }

    pub(crate) fn new_handle(&self, name: impl Into<String>) -> OutputHandle {
        OutputHandle {
            name: name.into(),
            output_client: self.output_client.clone(),
        }
    }

    /// Get a handle to all connected outputs.
    ///
    /// # Examples
    ///
    /// ```
    /// let outputs = output.get_all();
    /// ```
    pub fn get_all(&self) -> Vec<OutputHandle> {
        block_on_tokio(self.get_all_async())
    }

    /// The async version of [`Output::get_all`].
    pub async fn get_all_async(&self) -> Vec<OutputHandle> {
        let mut client = self.output_client.clone();

        client
            .get(output::v0alpha1::GetRequest {})
            .await
            .unwrap()
            .into_inner()
            .output_names
            .into_iter()
            .map(move |name| self.new_handle(name))
            .collect()
    }

    /// Get a handle to the output with the given name.
    ///
    /// By "name", we mean the name of the connector the output is connected to.
    ///
    /// # Examples
    ///
    /// ```
    /// let op = output.get_by_name("eDP-1")?;
    /// let op2 = output.get_by_name("HDMI-2")?;
    /// ```
    pub fn get_by_name(&self, name: impl Into<String>) -> Option<OutputHandle> {
        block_on_tokio(self.get_by_name_async(name))
    }

    /// The async version of [`Output::get_by_name`].
    pub async fn get_by_name_async(&self, name: impl Into<String>) -> Option<OutputHandle> {
        let name: String = name.into();
        self.get_all_async()
            .await
            .into_iter()
            .find(|output| output.name == name)
    }

    /// Get a handle to the focused output.
    ///
    /// This is currently implemented as the one that has had the most recent pointer movement.
    ///
    /// # Examples
    ///
    /// ```
    /// let op = output.get_focused()?;
    /// ```
    pub fn get_focused(&self) -> Option<OutputHandle> {
        self.get_all()
            .into_iter()
            .find(|output| matches!(output.props().focused, Some(true)))
    }

    /// The async version of [`Output::get_focused`].
    pub async fn get_focused_async(&self) -> Option<OutputHandle> {
        self.get_all_async().await.batch_find(
            |output| output.props_async().boxed(),
            |props| props.focused.is_some_and(|focused| focused),
        )
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
    ///     tags.next().unwrap().set_active(true);
    /// });
    /// ```
    pub fn connect_for_all(&self, mut for_all: impl FnMut(&OutputHandle) + Send + 'static) {
        for output in self.get_all() {
            for_all(&output);
        }

        let mut signal_state = block_on_tokio(SIGNAL.get().expect("SIGNAL doesn't exist").write());
        signal_state.output_connect.add_callback(Box::new(for_all));
    }

    /// Connect to an output signal.
    ///
    /// The compositor will fire off signals that your config can listen for and act upon.
    /// You can pass in an [`OutputSignal`] along with a callback and it will get run
    /// with the necessary arguments every time a signal of that type is received.
    pub fn connect_signal(&self, signal: OutputSignal) -> SignalHandle {
        let mut signal_state = block_on_tokio(SIGNAL.get().expect("SIGNAL doesn't exist").write());

        match signal {
            OutputSignal::Connect(f) => signal_state.output_connect.add_callback(f),
        }
    }
}

/// A handle to an output.
///
/// This allows you to manipulate outputs and get their properties.
#[derive(Clone, Debug)]
pub struct OutputHandle {
    pub(crate) name: String,
    output_client: OutputServiceClient<Channel>,
}

impl PartialEq for OutputHandle {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for OutputHandle {}

impl std::hash::Hash for OutputHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
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
    pub fn set_location(&self, x: impl Into<Option<i32>>, y: impl Into<Option<i32>>) {
        let mut client = self.output_client.clone();
        block_on_tokio(client.set_location(SetLocationRequest {
            output_name: Some(self.name.clone()),
            x: x.into(),
            y: y.into(),
        }))
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
        let self_props = self.props();
        let other_props = other.props();

        // poor man's try {}
        let attempt_set_loc = || -> Option<()> {
            let other_x = other_props.x?;
            let other_y = other_props.y?;
            let other_width = other_props.logical_width? as i32;
            let other_height = other_props.logical_height? as i32;

            let self_width = self_props.logical_width? as i32;
            let self_height = self_props.logical_height? as i32;

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

            self.set_location(Some(x), Some(y));

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
    pub fn set_mode(
        &self,
        pixel_width: u32,
        pixel_height: u32,
        refresh_rate_millihertz: impl Into<Option<u32>>,
    ) {
        let mut client = self.output_client.clone();
        block_on_tokio(client.set_mode(SetModeRequest {
            output_name: Some(self.name.clone()),
            pixel_width: Some(pixel_width),
            pixel_height: Some(pixel_height),
            refresh_rate_millihz: refresh_rate_millihertz.into(),
        }))
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
        let mut client = self.output_client.clone();
        block_on_tokio(client.set_scale(SetScaleRequest {
            output_name: Some(self.name.clone()),
            absolute_or_relative: Some(AbsoluteOrRelative::Absolute(scale)),
        }))
        .unwrap();
    }

    /// Increase this output's scaling factor by `increase_by`.
    ///
    /// # Examples
    ///
    /// ```
    /// output.get_focused()?.increase_scale(0.25);
    /// ```
    pub fn increase_scale(&self, increase_by: f32) {
        let mut client = self.output_client.clone();
        block_on_tokio(client.set_scale(SetScaleRequest {
            output_name: Some(self.name.clone()),
            absolute_or_relative: Some(AbsoluteOrRelative::Relative(increase_by)),
        }))
        .unwrap();
    }

    /// Decrease this output's scaling factor by `decrease_by`.
    ///
    /// This simply calls [`OutputHandle::increase_scale`] with the negative of `decrease_by`.
    ///
    /// # Examples
    ///
    /// ```
    /// output.get_focused()?.decrease_scale(0.25);
    /// ```
    pub fn decrease_scale(&self, decrease_by: f32) {
        self.increase_scale(-decrease_by);
    }

    /// Get all properties of this output.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::output::OutputProperties;
    ///
    /// let OutputProperties {
    ///     ..
    /// } = output.get_focused()?.props();
    /// ```
    pub fn props(&self) -> OutputProperties {
        block_on_tokio(self.props_async())
    }

    /// The async version of [`OutputHandle::props`].
    pub async fn props_async(&self) -> OutputProperties {
        let mut client = self.output_client.clone();
        let response = client
            .get_properties(output::v0alpha1::GetPropertiesRequest {
                output_name: Some(self.name.clone()),
            })
            .await
            .unwrap()
            .into_inner();

        let tag = TAG.get().expect("TAG doesn't exist");

        OutputProperties {
            make: response.make,
            model: response.model,
            x: response.x,
            y: response.y,
            logical_width: response.logical_width,
            logical_height: response.logical_height,
            current_mode: response.current_mode.and_then(|mode| {
                Some(Mode {
                    pixel_width: mode.pixel_width?,
                    pixel_height: mode.pixel_height?,
                    refresh_rate_millihertz: mode.refresh_rate_millihz?,
                })
            }),
            preferred_mode: response.preferred_mode.and_then(|mode| {
                Some(Mode {
                    pixel_width: mode.pixel_width?,
                    pixel_height: mode.pixel_height?,
                    refresh_rate_millihertz: mode.refresh_rate_millihz?,
                })
            }),
            modes: response
                .modes
                .into_iter()
                .flat_map(|mode| {
                    Some(Mode {
                        pixel_width: mode.pixel_width?,
                        pixel_height: mode.pixel_height?,
                        refresh_rate_millihertz: mode.refresh_rate_millihz?,
                    })
                })
                .collect(),
            physical_width: response.physical_width,
            physical_height: response.physical_height,
            focused: response.focused,
            tags: response
                .tag_ids
                .into_iter()
                .map(|id| tag.new_handle(id))
                .collect(),
            scale: response.scale,
        }
    }

    // TODO: make a macro for the following or something

    /// Get this output's make.
    ///
    /// Shorthand for `self.props().make`.
    pub fn make(&self) -> Option<String> {
        self.props().make
    }

    /// The async version of [`OutputHandle::make`].
    pub async fn make_async(&self) -> Option<String> {
        self.props_async().await.make
    }

    /// Get this output's model.
    ///
    /// Shorthand for `self.props().make`.
    pub fn model(&self) -> Option<String> {
        self.props().model
    }

    /// The async version of [`OutputHandle::model`].
    pub async fn model_async(&self) -> Option<String> {
        self.props_async().await.model
    }

    /// Get this output's x position in the global space.
    ///
    /// Shorthand for `self.props().x`.
    pub fn x(&self) -> Option<i32> {
        self.props().x
    }

    /// The async version of [`OutputHandle::x`].
    pub async fn x_async(&self) -> Option<i32> {
        self.props_async().await.x
    }

    /// Get this output's y position in the global space.
    ///
    /// Shorthand for `self.props().y`.
    pub fn y(&self) -> Option<i32> {
        self.props().y
    }

    /// The async version of [`OutputHandle::y`].
    pub async fn y_async(&self) -> Option<i32> {
        self.props_async().await.y
    }

    /// Get this output's logical width in pixels.
    ///
    /// Shorthand for `self.props().logical_width`.
    pub fn logical_width(&self) -> Option<u32> {
        self.props().logical_width
    }

    /// The async version of [`OutputHandle::logical_width`].
    pub async fn logical_width_async(&self) -> Option<u32> {
        self.props_async().await.logical_width
    }

    /// Get this output's logical height in pixels.
    ///
    /// Shorthand for `self.props().logical_height`.
    pub fn logical_height(&self) -> Option<u32> {
        self.props().logical_height
    }

    /// The async version of [`OutputHandle::logical_height`].
    pub async fn logical_height_async(&self) -> Option<u32> {
        self.props_async().await.logical_height
    }

    /// Get this output's current mode.
    ///
    /// Shorthand for `self.props().current_mode`.
    pub fn current_mode(&self) -> Option<Mode> {
        self.props().current_mode
    }

    /// The async version of [`OutputHandle::current_mode`].
    pub async fn current_mode_async(&self) -> Option<Mode> {
        self.props_async().await.current_mode
    }

    /// Get this output's preferred mode.
    ///
    /// Shorthand for `self.props().preferred_mode`.
    pub fn preferred_mode(&self) -> Option<Mode> {
        self.props().preferred_mode
    }

    /// The async version of [`OutputHandle::preferred_mode`].
    pub async fn preferred_mode_async(&self) -> Option<Mode> {
        self.props_async().await.preferred_mode
    }

    /// Get all available modes this output supports.
    ///
    /// Shorthand for `self.props().modes`.
    pub fn modes(&self) -> Vec<Mode> {
        self.props().modes
    }

    /// The async version of [`OutputHandle::modes`].
    pub async fn modes_async(&self) -> Vec<Mode> {
        self.props_async().await.modes
    }

    /// Get this output's physical width in millimeters.
    ///
    /// Shorthand for `self.props().physical_width`.
    pub fn physical_width(&self) -> Option<u32> {
        self.props().physical_width
    }

    /// The async version of [`OutputHandle::physical_width`].
    pub async fn physical_width_async(&self) -> Option<u32> {
        self.props_async().await.physical_width
    }

    /// Get this output's physical height in millimeters.
    ///
    /// Shorthand for `self.props().physical_height`.
    pub fn physical_height(&self) -> Option<u32> {
        self.props().physical_height
    }

    /// The async version of [`OutputHandle::physical_height`].
    pub async fn physical_height_async(&self) -> Option<u32> {
        self.props_async().await.physical_height
    }

    /// Get whether this output is focused or not.
    ///
    /// This is currently implemented as the output with the most recent pointer motion.
    ///
    /// Shorthand for `self.props().focused`.
    pub fn focused(&self) -> Option<bool> {
        self.props().focused
    }

    /// The async version of [`OutputHandle::focused`].
    pub async fn focused_async(&self) -> Option<bool> {
        self.props_async().await.focused
    }

    /// Get the tags this output has.
    ///
    /// Shorthand for `self.props().tags`
    pub fn tags(&self) -> Vec<TagHandle> {
        self.props().tags
    }

    /// The async version of [`OutputHandle::tags`].
    pub async fn tags_async(&self) -> Vec<TagHandle> {
        self.props_async().await.tags
    }

    /// Get this output's scaling factor.
    ///
    /// Shorthand for `self.props().scale`
    pub fn scale(&self) -> Option<f32> {
        self.props().scale
    }

    /// The async version of [`OutputHandle::scale`].
    pub async fn scale_async(&self) -> Option<f32> {
        self.props_async().await.scale
    }

    /// Get this output's unique name (the name of its connector).
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A possible output pixel dimension and refresh rate configuration.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Mode {
    /// The width of the output, in pixels.
    pub pixel_width: u32,
    /// The height of the output, in pixels.
    pub pixel_height: u32,
    /// The output's refresh rate, in millihertz.
    ///
    /// For example, 60Hz is returned as 60000.
    pub refresh_rate_millihertz: u32,
}

/// The properties of an output.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OutputProperties {
    /// The make of the output.
    pub make: Option<String>,
    /// The model of the output.
    ///
    /// This is something like "27GL83A" or whatever crap monitor manufacturers name their monitors
    /// these days.
    pub model: Option<String>,
    /// The x position of the output in the global space.
    pub x: Option<i32>,
    /// The y position of the output in the global space.
    pub y: Option<i32>,
    /// The logical width of this output in the global space
    /// taking into account scaling, in pixels.
    pub logical_width: Option<u32>,
    /// The logical height of this output in the global space
    /// taking into account scaling, in pixels.
    pub logical_height: Option<u32>,
    /// The output's current mode.
    pub current_mode: Option<Mode>,
    /// The output's preferred mode.
    pub preferred_mode: Option<Mode>,
    /// All available modes the output supports.
    pub modes: Vec<Mode>,
    /// The output's physical width in millimeters.
    pub physical_width: Option<u32>,
    /// The output's physical height in millimeters.
    pub physical_height: Option<u32>,
    /// Whether this output is focused or not.
    ///
    /// This is currently implemented as the output with the most recent pointer motion.
    pub focused: Option<bool>,
    /// The tags this output has.
    pub tags: Vec<TagHandle>,
    /// This output's scaling factor.
    pub scale: Option<f32>,
}
