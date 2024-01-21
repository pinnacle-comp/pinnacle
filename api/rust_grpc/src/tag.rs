//! Tag management.
//!
//! This module allows you to interact with Pinnacle's tag system.
//!
//! # The Tag System
//! Many Wayland compositors use workspaces for window management.
//! Each window is assigned to a workspace and will only show up if that workspace is being
//! viewed. This is a find way to manage windows, but it's not that powerful.
//!
//! Instead, Pinnacle works with a tag system similar to window managers like [dwm](https://dwm.suckless.org/)
//! and, the window manager Pinnacle takes inspiration from, [awesome](https://awesomewm.org/).
//!
//! In a tag system, there are no workspaces. Instead, each window can be tagged with zero or more
//! tags, and zero or more tags can be displayed on a monitor at once. This allows you to, for
//! example, bring in your browsers on the same screen as your IDE by toggling the "Browser" tag.
//!
//! Workspaces can be emulated by only displaying one tag at a time. Combining this feature with
//! the ability to tag windows with multiple tags allows you to have one window show up on multiple
//! different "workspaces". As you can see, this system is much more powerful than workspaces
//! alone.
//!
//! # Configuration
//! `tag` contains the [`Tag`] struct, which allows you to add new tags
//! and get handles to already defined ones.
//!
//! These [`TagHandle`]s allow you to manipulate individual tags and get their properties.

use futures::{channel::mpsc::UnboundedSender, executor::block_on, future::BoxFuture};
use num_enum::TryFromPrimitive;
use pinnacle_api_defs::pinnacle::{
    output::v0alpha1::output_service_client::OutputServiceClient,
    tag::{
        self,
        v0alpha1::{
            tag_service_client::TagServiceClient, AddRequest, RemoveRequest, SetActiveRequest,
            SetLayoutRequest, SwitchToRequest,
        },
    },
};
use tonic::transport::Channel;

use crate::output::{Output, OutputHandle};

/// A struct that allows you to add and remove tags and get [`TagHandle`]s.
#[derive(Clone, Debug)]
pub struct Tag {
    channel: Channel,
    fut_sender: UnboundedSender<BoxFuture<'static, ()>>,
}

impl Tag {
    pub(crate) fn new(
        channel: Channel,
        fut_sender: UnboundedSender<BoxFuture<'static, ()>>,
    ) -> Self {
        Self {
            channel,
            fut_sender,
        }
    }

    fn create_tag_client(&self) -> TagServiceClient<Channel> {
        TagServiceClient::new(self.channel.clone())
    }

    fn create_output_client(&self) -> OutputServiceClient<Channel> {
        OutputServiceClient::new(self.channel.clone())
    }

    /// Add tags to the specified output.
    ///
    /// This will add tags with the given names to `output` and return [`TagHandle`]s to all of
    /// them.
    ///
    /// # Examples
    ///
    /// ```
    /// // Add tags 1-5 to the focused output
    /// if let Some(op) = output.get_focused() {
    ///     let tags = tag.add(&op, ["1", "2", "3", "4", "5"]);
    /// }
    /// ```
    pub fn add(
        &self,
        output: &OutputHandle,
        tag_names: impl IntoIterator<Item = impl Into<String>>,
    ) -> impl Iterator<Item = TagHandle> {
        let mut client = self.create_tag_client();
        let output_client = self.create_output_client();

        let tag_names = tag_names.into_iter().map(Into::into).collect();

        let response = block_on(client.add(AddRequest {
            output_name: Some(output.name.clone()),
            tag_names,
        }))
        .unwrap()
        .into_inner();

        response.tag_ids.into_iter().map(move |id| TagHandle {
            client: client.clone(),
            output_client: output_client.clone(),
            id,
        })
    }

    /// Get handles to all tags across all outputs.
    ///
    /// # Examples
    ///
    /// ```
    /// let all_tags = tag.get_all();
    /// ```
    pub fn get_all(&self) -> impl Iterator<Item = TagHandle> {
        let mut client = self.create_tag_client();
        let output_client = self.create_output_client();

        let response = block_on(client.get(tag::v0alpha1::GetRequest {}))
            .unwrap()
            .into_inner();

        response.tag_ids.into_iter().map(move |id| TagHandle {
            client: client.clone(),
            output_client: output_client.clone(),
            id,
        })
    }

