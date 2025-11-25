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

use futures::FutureExt;
use pinnacle_api_defs::pinnacle::{
    tag::v1::{
        AddRequest, GetActiveRequest, GetNameRequest, GetOutputNameRequest, GetRequest,
        RemoveRequest, SetActiveRequest, MoveToOutputRequest, SwitchToRequest,
    },
    util::v1::SetOrToggle,
};

use crate::{
    BlockOnTokio,
    client::Client,
    output::OutputHandle,
    signal::{SignalHandle, TagSignal},
    util::Batch,
    window::WindowHandle,
};

/// Adds tags to the specified output.
///
/// This will add tags with the given names to `output` and return [`TagHandle`]s to all of
/// them.
///
/// # Examples
///
/// ```no_run
/// # use pinnacle_api::output;
/// # use pinnacle_api::tag;
/// // Add tags 1-5 to the focused output
/// if let Some(op) = output::get_focused() {
///     let tags = tag::add(&op, ["1", "2", "3", "4", "5"]);
/// }
/// ```
pub fn add<I, T>(output: &OutputHandle, tag_names: I) -> impl Iterator<Item = TagHandle> + use<I, T>
where
    I: IntoIterator<Item = T>,
    T: ToString,
{
    let output_name = output.name();
    let tag_names = tag_names.into_iter().map(|name| name.to_string()).collect();

    Client::tag()
        .add(AddRequest {
            output_name,
            tag_names,
        })
        .block_on_tokio()
        .unwrap()
        .into_inner()
        .tag_ids
        .into_iter()
        .map(|id| TagHandle { id })
}

/// Move existing tags to the specified output.
///
/// # Examples
///
/// ```no_run
/// # use pinnacle_api::output;
/// # use pinnacle_api::tag;
/// let output = output::get_by_name("eDP-1")?;
/// let tag_to_move = tag::get("1")?;
/// tag::move_to_output(&output, tag_to_move);
/// ```
pub fn move_to_output<I>(output: &OutputHandle, tag_handles: I)
where
    I: IntoIterator<Item = TagHandle>,
{
    let output_name = output.name();
    let tag_ids = tag_handles.into_iter().map(|h| h.id).collect();

    Client::tag()
        .move_to_output(MoveToOutputRequest {
            output_name,
            tag_ids,
        })
        .block_on_tokio()
        .unwrap();
}

/// Gets handles to all tags across all outputs.
///
/// # Examples
///
/// ```no_run
/// # use pinnacle_api::tag;
/// for tag in tag::get_all() {
///     println!("{}", tag.name());
/// }
/// ```
pub fn get_all() -> impl Iterator<Item = TagHandle> {
    get_all_async().block_on_tokio()
}

/// Async impl for [`get_all_async`].
pub async fn get_all_async() -> impl Iterator<Item = TagHandle> {
    Client::tag()
        .get(GetRequest {})
        .await
        .unwrap()
        .into_inner()
        .tag_ids
        .into_iter()
        .map(|id| TagHandle { id })
}

/// Gets a handle to the first tag with the given `name` on the focused output.
///
/// To get the first tag with the given `name` on a specific output, see
/// [`get_on_output`].
///
/// # Examples
///
/// ```no_run
/// # use pinnacle_api::tag;
/// # || {
/// let tag = tag::get("2")?;
/// # Some(())
/// # };
/// ```
pub fn get(name: impl ToString) -> Option<TagHandle> {
    get_async(name).block_on_tokio()
}

/// Async impl for [`get`].
pub async fn get_async(name: impl ToString) -> Option<TagHandle> {
    let name = name.to_string();
    let focused_op = crate::output::get_focused_async().await?;

    get_on_output_async(name, &focused_op).await
}

