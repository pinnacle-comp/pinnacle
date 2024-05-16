// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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

use std::sync::OnceLock;

use futures::FutureExt;
use pinnacle_api_defs::pinnacle::{
    tag::{
        self,
        v0alpha1::{
            tag_service_client::TagServiceClient, AddRequest, RemoveRequest, SetActiveRequest,
            SwitchToRequest,
        },
    },
    v0alpha1::SetOrToggle,
};
use tonic::transport::Channel;

use crate::{
    block_on_tokio,
    output::OutputHandle,
    signal::{SignalHandle, TagSignal},
    util::Batch,
    window::WindowHandle,
    ApiModules,
};

/// A struct that allows you to add and remove tags and get [`TagHandle`]s.
#[derive(Clone, Debug)]
pub struct Tag {
    tag_client: TagServiceClient<Channel>,
    api: OnceLock<ApiModules>,
}

impl Tag {
    pub(crate) fn new(channel: Channel) -> Self {
        Self {
            tag_client: TagServiceClient::new(channel.clone()),
            api: OnceLock::new(),
        }
    }

    pub(crate) fn finish_init(&self, api: ApiModules) {
        self.api.set(api).unwrap();
    }

    pub(crate) fn new_handle(&self, id: u32) -> TagHandle {
        TagHandle {
            id,
            tag_client: self.tag_client.clone(),
            api: self.api.get().unwrap().clone(),
        }
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
    ) -> Vec<TagHandle> {
        block_on_tokio(self.add_async(output, tag_names))
    }

    /// The async version of [`Tag::add`].
    pub async fn add_async(
        &self,
        output: &OutputHandle,
        tag_names: impl IntoIterator<Item = impl Into<String>>,
    ) -> Vec<TagHandle> {
        let mut client = self.tag_client.clone();

        let tag_names = tag_names.into_iter().map(Into::into).collect();

        let response = client
            .add(AddRequest {
                output_name: Some(output.name.clone()),
                tag_names,
            })
            .await
            .unwrap()
            .into_inner();

        response
            .tag_ids
            .into_iter()
            .map(move |id| self.new_handle(id))
            .collect()
    }

    /// Get handles to all tags across all outputs.
    ///
    /// # Examples
    ///
    /// ```
    /// let all_tags = tag.get_all();
    /// ```
    pub fn get_all(&self) -> Vec<TagHandle> {
        block_on_tokio(self.get_all_async())
    }

    /// The async version of [`Tag::get_all`].
    pub async fn get_all_async(&self) -> Vec<TagHandle> {
        let mut client = self.tag_client.clone();

        let response = client
            .get(tag::v0alpha1::GetRequest {})
            .await
            .unwrap()
            .into_inner();

        response
            .tag_ids
            .into_iter()
            .map(move |id| self.new_handle(id))
            .collect()
    }

    /// Get a handle to the first tag with the given name on the focused output.
    ///
    /// If you need to get a tag on a specific output, see [`Tag::get_on_output`].
    ///
    /// # Examples
    ///
    /// ```
    /// // Get tag "Thing" on the focused output
    /// let tg = tag.get("Thing");
    /// ```
    pub fn get(&self, name: impl Into<String>) -> Option<TagHandle> {
        block_on_tokio(self.get_async(name))
    }

    /// The async version of [`Tag::get`].
    pub async fn get_async(&self, name: impl Into<String>) -> Option<TagHandle> {
        let name = name.into();
        let focused_output = self.api.get().unwrap().output.get_focused();

        if let Some(output) = focused_output {
            self.get_on_output_async(name, &output).await
        } else {
            None
        }
    }

    /// Get a handle to the first tag with the given name on the specified output.
    ///
    /// If you just need to get a tag on the focused output, see [`Tag::get`].
    ///
    /// # Examples
    ///
    /// ```
    /// // Get tag "Thing" on "HDMI-1"
    /// let tg = tag.get_on_output("Thing", output.get_by_name("HDMI-2")?);
    /// ```
    pub fn get_on_output(
        &self,
        name: impl Into<String>,
        output: &OutputHandle,
    ) -> Option<TagHandle> {
        block_on_tokio(self.get_on_output_async(name, output))
    }

