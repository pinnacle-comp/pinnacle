// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Output management.
//!
//! An output is Pinnacle's terminology for a monitor.
//!
//! This module provides [`Output`], which allows you to get [`OutputHandle`]s for different
//! connected monitors and set them up.

use futures::{channel::mpsc::UnboundedSender, future::BoxFuture, FutureExt, StreamExt};
use pinnacle_api_defs::pinnacle::{
    output::{
        self,
        v0alpha1::{
            output_service_client::OutputServiceClient, ConnectForAllRequest, SetLocationRequest,
        },
    },
    tag::v0alpha1::tag_service_client::TagServiceClient,
};
use tonic::transport::Channel;

use crate::{block_on_tokio, tag::TagHandle};

/// A struct that allows you to get handles to connected outputs and set them up.
///
/// See [`OutputHandle`] for more information.
#[derive(Debug, Clone)]
pub struct Output {
    channel: Channel,
    fut_sender: UnboundedSender<BoxFuture<'static, ()>>,
}

impl Output {
    pub(crate) fn new(
        channel: Channel,
        fut_sender: UnboundedSender<BoxFuture<'static, ()>>,
    ) -> Self {
        Self {
            channel,
            fut_sender,
        }
    }

    fn create_output_client(&self) -> OutputServiceClient<Channel> {
        OutputServiceClient::new(self.channel.clone())
    }

    fn create_tag_client(&self) -> TagServiceClient<Channel> {
        TagServiceClient::new(self.channel.clone())
    }

    /// Get a handle to all connected outputs.
    ///
    /// # Examples
    ///
    /// ```
    /// let outputs = output.get_all();
    /// ```
    pub fn get_all(&self) -> impl Iterator<Item = OutputHandle> {
        let mut client = self.create_output_client();
        let tag_client = self.create_tag_client();
        block_on_tokio(client.get(output::v0alpha1::GetRequest {}))
            .unwrap()
            .into_inner()
            .output_names
            .into_iter()
            .map(move |name| OutputHandle {
                client: client.clone(),
                tag_client: tag_client.clone(),
                name,
            })
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
        let name: String = name.into();
        self.get_all().find(|output| output.name == name)
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
            .find(|output| matches!(output.props().focused, Some(true)))
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
    pub fn connect_for_all(&self, mut for_all: impl FnMut(OutputHandle) + Send + 'static) {
        for output in self.get_all() {
            for_all(output);
        }

        let mut client = self.create_output_client();
        let tag_client = self.create_tag_client();

        self.fut_sender
            .unbounded_send(
                async move {
                    let mut stream = client
                        .connect_for_all(ConnectForAllRequest {})
                        .await
                        .unwrap()
                        .into_inner();

                    while let Some(Ok(response)) = stream.next().await {
                        let Some(output_name) = response.output_name else {
                            continue;
                        };

                        let output = OutputHandle {
                            client: client.clone(),
                            tag_client: tag_client.clone(),
                            name: output_name,
                        };

                        for_all(output);
                    }
                }
                .boxed(),
            )
            .unwrap();
    }
}

/// A handle to an output.
///
/// This allows you to manipulate outputs and get their properties.
#[derive(Clone, Debug)]
pub struct OutputHandle {
    pub(crate) client: OutputServiceClient<Channel>,
    pub(crate) tag_client: TagServiceClient<Channel>,
    pub(crate) name: String,
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
        let mut client = self.client.clone();
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
            let other_width = other_props.pixel_width? as i32;
            let other_height = other_props.pixel_height? as i32;

            let self_width = self_props.pixel_width? as i32;
            let self_height = self_props.pixel_height? as i32;

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