/// Gets a handle to the first tag with the given `name` on `output`.
///
/// For a simpler way to get a tag on the focused output, see [`get`].
///
/// # Examples
///
/// ```no_run
/// # use pinnacle_api::output;
/// # use pinnacle_api::tag;
/// # || {
/// let output = output::get_by_name("eDP-1")?;
/// let tag = tag::get_on_output("2", &output)?;
/// # Some(())
/// # };
/// ```
pub fn get_on_output(name: impl ToString, output: &OutputHandle) -> Option<TagHandle> {
    get_on_output_async(name, output).block_on_tokio()
}

/// Async impl for [`get_on_output`].
pub async fn get_on_output_async(name: impl ToString, output: &OutputHandle) -> Option<TagHandle> {
    let name = name.to_string();
    let output = output.clone();
    get_all_async().await.batch_find(
        |tag| async { (tag.name_async().await, tag.output_async().await) }.boxed(),
        |(n, op)| *n == name && *op == output,
    )
}

/// Removes the given tags from their outputs.
///
/// # Examples
///
/// ```no_run
/// # use pinnacle_api::tag;
/// # use pinnacle_api::output;
/// # || {
/// let tags = tag::add(&output::get_by_name("DP-1")?, ["1", "2", "Buckle", "Shoe"]);
///
/// tag::remove(tags); // "DP-1" no longer has any tags
/// # Some(())
/// # };
/// ```
pub fn remove(tags: impl IntoIterator<Item = TagHandle>) {
    let tag_ids = tags.into_iter().map(|handle| handle.id).collect::<Vec<_>>();

    Client::tag()
        .remove(RemoveRequest { tag_ids })
        .block_on_tokio()
        .unwrap();
}

/// Connects to a [`TagSignal`].
///
/// # Examples
///
/// ```no_run
/// # use pinnacle_api::tag;
/// # use pinnacle_api::signal::TagSignal;
/// tag::connect_signal(TagSignal::Active(Box::new(|tag, active| {
///     println!("Tag is active = {active}");
/// })));
/// ```
pub fn connect_signal(signal: TagSignal) -> SignalHandle {
    let mut signal_state = Client::signal_state();

    match signal {
        TagSignal::Active(f) => signal_state.tag_active.add_callback(f),
        TagSignal::Created(f) => signal_state.tag_created.add_callback(f),
        TagSignal::Removed(f) => signal_state.tag_removed.add_callback(f),
    }
}

/// A handle to a tag.
///
/// This handle allows you to do things like switch to tags and get their properties.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TagHandle {
    pub(crate) id: u32,
}

impl TagHandle {
    /// Creates a tag handle from a numeric id.
    pub fn from_id(id: u32) -> Self {
        Self { id }
    }

    /// Activates this tag and deactivates all other ones on the same output.
    ///
    /// This emulates what a traditional workspace is.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pinnacle_api::tag;
    /// // Assume the focused output has the following inactive tags and windows:
    /// // "1": Alacritty
    /// // "2": Firefox, Discord
    /// // "3": Steam
    /// # || {
    /// tag::get("2")?.switch_to(); // Displays Firefox and Discord
    /// tag::get("3")?.switch_to(); // Displays Steam
    /// # Some(())
    /// # };
    /// ```
    pub fn switch_to(&self) {
        let tag_id = self.id;

        Client::tag()
            .switch_to(SwitchToRequest { tag_id })
            .block_on_tokio()
            .unwrap();
    }