    /// Get a handle to the first tag with the given name on `output`.
    ///
    /// If `output` is `None`, the focused output will be used.
    ///
    /// # Examples
    ///
    /// ```
    /// // Get tag "1" on output "HDMI-1"
    /// if let Some(op) = output.get_by_name("HDMI-1") {
    ///     let tg = tag.get("1", &op);
    /// }
    ///
    /// // Get tag "Thing" on the focused output
    /// let tg = tag.get("Thing", None);
    /// ```
    pub fn get<'a>(
        &self,
        name: impl Into<String>,
        output: impl Into<Option<&'a OutputHandle>>,
    ) -> Option<TagHandle> {
        let name = name.into();
        let output: Option<&OutputHandle> = output.into();
        let output_module = Output::new(self.channel.clone(), self.fut_sender.clone());

        self.get_all().find(|tag| {
            let props = tag.props();

            let same_tag_name = props.name.as_ref() == Some(&name);
            let same_output = props.output.is_some_and(|op| {
                Some(op.name)
                    == output
                        .map(|o| o.name.clone())
                        .or_else(|| output_module.get_focused().map(|o| o.name))
            });

            same_tag_name && same_output
        })
    }

    /// Remove the given tags from their outputs.
    ///
    /// # Examples
    ///
    /// ```
    /// let tags = tag.add(output.get_by_name("DP-1")?, ["1", "2", "Buckle", "Shoe"]);
    ///
    /// tag.remove(tags); // "DP-1" no longer has any tags
    /// ```
    pub fn remove(&self, tags: impl IntoIterator<Item = TagHandle>) {
        let tag_ids = tags.into_iter().map(|handle| handle.id).collect::<Vec<_>>();

        let mut client = self.create_tag_client();

        block_on(client.remove(RemoveRequest { tag_ids })).unwrap();
    }
}

/// A handle to a tag.
///
/// This handle allows you to do things like switch to tags and get their properties.
#[derive(Debug, Clone)]
pub struct TagHandle {
    pub(crate) client: TagServiceClient<Channel>,
    pub(crate) output_client: OutputServiceClient<Channel>,
    pub(crate) id: u32,
}

/// Various static layouts.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, TryFromPrimitive)]
pub enum Layout {
    /// One master window on the left with all other windows stacked to the right
    MasterStack = 1,
    /// Windows split in half towards the bottom right corner
    Dwindle,
    /// Windows split in half in a spiral
    Spiral,
    /// One main corner window in the top left with a column of windows on the right and a row on the bottom
    CornerTopLeft,
    /// One main corner window in the top right with a column of windows on the left and a row on the bottom
    CornerTopRight,
    /// One main corner window in the bottom left with a column of windows on the right and a row on the top.
    CornerBottomLeft,
    /// One main corner window in the bottom right with a column of windows on the left and a row on the top.
    CornerBottomRight,
}

impl TagHandle {
    /// Activate this tag and deactivate all other ones on the same output.
    ///
    /// This essentially emulates what a traditional workspace is.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assume the focused output has the following inactive tags and windows:
    /// // "1": Alacritty
    /// // "2": Firefox, Discord
    /// // "3": Steam
    /// tag.get("2")?.switch_to(); // Displays Firefox and Discord
    /// tag.get("3")?.switch_to(); // Displays Steam
    /// ```
    pub fn switch_to(&self) {
        let mut client = self.client.clone();
        block_on(client.switch_to(SwitchToRequest {
            tag_id: Some(self.id),
        }))
        .unwrap();
    }

    /// Set this tag to active or not.
    ///
    /// While active, windows with this tag will be displayed.
    ///
    /// While inactive, windows with this tag will not be displayed unless they have other active
    /// tags.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assume the focused output has the following inactive tags and windows:
    /// // "1": Alacritty
    /// // "2": Firefox, Discord
    /// // "3": Steam
    /// tag.get("2")?.set_active(true);  // Displays Firefox and Discord
    /// tag.get("3")?.set_active(true);  // Displays Firefox, Discord, and Steam
    /// tag.get("2")?.set_active(false); // Displays Steam
    /// ```
    pub fn set_active(&self, set: bool) {
        let mut client = self.client.clone();
        block_on(client.set_active(SetActiveRequest {
            tag_id: Some(self.id),
            set_or_toggle: Some(tag::v0alpha1::set_active_request::SetOrToggle::Set(set)),
        }))
        .unwrap();
    }

