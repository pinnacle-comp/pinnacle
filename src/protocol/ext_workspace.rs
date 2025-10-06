use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::mem;

use ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1;
use ext_workspace_handle_v1::ExtWorkspaceHandleV1;
use ext_workspace_manager_v1::ExtWorkspaceManagerV1;
use smithay::output::Output;
use smithay::reexports::wayland_protocols::ext::workspace::v1::server::{
    ext_workspace_group_handle_v1, ext_workspace_handle_v1, ext_workspace_manager_v1,
};
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
};
use wayland_backend::server::ClientId;

use crate::state::{State, WithState};
use crate::tag::{Tag, TagId};

const VERSION: u32 = 1;

pub trait ExtWorkspaceHandler {
    fn ext_workspace_manager_state(&mut self) -> &mut ExtWorkspaceManagerState;
    fn activate_workspace(&mut self, id: TagId);
    fn deactivate_workspace(&mut self, id: TagId);
    fn remove_workspace(&mut self, id: TagId);
    fn add_workspace(&mut self, output: &Output, name: String);
}

enum Action {
    Activate(TagId),
    Deactivate(TagId),
    Remove(TagId),
}

impl Action {
    fn order(&self) -> u8 {
        match self {
            Action::Activate(_) => 2,
            Action::Deactivate(_) => 1,
            Action::Remove(_) => 0,
        }
    }
}

pub struct ExtWorkspaceManagerState {
    display: DisplayHandle,
    instances: HashMap<ExtWorkspaceManagerV1, Vec<Action>>,
    tag_groups: HashMap<Output, ExtWorkspaceGroupData>,
    tags: HashMap<TagId, ExtWorkspaceData>,
}

struct ExtWorkspaceGroupData {
    instances: Vec<ExtWorkspaceGroupHandleV1>,
}

struct ExtWorkspaceData {
    id: String,
    name: String,
    state: ext_workspace_handle_v1::State,
    instances: Vec<ExtWorkspaceHandleV1>,
    output: Output,
}

pub struct ExtWorkspaceGlobalData {
    filter: Box<dyn for<'c> Fn(&'c Client) -> bool + Send + Sync>,
}

fn tags_by_output<'a>(
    outputs: impl Iterator<Item = &'a Output> + 'a,
) -> impl Iterator<Item = (Output, Tag)> {
    outputs.flat_map(|output| {
        output.with_state(|outp| {
            outp.tags
                .clone()
                .into_iter()
                .map(|tag| (output.clone(), tag))
        })
    })
}

pub fn on_output_bound(state: &mut State, output: &Output, wl_output: &WlOutput) {
    let Some(client) = wl_output.client() else {
        return;
    };
    let mut sent = false;

    let protocol_state = &mut state.pinnacle.ext_workspace_state;
    if let Some(data) = protocol_state.tag_groups.get_mut(output) {
        for group in &mut data.instances {
            if group.client().as_ref() != Some(&client) {
                continue;
            }

            group.output_enter(wl_output);
            sent = true;
        }
    }

    if !sent {
        return;
    }

    for manager in protocol_state.instances.keys() {
        if manager.client().as_ref() == Some(&client) {
            manager.done();
        }
    }
}