    /// Sets this tag to active or not.
    ///
    /// While active, windows with this tag will be displayed.
    ///
    /// While inactive, windows with this tag will not be displayed unless they have other active
    /// tags.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pinnacle_api::tag;
    /// // Assume the focused output has the following inactive tags and windows:
    /// // "1": Alacritty
    /// // "2": Firefox, Discord
    /// // "3": Steam
    /// # || {
    /// tag::get("2")?.set_active(true);  // Displays Firefox and Discord
    /// tag::get("3")?.set_active(true);  // Displays Firefox, Discord, and Steam
    /// tag::get("2")?.set_active(false); // Displays Steam
    /// # Some(())
    /// # };
    /// ```
    pub fn set_active(&self, set: bool) {
        let tag_id = self.id;

        Client::tag()
            .set_active(SetActiveRequest {
                tag_id,
                set_or_toggle: match set {
                    true => SetOrToggle::Set,
                    false => SetOrToggle::Unset,
                }
                .into(),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Toggles this tag between active and inactive.
    ///
    /// While active, windows with this tag will be displayed.
    ///
    /// While inactive, windows with this tag will not be displayed unless they have other active
    /// tags.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pinnacle_api::tag;
    /// // Assume the focused output has the following inactive tags and windows:
    /// // "1": Alacritty
    /// // "2": Firefox, Discord
    /// // "3": Steam
    /// # || {
    /// tag::get("2")?.toggle_active(); // Displays Firefox and Discord
    /// tag::get("3")?.toggle_active(); // Displays Firefox, Discord, and Steam
    /// tag::get("3")?.toggle_active(); // Displays Firefox, Discord
    /// tag::get("2")?.toggle_active(); // Displays nothing
    /// # Some(())
    /// # };
    /// ```
    pub fn toggle_active(&self) {
        let tag_id = self.id;

        Client::tag()
            .set_active(SetActiveRequest {
                tag_id,
                set_or_toggle: SetOrToggle::Toggle.into(),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Removes this tag from its output.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pinnacle_api::tag;
    /// # use pinnacle_api::output;
    /// # || {
    /// let tags =
    ///     tag::add(&output::get_by_name("DP-1")?, ["1", "2", "Buckle", "Shoe"]).collect::<Vec<_>>();
    ///
    /// tags[1].remove();
    /// tags[3].remove();
    /// # Some(())
    /// # };
    /// // "DP-1" now only has tags "1" and "Buckle"
    /// ```
    pub fn remove(&self) {
        let tag_id = self.id;

        Client::tag()
            .remove(RemoveRequest {
                tag_ids: vec![tag_id],
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Gets whether or not this tag is active.
    pub fn active(&self) -> bool {
        self.active_async().block_on_tokio()
    }

    /// Async impl for [`Self::active`].
    pub async fn active_async(&self) -> bool {
        let tag_id = self.id;

        Client::tag()
            .get_active(GetActiveRequest { tag_id })
            .await
            .unwrap()
            .into_inner()
            .active
    }

    /// Gets this tag's name.
    pub fn name(&self) -> String {
        self.name_async().block_on_tokio()
    }

    /// Async impl for [`Self::name`].
    pub async fn name_async(&self) -> String {
        let tag_id = self.id;

        Client::tag()
            .get_name(GetNameRequest { tag_id })
            .await
            .unwrap()
            .into_inner()
            .name
    }

    /// Gets a handle to the output this tag is on.
    pub fn output(&self) -> OutputHandle {
        self.output_async().block_on_tokio()
    }

    /// Async impl for [`Self::output`].
    pub async fn output_async(&self) -> OutputHandle {
        let tag_id = self.id;

        let name = Client::tag()
            .get_output_name(GetOutputNameRequest { tag_id })
            .await
            .unwrap()
            .into_inner()
            .output_name;
        OutputHandle { name }
    }

    /// Gets all windows with this tag.
    pub fn windows(&self) -> impl Iterator<Item = WindowHandle> + use<> {
        self.windows_async().block_on_tokio()
    }

    /// Async impl for [`Self::windows`].
    pub async fn windows_async(&self) -> impl Iterator<Item = WindowHandle> + use<> {
        let windows = crate::window::get_all_async().await;
        let this = self.clone();
        windows.batch_filter(
            |win| win.tags_async().boxed(),
            move |mut tags| tags.any(|tag| tag == this),
        )
    }

    /// Gets this tag's raw compositor id.
    pub fn id(&self) -> u32 {
        self.id
    }
}