    /// The async version of [`Tag::get_on_output`].
    pub async fn get_on_output_async(
        &self,
        name: impl Into<String>,
        output: &OutputHandle,
    ) -> Option<TagHandle> {
        let name = name.into();

        self.get_all_async().await.batch_find(
            |tag| tag.props_async().boxed(),
            |props| {
                let same_tag_name = props.name.as_ref() == Some(&name);
                let same_output = props.output.as_ref().is_some_and(|op| op == output);

                same_tag_name && same_output
            },
        )
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

        let mut client = self.tag_client.clone();

        block_on_tokio(client.remove(RemoveRequest { tag_ids })).unwrap();
    }

    /// Connect to a tag signal.
    ///
    /// The compositor will fire off signals that your config can listen for and act upon.
    /// You can pass in a [`TagSignal`] along with a callback and it will get run
    /// with the necessary arguments every time a signal of that type is received.
    pub fn connect_signal(&self, signal: TagSignal) -> SignalHandle {
        let mut signal_state = block_on_tokio(self.api.get().unwrap().signal.write());

        match signal {
            TagSignal::Active(f) => signal_state.tag_active.add_callback(f),
        }
    }
}

/// A handle to a tag.
///
/// This handle allows you to do things like switch to tags and get their properties.
#[derive(Debug, Clone)]
pub struct TagHandle {
    pub(crate) id: u32,
    tag_client: TagServiceClient<Channel>,
    api: ApiModules,
}

impl PartialEq for TagHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for TagHandle {}

impl std::hash::Hash for TagHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
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
        let mut client = self.tag_client.clone();
        block_on_tokio(client.switch_to(SwitchToRequest {
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
        let mut client = self.tag_client.clone();
        block_on_tokio(client.set_active(SetActiveRequest {
            tag_id: Some(self.id),
            set_or_toggle: Some(match set {
                true => SetOrToggle::Set,
                false => SetOrToggle::Unset,
            } as i32),
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
        let mut client = self.tag_client.clone();
        block_on_tokio(client.set_active(SetActiveRequest {
            tag_id: Some(self.id),
            set_or_toggle: Some(SetOrToggle::Toggle as i32),
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
    pub fn remove(&self) {
        let mut tag_client = self.tag_client.clone();
        block_on_tokio(tag_client.remove(RemoveRequest {
            tag_ids: vec![self.id],
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
        block_on_tokio(self.props_async())
    }

    /// The async version of [`TagHandle::props`].
    pub async fn props_async(&self) -> TagProperties {
        let mut client = self.tag_client.clone();

        let response = client
            .get_properties(tag::v0alpha1::GetPropertiesRequest {
                tag_id: Some(self.id),
            })
            .await
            .unwrap()
            .into_inner();

        let output = self.api.output;
        let window = self.api.window;

        TagProperties {
            active: response.active,
            name: response.name,
            output: response.output_name.map(|name| output.new_handle(name)),
            windows: response
                .window_ids
                .into_iter()
                .map(|id| window.new_handle(id))
                .collect(),
        }
    }

    /// Get this tag's active status.
    ///
    /// Shorthand for `self.props().active`.
    pub fn active(&self) -> Option<bool> {
        self.props().active
    }

    /// The async version of [`TagHandle::active`].
    pub async fn active_async(&self) -> Option<bool> {
        self.props_async().await.active
    }

    /// Get this tag's name.
    ///
    /// Shorthand for `self.props().name`.
    pub fn name(&self) -> Option<String> {
        self.props().name
    }

    /// The async version of [`TagHandle::name`].
    pub async fn name_async(&self) -> Option<String> {
        self.props_async().await.name
    }

    /// Get a handle to the output this tag is on.
    ///
    /// Shorthand for `self.props().output`.
    pub fn output(&self) -> Option<OutputHandle> {
        self.props().output
    }

    /// The async version of [`TagHandle::output`].
    pub async fn output_async(&self) -> Option<OutputHandle> {
        self.props_async().await.output
    }

    /// Get all windows with this tag.
    ///
    /// Shorthand for `self.props().windows`.
    pub fn windows(&self) -> Vec<WindowHandle> {
        self.props().windows
    }

    /// The async version of [`TagHandle::windows`].
    pub async fn windows_async(&self) -> Vec<WindowHandle> {
        self.props_async().await.windows
    }

    /// Get this tag's raw compositor id.
    pub fn id(&self) -> u32 {
        self.id
    }
}

/// Properties of a tag.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TagProperties {
    /// Whether the tag is active or not
    pub active: Option<bool>,
    /// The name of the tag
    pub name: Option<String>,
    /// The output the tag is on
    pub output: Option<OutputHandle>,
    /// The windows that have this tag
    pub windows: Vec<WindowHandle>,
}