pub fn refresh(state: &mut State) {
    let _span = tracy_client::span!("ext_workspace::refresh");

    let protocol_state = &mut state.pinnacle.ext_workspace_state;

    let mut changed = false;

    let mut seen_tags = HashMap::new();
    for (output, tag) in tags_by_output(state.pinnacle.outputs.iter()) {
        seen_tags.insert(tag.id(), output);
    }

    protocol_state.tags.retain(|id, tag| {
        if seen_tags.contains_key(id) {
            return true;
        }

        remove_workspace_instances(&protocol_state.tag_groups, tag);
        changed = true;
        false
    });

    // Remove tag groups for outputs that no longer exist.
    protocol_state.tag_groups.retain(|output, data| {
        if state.pinnacle.outputs.contains(output) {
            return true;
        }

        for group in &data.instances {
            // Send workspace_leave for all workspaces in this group with matching manager.
            let manager: &ExtWorkspaceManagerV1 = group.data().unwrap();
            for tag in protocol_state.tags.values() {
                if &tag.output == output {
                    for tag in &tag.instances {
                        if tag.data() == Some(manager) {
                            group.workspace_leave(tag);
                        }
                    }
                }
            }

            group.removed();
        }

        changed = true;
        false
    });

    // Update existing tags and create new ones.
    for (output, tag) in tags_by_output(state.pinnacle.outputs.iter()) {
        changed |= refresh_workspace(protocol_state, &output, &tag);
    }

    // Update tag groups and create new ones, sending workspace_enter events as needed.
    for output in &state.pinnacle.outputs {
        changed |= refresh_workspace_group(protocol_state, output);
    }

    if changed {
        for manager in protocol_state.instances.keys() {
            manager.done();
        }
    }
}

fn refresh_workspace_group(protocol_state: &mut ExtWorkspaceManagerState, output: &Output) -> bool {
    if protocol_state.tag_groups.contains_key(output) {
        // Existing tag group. Nothing can actually change since our tag groups are tied
        // to an output. That is, a tag group is the set of tags assigned to an output.
        return false;
    }

    // New workspace group, start tracking it.
    let mut data = ExtWorkspaceGroupData {
        instances: Vec::new(),
    };

    // Create workspace group handle for each manager instance.
    for manager in protocol_state.instances.keys() {
        if let Some(client) = manager.client() {
            data.add_instance::<State>(&protocol_state.display, &client, manager, output);
        }
    }

    // Send workspace_enter for all existing workspaces on this output.
    for group in &data.instances {
        let manager: &ExtWorkspaceManagerV1 = group.data().unwrap();
        for (_, tag_data) in protocol_state.tags.iter() {
            if &tag_data.output != output {
                continue;
            }
            for workspace_handle in &tag_data.instances {
                if workspace_handle.data() == Some(manager) {
                    group.workspace_enter(workspace_handle);
                }
            }
        }
    }

    protocol_state.tag_groups.insert(output.clone(), data);
    true
}

fn send_workspace_enter_leave(
    tag_groups: &HashMap<Output, ExtWorkspaceGroupData>,
    data: &ExtWorkspaceData,
    enter: bool,
) {
    if let Some(group_data) = tag_groups.get(&data.output) {
        for group in &group_data.instances {
            let manager: &ExtWorkspaceManagerV1 = group.data().unwrap();
            for workspace in &data.instances {
                if workspace.data() == Some(manager) {
                    if enter {
                        group.workspace_enter(workspace);
                    } else {
                        group.workspace_leave(workspace);
                    }
                }
            }
        }
    }
}

fn remove_workspace_instances(
    workspace_groups: &HashMap<Output, ExtWorkspaceGroupData>,
    data: &ExtWorkspaceData,
) {
    send_workspace_enter_leave(workspace_groups, data, false);

    for workspace in &data.instances {
        workspace.removed();
    }
}