    /// Get all properties of this output.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::output::OutputProperties;
    ///
    /// let OutputProperties {
    ///     make,
    ///     model,
    ///     x,
    ///     y,
    ///     pixel_width,
    ///     pixel_height,
    ///     refresh_rate,
    ///     physical_width,
    ///     physical_height,
    ///     focused,
    ///     tags,
    /// } = output.get_focused()?.props();
    /// ```
    pub fn props(&self) -> OutputProperties {
        let mut client = self.client.clone();
        let response = block_on_tokio(client.get_properties(
            output::v0alpha1::GetPropertiesRequest {
                output_name: Some(self.name.clone()),
            },
        ))
        .unwrap()
        .into_inner();

        OutputProperties {
            make: response.make,
            model: response.model,
            x: response.x,
            y: response.y,
            pixel_width: response.pixel_width,
            pixel_height: response.pixel_height,
            refresh_rate: response.refresh_rate,
            physical_width: response.physical_width,
            physical_height: response.physical_height,
            focused: response.focused,
            tags: response
                .tag_ids
                .into_iter()
                .map(|id| TagHandle {
                    client: self.tag_client.clone(),
                    output_client: self.client.clone(),
                    id,
                })
                .collect(),
        }
    }

    // TODO: make a macro for the following or something

    /// Get this output's make.
    ///
    /// Shorthand for `self.props().make`.
    pub fn make(&self) -> Option<String> {
        self.props().make
    }

    /// Get this output's model.
    ///
    /// Shorthand for `self.props().make`.
    pub fn model(&self) -> Option<String> {
        self.props().model
    }

    /// Get this output's x position in the global space.
    ///
    /// Shorthand for `self.props().x`.
    pub fn x(&self) -> Option<i32> {
        self.props().x
    }

    /// Get this output's y position in the global space.
    ///
    /// Shorthand for `self.props().y`.
    pub fn y(&self) -> Option<i32> {
        self.props().y
    }

    /// Get this output's screen width in pixels.
    ///
    /// Shorthand for `self.props().pixel_width`.
    pub fn pixel_width(&self) -> Option<u32> {
        self.props().pixel_width
    }

    /// Get this output's screen height in pixels.
    ///
    /// Shorthand for `self.props().pixel_height`.
    pub fn pixel_height(&self) -> Option<u32> {
        self.props().pixel_height
    }

    /// Get this output's refresh rate in millihertz.
    ///
    /// For example, 144Hz will be returned as 144000.
    ///
    /// Shorthand for `self.props().refresh_rate`.
    pub fn refresh_rate(&self) -> Option<u32> {
        self.props().refresh_rate
    }

    /// Get this output's physical width in millimeters.
    ///
    /// Shorthand for `self.props().physical_width`.
    pub fn physical_width(&self) -> Option<u32> {
        self.props().physical_width
    }

    /// Get this output's physical height in millimeters.
    ///
    /// Shorthand for `self.props().physical_height`.
    pub fn physical_height(&self) -> Option<u32> {
        self.props().physical_height
    }

    /// Get whether this output is focused or not.
    ///
    /// This is currently implemented as the output with the most recent pointer motion.
    ///
    /// Shorthand for `self.props().focused`.
    pub fn focused(&self) -> Option<bool> {
        self.props().focused
    }

    /// Get the tags this output has.
    ///
    /// Shorthand for `self.props().tags`
    pub fn tags(&self) -> Vec<TagHandle> {
        self.props().tags
    }

    /// Get this output's unique name (the name of its connector).
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// The properties of an output.
#[derive(Clone, Debug)]
pub struct OutputProperties {
    /// The make of the output
    pub make: Option<String>,
    /// The model of the output
    ///
    /// This is something like "27GL83A" or whatever crap monitor manufacturers name their monitors
    /// these days.
    pub model: Option<String>,
    /// The x position of the output in the global space
    pub x: Option<i32>,
    /// The y position of the output in the global space
    pub y: Option<i32>,
    /// The output's screen width in pixels
    pub pixel_width: Option<u32>,
    /// The output's screen height in pixels
    pub pixel_height: Option<u32>,
    /// The output's refresh rate in millihertz
    pub refresh_rate: Option<u32>,
    /// The output's physical width in millimeters
    pub physical_width: Option<u32>,
    /// The output's physical height in millimeters
    pub physical_height: Option<u32>,
    /// Whether this output is focused or not
    ///
    /// This is currently implemented as the output with the most recent pointer motion.
    pub focused: Option<bool>,
    /// The tags this output has
    pub tags: Vec<TagHandle>,
}