    /// Toggle this tag between active and inactive.
    ///
    /// While active, windows with this tag will be displayed.
    ///
    /// While inactive, windows with this tag will not be displayed unless they have other active
    /// tags.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assume the focused output has the following inactive tags and windows:
    /// // "1": Alacritty
    /// // "2": Firefox, Discord
    /// // "3": Steam
    /// tag.get("2")?.toggle(); // Displays Firefox and Discord
    /// tag.get("3")?.toggle(); // Displays Firefox, Discord, and Steam
    /// tag.get("3")?.toggle(); // Displays Firefox, Discord
    /// tag.get("2")?.toggle(); // Displays nothing
    /// ```
    pub fn toggle_active(&self) {
        let mut client = self.client.clone();
        block_on(client.set_active(SetActiveRequest {
            tag_id: Some(self.id),
            set_or_toggle: Some(tag::v0alpha1::set_active_request::SetOrToggle::Toggle(())),
        }))
        .unwrap();
    }

    /// Remove this tag from its output.
    ///
    /// # Examples
    ///
    /// ```
    /// let tags = tag
    ///     .add(output.get_by_name("DP-1")?, ["1", "2", "Buckle", "Shoe"])
    ///     .collect::<Vec<_>>;
    ///
    /// tags[1].remove();
    /// tags[3].remove();
    /// // "DP-1" now only has tags "1" and "Buckle"
    /// ```
    pub fn remove(mut self) {
        block_on(self.client.remove(RemoveRequest {
            tag_ids: vec![self.id],
        }))
        .unwrap();
    }

    /// Set this tag's layout.
    ///
    /// Layouting only applies to tiled windows (windows that are not floating, maximized, or
    /// fullscreen). If multiple tags are active on an output, the first active tag's layout will
    /// determine the layout strategy.
    ///
    /// See [`Layout`] for the different static layouts Pinnacle currently has to offer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::tag::Layout;
    ///
    /// // Set the layout of tag "1" on the focused output to "corner top left".
    /// tag.get("1", None)?.set_layout(Layout::CornerTopLeft);
    /// ```
    pub fn set_layout(&self, layout: Layout) {
        let mut client = self.client.clone();
        block_on(client.set_layout(SetLayoutRequest {
            tag_id: Some(self.id),
            layout: Some(layout as i32),
        }))
        .unwrap();
    }

    /// Get all properties of this tag.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::tag::TagProperties;
    ///
    /// let TagProperties {
    ///     active,
    ///     name,
    ///     output,
    /// } = tag.get("1", None)?.props();
    /// ```
    pub fn props(&self) -> TagProperties {
        let mut client = self.client.clone();
        let output_client = self.output_client.clone();

        let response = block_on(client.get_properties(tag::v0alpha1::GetPropertiesRequest {
            tag_id: Some(self.id),
        }))
        .unwrap()
        .into_inner();

        TagProperties {
            active: response.active,
            name: response.name,
            output: response.output_name.map(|name| OutputHandle {
                client: output_client,
                tag_client: client,
                name,
            }),
        }
    }

    /// Get this tag's active status.
    ///
    /// Shorthand for `self.props().active`.
    pub fn active(&self) -> Option<bool> {
        self.props().active
    }

    /// Get this tag's name.
    ///
    /// Shorthand for `self.props().name`.
    pub fn name(&self) -> Option<String> {
        self.props().name
    }

    /// Get a handle to the output this tag is on.
    ///
    /// Shorthand for `self.props().output`.
    pub fn output(&self) -> Option<OutputHandle> {
        self.props().output
    }
}

/// Properties of a tag.
pub struct TagProperties {
    /// Whether the tag is active or not
    pub active: Option<bool>,
    /// The name of the tag
    pub name: Option<String>,
    /// The output the tag is on
    pub output: Option<OutputHandle>,
}