fn refresh_workspace(
    protocol_state: &mut ExtWorkspaceManagerState,
    output: &Output,
    tag: &Tag,
) -> bool {
    let mut state = ext_workspace_handle_v1::State::empty();
    if tag.active() {
        state |= ext_workspace_handle_v1::State::Active;
    }

    match protocol_state.tags.entry(tag.id()) {
        Entry::Occupied(entry) => {
            // Existing workspace, check if anything changed.
            let data = entry.into_mut();

            let mut state_changed = false;
            if data.state != state {
                data.state = state;
                state_changed = true;
            }

            let mut output_changed = false;
            if &data.output != output {
                send_workspace_enter_leave(&protocol_state.tag_groups, data, false);
                data.output = output.clone();
                output_changed = true;
            }

            if output_changed {
                // Send workspace_enter to the new output's group. If the group doesn't exist yet
                // (new groups are created after refreshing workspaces), then workspace_enter() will
                // be sent when the group is created.
                send_workspace_enter_leave(&protocol_state.tag_groups, data, true);
            }

            if state_changed {
                for instance in &data.instances {
                    instance.id(data.id.clone());
                    instance.name(data.name.clone());
                    instance.state(data.state);
                }
            }

            output_changed || state_changed
        }
        Entry::Vacant(entry) => {
            // New workspace, start tracking it.
            let mut data = ExtWorkspaceData {
                id: tag.id().to_string(),
                name: tag.name(),
                state,
                instances: Vec::new(),
                output: output.clone(),
            };

            for manager in protocol_state.instances.keys() {
                if let Some(client) = manager.client() {
                    data.add_instance::<State>(&protocol_state.display, &client, manager);
                }
            }

            send_workspace_enter_leave(&protocol_state.tag_groups, &data, true);
            entry.insert(data);
            true
        }
    }
}

impl ExtWorkspaceGroupData {
    fn add_instance<D>(
        &mut self,
        handle: &DisplayHandle,
        client: &Client,
        manager: &ExtWorkspaceManagerV1,
        output: &Output,
    ) -> &ExtWorkspaceGroupHandleV1
    where
        D: Dispatch<ExtWorkspaceGroupHandleV1, ExtWorkspaceManagerV1>,
        D: 'static,
    {
        let group = client
            .create_resource::<ExtWorkspaceGroupHandleV1, _, D>(
                handle,
                manager.version(),
                manager.clone(),
            )
            .unwrap();
        manager.workspace_group(&group);

        group.capabilities(ext_workspace_group_handle_v1::GroupCapabilities::CreateWorkspace);

        for wl_output in output.client_outputs(client) {
            group.output_enter(&wl_output);
        }

        self.instances.push(group);
        self.instances.last().unwrap()
    }
}

impl ExtWorkspaceData {
    fn add_instance<D>(
        &mut self,
        handle: &DisplayHandle,
        client: &Client,
        manager: &ExtWorkspaceManagerV1,
    ) -> &ExtWorkspaceHandleV1
    where
        D: Dispatch<ExtWorkspaceHandleV1, ExtWorkspaceManagerV1>,
        D: 'static,
    {
        let workspace = client
            .create_resource::<ExtWorkspaceHandleV1, _, D>(
                handle,
                manager.version(),
                manager.clone(),
            )
            .unwrap();
        manager.workspace(&workspace);

        workspace.id(self.id.clone());
        workspace.name(self.name.clone());
        workspace.state(self.state);
        workspace.capabilities(
            ext_workspace_handle_v1::WorkspaceCapabilities::Activate
                | ext_workspace_handle_v1::WorkspaceCapabilities::Deactivate
                | ext_workspace_handle_v1::WorkspaceCapabilities::Remove,
        );

        self.instances.push(workspace);
        self.instances.last().unwrap()
    }
}

impl ExtWorkspaceManagerState {
    pub fn new<D, F>(display: &DisplayHandle, filter: F) -> Self
    where
        D: GlobalDispatch<ExtWorkspaceManagerV1, ExtWorkspaceGlobalData>,
        D: Dispatch<ExtWorkspaceManagerV1, ()>,
        D: 'static,
        F: for<'c> Fn(&'c Client) -> bool + Send + Sync + 'static,
    {
        let global_data = ExtWorkspaceGlobalData {
            filter: Box::new(filter),
        };
        display.create_global::<D, ExtWorkspaceManagerV1, _>(VERSION, global_data);
        Self {
            instances: HashMap::new(),
            tag_groups: HashMap::new(),
            tags: HashMap::new(),
            display: display.clone(),
        }
    }
}

impl<D> GlobalDispatch<ExtWorkspaceManagerV1, ExtWorkspaceGlobalData, D>
    for ExtWorkspaceManagerState
where
    D: GlobalDispatch<ExtWorkspaceManagerV1, ExtWorkspaceGlobalData>,
    D: Dispatch<ExtWorkspaceManagerV1, ()>,
    D: Dispatch<ExtWorkspaceHandleV1, ExtWorkspaceManagerV1>,
    D: ExtWorkspaceHandler,
{
    fn bind(
        state: &mut D,
        handle: &DisplayHandle,
        client: &Client,
        resource: New<ExtWorkspaceManagerV1>,
        _global_data: &ExtWorkspaceGlobalData,
        data_init: &mut DataInit<'_, D>,
    ) {
        let manager = data_init.init(resource, ());

        let state = state.ext_workspace_manager_state();

        // Send existing workspaces to the new client.
        let mut new_tags: HashMap<_, Vec<_>> = HashMap::new();
        for data in state.tags.values_mut() {
            let output = data.output.clone();
            let tag = data.add_instance::<State>(handle, client, &manager);

            new_tags.entry(output).or_default().push(tag);
        }

        // Create workspace groups for all outputs.
        for (output, group_data) in &mut state.tag_groups {
            let group = group_data.add_instance::<State>(handle, client, &manager, output);

            for tag in new_tags.get(output).into_iter().flatten() {
                group.workspace_enter(tag);
            }
        }

        manager.done();
        state.instances.insert(manager, Vec::new());
    }

    fn can_view(client: Client, global_data: &ExtWorkspaceGlobalData) -> bool {
        (global_data.filter)(&client)
    }
}

impl<D> Dispatch<ExtWorkspaceManagerV1, (), D> for ExtWorkspaceManagerState
where
    D: Dispatch<ExtWorkspaceManagerV1, ()>,
    D: ExtWorkspaceHandler,
{
    fn request(
        state: &mut D,
        _client: &Client,
        resource: &ExtWorkspaceManagerV1,
        request: <ExtWorkspaceManagerV1 as Resource>::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            ext_workspace_manager_v1::Request::Commit => {
                let protocol_state = state.ext_workspace_manager_state();
                let actions = protocol_state.instances.get_mut(resource).unwrap();
                let mut actions = mem::take(actions);

                actions.sort_by_key(Action::order);

                for action in actions {
                    match action {
                        Action::Activate(id) => state.activate_workspace(id),
                        Action::Deactivate(id) => state.deactivate_workspace(id),
                        Action::Remove(id) => state.remove_workspace(id),
                    }
                }
            }
            ext_workspace_manager_v1::Request::Stop => {
                resource.finished();

                let state = state.ext_workspace_manager_state();
                state.instances.retain(|x, _| x != resource);

                for data in state.tag_groups.values_mut() {
                    data.instances
                        .retain(|instance| instance.data() != Some(resource));
                }

                for data in state.tags.values_mut() {
                    data.instances
                        .retain(|instance| instance.data() != Some(resource));
                }
            }
            _ => unreachable!(),
        }
    }

    fn destroyed(state: &mut D, _client: ClientId, resource: &ExtWorkspaceManagerV1, _data: &()) {
        let state = state.ext_workspace_manager_state();
        state.instances.retain(|x, _| x != resource);
    }
}

impl<D> Dispatch<ExtWorkspaceHandleV1, ExtWorkspaceManagerV1, D> for ExtWorkspaceManagerState
where
    D: Dispatch<ExtWorkspaceHandleV1, ExtWorkspaceManagerV1>,
    D: ExtWorkspaceHandler,
{
    fn request(
        state: &mut D,
        _client: &Client,
        resource: &ExtWorkspaceHandleV1,
        request: <ExtWorkspaceHandleV1 as Resource>::Request,
        data: &ExtWorkspaceManagerV1,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        let protocol_state = state.ext_workspace_manager_state();

        let Some((workspace, _)) = protocol_state
            .tags
            .iter()
            .find(|(_, data)| data.instances.contains(resource))
        else {
            return;
        };
        let workspace = *workspace;

        match request {
            ext_workspace_handle_v1::Request::Activate => {
                let actions = protocol_state.instances.get_mut(data).unwrap();
                actions.push(Action::Activate(workspace));
            }
            ext_workspace_handle_v1::Request::Deactivate => {
                let actions = protocol_state.instances.get_mut(data).unwrap();
                actions.push(Action::Deactivate(workspace));
            }
            ext_workspace_handle_v1::Request::Assign { .. } => (),
            ext_workspace_handle_v1::Request::Remove => {
                let actions = protocol_state.instances.get_mut(data).unwrap();
                actions.push(Action::Remove(workspace));
            }
            ext_workspace_handle_v1::Request::Destroy => (),
            _ => unreachable!(),
        }
    }

    fn destroyed(
        state: &mut D,
        _client: ClientId,
        resource: &ExtWorkspaceHandleV1,
        _data: &ExtWorkspaceManagerV1,
    ) {
        let state = state.ext_workspace_manager_state();
        for data in state.tags.values_mut() {
            data.instances.retain(|instance| instance != resource);
        }
    }
}

impl<D> Dispatch<ExtWorkspaceGroupHandleV1, ExtWorkspaceManagerV1, D> for ExtWorkspaceManagerState
where
    D: Dispatch<ExtWorkspaceGroupHandleV1, ExtWorkspaceManagerV1>,
    D: ExtWorkspaceHandler,
{
    fn request(
        state: &mut D,
        _client: &Client,
        resource: &ExtWorkspaceGroupHandleV1,
        request: <ExtWorkspaceGroupHandleV1 as Resource>::Request,
        _data: &ExtWorkspaceManagerV1,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            ext_workspace_group_handle_v1::Request::CreateWorkspace { workspace } => {
                if let Some(output) = state
                    .ext_workspace_manager_state()
                    .tag_groups
                    .iter()
                    .find_map(|(output, group)| {
                        (group.instances.contains(resource)).then_some(output.clone())
                    })
                {
                    state.add_workspace(&output, workspace);
                }
            }
            ext_workspace_group_handle_v1::Request::Destroy => (),
            _ => unreachable!(),
        }
    }

    fn destroyed(
        state: &mut D,
        _client: ClientId,
        resource: &ExtWorkspaceGroupHandleV1,
        _data: &ExtWorkspaceManagerV1,
    ) {
        let state = state.ext_workspace_manager_state();
        for data in state.tag_groups.values_mut() {
            data.instances.retain(|instance| instance != resource);
        }
    }
}

#[macro_export]
macro_rules! delegate_ext_workspace {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        smithay::reexports::wayland_server::delegate_global_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols::ext::workspace::v1::server::ext_workspace_manager_v1::ExtWorkspaceManagerV1: $crate::protocol::ext_workspace::ExtWorkspaceGlobalData
        ] => $crate::protocol::ext_workspace::ExtWorkspaceManagerState);
        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols::ext::workspace::v1::server::ext_workspace_manager_v1::ExtWorkspaceManagerV1: ()
        ] => $crate::protocol::ext_workspace::ExtWorkspaceManagerState);
        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols::ext::workspace::v1::server::ext_workspace_handle_v1::ExtWorkspaceHandleV1: smithay::reexports::wayland_protocols::ext::workspace::v1::server::ext_workspace_manager_v1::ExtWorkspaceManagerV1
        ] => $crate::protocol::ext_workspace::ExtWorkspaceManagerState);
        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols::ext::workspace::v1::server::ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1: smithay::reexports::wayland_protocols::ext::workspace::v1::server::ext_workspace_manager_v1::ExtWorkspaceManagerV1
        ] => $crate::protocol::ext_workspace::ExtWorkspaceManagerState);
    };
}
