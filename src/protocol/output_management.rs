use anyhow::Context;
use smithay::{
    output::{Output, WeakOutput},
    reexports::{
        wayland_protocols_wlr::output_management::v1::server::{
            zwlr_output_configuration_head_v1::{self, ZwlrOutputConfigurationHeadV1},
            zwlr_output_configuration_v1,
            zwlr_output_head_v1::{self, AdaptiveSyncState},
            zwlr_output_mode_v1::{self, ZwlrOutputModeV1},
        },
        wayland_server::{Resource, WEnum},
    },
    utils::{Logical, Physical, Point, SERIAL_COUNTER, Size, Transform},
};
use std::{
    collections::{HashMap, HashSet},
    num::NonZeroU32,
    sync::Mutex,
};
use tracing::error;

use smithay::{
    output::Mode,
    reexports::{
        wayland_protocols_wlr::output_management::v1::server::{
            zwlr_output_configuration_v1::ZwlrOutputConfigurationV1,
            zwlr_output_head_v1::ZwlrOutputHeadV1,
            zwlr_output_manager_v1::{self, ZwlrOutputManagerV1},
        },
        wayland_server::{
            self, Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, backend::ClientId,
        },
    },
};

use crate::state::WithState;

const VERSION: u32 = 4;

pub struct OutputManagementManagerState {
    display_handle: DisplayHandle,
    managers: HashMap<ZwlrOutputManagerV1, OutputManagerData>,
    outputs: HashMap<WeakOutput, OutputData>,
    removed_outputs: HashSet<WeakOutput>,
}

struct OutputManagerData {
    serial: u32,
    configurations: Vec<ZwlrOutputConfigurationV1>,
    heads: HashMap<ZwlrOutputHeadV1, Vec<ZwlrOutputModeV1>>,
}

pub struct OutputManagementGlobalData {
    filter: Box<dyn Fn(&Client) -> bool + Send + Sync>,
}

#[derive(Debug)]
enum PendingHead {
    NotConfigured,
    Enabled(ZwlrOutputConfigurationHeadV1),
    Disabled,
}

#[derive(Debug)]
pub struct PendingOutputConfiguration {
    serial: u32,
    inner: Mutex<PendingOutputConfigurationInner>,
}

#[derive(Default, Debug)]
struct PendingOutputConfigurationInner {
    cancelled: bool,
    used: bool,
    pending_heads: HashMap<ZwlrOutputHeadV1, PendingHead>,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct PendingOutputHeadConfiguration {
    pub mode: Option<(Size<i32, Physical>, Option<NonZeroU32>)>,
    pub position: Option<Point<i32, Logical>>,
    pub transform: Option<Transform>,
    pub scale: Option<f64>,
    pub adaptive_sync: Option<bool>,
}

#[derive(Debug)]
pub enum OutputConfiguration {
    Disabled,
    Enabled {
        mode: Option<(Size<i32, Physical>, Option<NonZeroU32>)>,
        position: Option<Point<i32, Logical>>,
        transform: Option<Transform>,
        scale: Option<f64>,
        adaptive_sync: Option<bool>,
    },
}

pub trait OutputManagementHandler {
    fn output_management_manager_state(&mut self) -> &mut OutputManagementManagerState;
    fn apply_configuration(&mut self, config: HashMap<Output, OutputConfiguration>) -> bool;
    fn test_configuration(&mut self, config: HashMap<Output, OutputConfiguration>) -> bool;
}

#[derive(Debug, Clone)]
pub struct OutputData {
    enabled: bool,
    current_mode: Option<Mode>,
    position: Point<i32, Logical>,
    transform: Transform,
    scale: f64,
    _adaptive_sync: bool,
}

impl OutputManagementManagerState {
    /// Adds this head.
    ///
    /// [`OutputManagementManagerState::update`] needs to be called afterwards to apply the new state.
    pub fn add_head<D>(&mut self, output: &Output)
    where
        D: Dispatch<ZwlrOutputHeadV1, Output>
            + Dispatch<ZwlrOutputModeV1, Mode>
            + OutputManagementHandler
            + 'static,
    {
        if self.outputs.contains_key(&output.downgrade()) {
            return;
        }

        for (manager, manager_data) in self.managers.iter_mut() {
            let (head, modes) = match advertise_output::<D>(&self.display_handle, manager, output) {
                Ok(ret) => ret,
                Err(err) => {
                    error!("Failed to advertise output to output management: {err}");
                    continue;
                }
            };
            manager_data.heads.insert(head, modes);
        }

        let output_data = OutputData {
            enabled: true,
            current_mode: output.current_mode(),
            position: output.current_location(),
            transform: output.current_transform(),
            scale: output.current_scale().fractional_scale(),
            _adaptive_sync: false, // TODO:
        };

        self.outputs.insert(output.downgrade(), output_data);
    }

    /// Mark this head as removed.
    ///
    /// [`OutputManagementManagerState::update`] needs to be called afterwards to apply the new state.
    pub fn remove_head(&mut self, output: &Output) {
        if self.outputs.remove(&output.downgrade()).is_some() {
            self.removed_outputs.insert(output.downgrade());
        }
    }

    /// Mark this head as enabled or not.
    ///
    /// [`OutputManagementManagerState::update`] needs to be called afterwards to apply the new state.
    pub fn set_head_enabled<D>(&mut self, output: &Output, enabled: bool)
    where
        D: Dispatch<ZwlrOutputHeadV1, Output>
            + Dispatch<ZwlrOutputModeV1, Mode>
            + OutputManagementHandler
            + 'static,
    {
        let Some(output_data) = self.outputs.get_mut(&output.downgrade()) else {
            return;
        };

        output_data.enabled = enabled;

        for manager_data in self.managers.values_mut() {
            for (head, wlr_modes) in manager_data.heads.iter_mut() {
                if head.data::<Output>() == Some(output) {
                    head.enabled(enabled as i32);

                    if enabled {
                        if let Some(current_mode) = output.current_mode() {
                            let wlr_current_mode = wlr_modes
                                .iter()
                                .find(|wlr_mode| wlr_mode.data::<Mode>() == Some(&current_mode));
                            if let Some(wlr_current_mode) = wlr_current_mode {
                                head.current_mode(wlr_current_mode);
                            } else {
                                let new_wlr_mode = create_mode_for_head::<D>(
                                    head,
                                    &self.display_handle,
                                    current_mode,
                                    output.preferred_mode() == Some(current_mode),
                                );

                                match new_wlr_mode {
                                    Ok(new_wlr_current_mode) => {
                                        head.current_mode(&new_wlr_current_mode);
                                        wlr_modes.push(new_wlr_current_mode);
                                    }
                                    Err(err) => error!("Failed to create wlr mode: {err}"),
                                }
                            }
                        }
                        let new_loc = output.current_location();
                        head.position(new_loc.x, new_loc.y);
                        output_data.position = new_loc;

                        let new_transform = output.current_transform();
                        head.transform(new_transform.into());
                        output_data.transform = new_transform;

                        let new_scale = output.current_scale().fractional_scale();
                        head.scale(new_scale);
                        output_data.scale = new_scale;
                    }
                }
            }
        }
    }

    /// Update output management state.
    ///
    /// This needs to be called whenever output state changes to notify clients of the new state.
    pub fn update<D>(&mut self)
    where
        D: Dispatch<ZwlrOutputHeadV1, Output>
            + Dispatch<ZwlrOutputModeV1, Mode>
            + OutputManagementHandler
            + 'static,
    {
        for output in self.removed_outputs.drain() {
            let Some(output) = output.upgrade() else {
                continue;
            };
            for data in self.managers.values_mut() {
                let heads = data.heads.keys().cloned().collect::<Vec<_>>();
                for head in heads {
                    if head.data::<Output>() == Some(&output) {
                        let modes = data.heads.remove(&head);
                        if let Some(modes) = modes {
                            for mode in modes {
                                mode.finished();
                            }
                        }
                        head.finished();
                    }
                }
            }
        }

        let serial = u32::from(SERIAL_COUNTER.next_serial());

        self.outputs.retain(|output, output_data| {
            let Some(output) = output.upgrade() else {
                return false;
            };

            for (manager, manager_data) in self.managers.iter_mut() {
                for (head, wlr_modes) in manager_data.heads.iter_mut() {
                    if head.data::<Output>() != Some(&output) {
                        continue;
                    }

                    let modes = output.with_state(|state| state.modes.clone());

                    wlr_modes.retain(|wlr_mode| {
                        if !modes.contains(wlr_mode.data::<Mode>().unwrap()) {
                            wlr_mode.finished();
                            false
                        } else {
                            true
                        }
                    });

                    for mode in modes {
                        if !wlr_modes
                            .iter()
                            .any(|wlr_mode| wlr_mode.data::<Mode>().unwrap() == &mode)
                        {
                            let new_wlr_mode = create_mode_for_head::<D>(
                                head,
                                &self.display_handle,
                                mode,
                                output.preferred_mode() == Some(mode),
                            );

                            match new_wlr_mode {
                                Ok(new_wlr_current_mode) => wlr_modes.push(new_wlr_current_mode),
                                Err(err) => error!("Failed to create wlr mode: {err}"),
                            }
                        }
                    }

                    // enabled handled in `set_head_enabled`

                    if output_data.enabled {
                        if output.current_mode() != output_data.current_mode
                            && let Some(new_cur_mode) = output.current_mode()
                        {
                            let new_cur_wlr_mode = wlr_modes
                                .iter()
                                .find(|wlr_mode| wlr_mode.data::<Mode>() == Some(&new_cur_mode));

                            match new_cur_wlr_mode {
                                Some(new_cur_wlr_mode) => {
                                    head.current_mode(new_cur_wlr_mode);
                                }
                                None => {
                                    let new_wlr_current_mode = create_mode_for_head::<D>(
                                        head,
                                        &self.display_handle,
                                        new_cur_mode,
                                        output.preferred_mode() == Some(new_cur_mode),
                                    );

                                    match new_wlr_current_mode {
                                        Ok(new_wlr_current_mode) => {
                                            head.current_mode(&new_wlr_current_mode);
                                            wlr_modes.push(new_wlr_current_mode);
                                        }
                                        Err(err) => error!("Failed to create wlr mode: {err}"),
                                    }
                                }
                            }

                            output_data.current_mode = Some(new_cur_mode);
                        }

                        if output.current_location() != output_data.position {
                            let new_loc = output.current_location();
                            head.position(new_loc.x, new_loc.y);
                            output_data.position = new_loc;
                        }

                        if output.current_transform() != output_data.transform {
                            let new_transform = output.current_transform();
                            head.transform(new_transform.into());
                            output_data.transform = new_transform;
                        }

                        if output.current_scale().fractional_scale() != output_data.scale {
                            let new_scale = output.current_scale().fractional_scale();
                            head.scale(new_scale);
                            output_data.scale = new_scale;
                        }
                    }

                    // TODO: adaptive sync
                }

                manager_data.serial = serial;
                manager.done(serial);
            }

            true
        });
    }
}

fn advertise_output<D>(
    display: &DisplayHandle,
    manager: &ZwlrOutputManagerV1,
    output: &Output,
) -> anyhow::Result<(ZwlrOutputHeadV1, Vec<ZwlrOutputModeV1>)>
where
    D: Dispatch<ZwlrOutputHeadV1, Output>
        + Dispatch<ZwlrOutputModeV1, Mode>
        + OutputManagementHandler
        + 'static,
{
    let client = manager
        .client()
        .context("output manager has no owning client")?;

    let head = client.create_resource::<ZwlrOutputHeadV1, _, D>(
        display,
        manager.version(),
        output.clone(),
    )?;

    manager.head(&head);

    head.name(output.name());
    head.description(output.description());

    let physical_props = output.physical_properties();
    head.physical_size(physical_props.size.w, physical_props.size.h);

    let mut wlr_modes = Vec::new();
    for mode in output.with_state(|state| state.modes.clone()) {
        let wlr_mode =
            create_mode_for_head::<D>(&head, display, mode, output.preferred_mode() == Some(mode));

        match wlr_mode {
            Ok(wlr_mode) => wlr_modes.push(wlr_mode),
            Err(err) => error!("Failed to create wlr mode: {err}"),
        }
    }

    if head.version() >= zwlr_output_head_v1::EVT_MAKE_SINCE {
        head.make(physical_props.make);
        head.model(physical_props.model);
        head.serial_number(physical_props.serial_number);
    }

    if head.version() >= zwlr_output_head_v1::EVT_ADAPTIVE_SYNC_SINCE {
        // TODO:
        // head.adaptive_sync(match data.adaptive_sync {
        //     true => AdaptiveSyncState::Enabled,
        //     false => AdaptiveSyncState::Disabled,
        // });
        head.adaptive_sync(AdaptiveSyncState::Disabled);
    }

    head.enabled(true as i32);
    if let Some(current_mode) = output.current_mode() {
        let wlr_current_mode = wlr_modes
            .iter()
            .find(|wlr_mode| wlr_mode.data::<Mode>() == Some(&current_mode));
        if let Some(wlr_current_mode) = wlr_current_mode {
            head.current_mode(wlr_current_mode);
        } else {
            let new_wlr_current_mode = create_mode_for_head::<D>(
                &head,
                display,
                current_mode,
                output.preferred_mode() == Some(current_mode),
            );

            match new_wlr_current_mode {
                Ok(new_wlr_current_mode) => {
                    head.current_mode(&new_wlr_current_mode);
                    wlr_modes.push(new_wlr_current_mode);
                }
                Err(err) => error!("Failed to create wlr mode: {err}"),
            }
        }
    }
    head.position(output.current_location().x, output.current_location().y);
    head.transform(output.current_transform().into());
    head.scale(output.current_scale().fractional_scale());

    Ok((head, wlr_modes))
}

fn create_mode_for_head<D>(
    head: &ZwlrOutputHeadV1,
    display_handle: &DisplayHandle,
    mode: Mode,
    is_preferred: bool,
) -> anyhow::Result<ZwlrOutputModeV1>
where
    D: Dispatch<ZwlrOutputHeadV1, Output>
        + Dispatch<ZwlrOutputModeV1, Mode>
        + OutputManagementHandler
        + 'static,
{
    let client = head.client().context("head has no owning client")?;
    let wlr_mode =
        client.create_resource::<ZwlrOutputModeV1, _, D>(display_handle, head.version(), mode)?;

    // do not reorder or wlr-randr gets 0x0 modes
    head.mode(&wlr_mode);

    wlr_mode.size(mode.size.w, mode.size.h);
    wlr_mode.refresh(mode.refresh);

    if is_preferred {
        wlr_mode.preferred();
    }

    Ok(wlr_mode)
}

fn manager_for_configuration<'a, D>(
    state: &'a mut D,
    configuration: &ZwlrOutputConfigurationV1,
) -> Option<(&'a ZwlrOutputManagerV1, &'a mut OutputManagerData)>
where
    D: OutputManagementHandler,
{
    state
        .output_management_manager_state()
        .managers
        .iter_mut()
        .find(|(_, manager_data)| manager_data.configurations.contains(configuration))
}

impl OutputManagementManagerState {
    pub fn new<D, F>(display: &DisplayHandle, filter: F) -> Self
    where
        D: GlobalDispatch<ZwlrOutputManagerV1, OutputManagementGlobalData>
            + Dispatch<ZwlrOutputManagerV1, ()>
            + 'static,
        F: Fn(&Client) -> bool + Send + Sync + 'static,
    {
        let global_data = OutputManagementGlobalData {
            filter: Box::new(filter),
        };

        display.create_global::<D, ZwlrOutputManagerV1, _>(VERSION, global_data);

        Self {
            display_handle: display.clone(),
            managers: HashMap::new(),
            outputs: HashMap::new(),
            removed_outputs: HashSet::new(),
        }
    }
}

impl<D> GlobalDispatch<ZwlrOutputManagerV1, OutputManagementGlobalData, D>
    for OutputManagementManagerState
where
    D: GlobalDispatch<ZwlrOutputManagerV1, OutputManagementGlobalData>
        + Dispatch<ZwlrOutputManagerV1, ()>
        + Dispatch<ZwlrOutputHeadV1, Output>
        + Dispatch<ZwlrOutputModeV1, Mode>
        + OutputManagementHandler,
{
    fn bind(
        state: &mut D,
        handle: &DisplayHandle,
        _client: &Client,
        resource: wayland_server::New<ZwlrOutputManagerV1>,
        _global_data: &OutputManagementGlobalData,
        data_init: &mut DataInit<'_, D>,
    ) {
        let manager = data_init.init(resource, ());

        let mut outputs = Vec::new();

        state
            .output_management_manager_state()
            .outputs
            .retain(|output, _| match output.upgrade() {
                Some(output) => {
                    outputs.push(output);
                    true
                }
                None => false,
            });

        let heads = outputs
            .into_iter()
            .flat_map(|output| advertise_output::<D>(handle, &manager, &output))
            .collect();

        let serial = u32::from(SERIAL_COUNTER.next_serial());

        manager.done(serial);

        let state = state.output_management_manager_state();

        let data = OutputManagerData {
            serial,
            configurations: Vec::new(),
            heads,
        };

        state.managers.insert(manager, data);
    }

    fn can_view(client: Client, global_data: &OutputManagementGlobalData) -> bool {
        (global_data.filter)(&client)
    }
}

impl<D> Dispatch<ZwlrOutputManagerV1, (), D> for OutputManagementManagerState
where
    D: Dispatch<ZwlrOutputManagerV1, ()> + OutputManagementHandler,
    D: Dispatch<ZwlrOutputConfigurationV1, PendingOutputConfiguration> + OutputManagementHandler,
{
    fn request(
        state: &mut D,
        _client: &Client,
        resource: &ZwlrOutputManagerV1,
        request: <ZwlrOutputManagerV1 as wayland_server::Resource>::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            zwlr_output_manager_v1::Request::CreateConfiguration { id, serial } => {
                let Some(manager_data) = state
                    .output_management_manager_state()
                    .managers
                    .get_mut(resource)
                else {
                    let config = PendingOutputConfiguration {
                        serial,
                        inner: Mutex::new(PendingOutputConfigurationInner {
                            cancelled: false,
                            used: false,
                            pending_heads: HashMap::new(),
                        }),
                    };

                    let config = data_init.init(id, config);

                    config.cancelled();
                    return;
                };

                #[allow(clippy::mutable_key_type)]
                let pending_heads = manager_data
                    .heads
                    .keys()
                    .map(|head| (head.clone(), PendingHead::NotConfigured))
                    .collect::<HashMap<_, _>>();

                let config = PendingOutputConfiguration {
                    serial,
                    inner: Mutex::new(PendingOutputConfigurationInner {
                        cancelled: false,
                        used: false,
                        pending_heads,
                    }),
                };

                let config = data_init.init(id, config);

                let correct_serial = manager_data.serial == serial;

                if !correct_serial {
                    config.cancelled();
                    config
                        .data::<PendingOutputConfiguration>()
                        .unwrap()
                        .inner
                        .lock()
                        .unwrap()
                        .cancelled = true;
                    return;
                }

                manager_data.configurations.push(config);
            }
            zwlr_output_manager_v1::Request::Stop => {
                resource.finished();

                state
                    .output_management_manager_state()
                    .managers
                    .remove(resource);
            }
            _ => unreachable!(),
        }
    }

    fn destroyed(state: &mut D, _client: ClientId, resource: &ZwlrOutputManagerV1, _data: &()) {
        state
            .output_management_manager_state()
            .managers
            .remove(resource);
    }
}

impl<D> Dispatch<ZwlrOutputHeadV1, Output, D> for OutputManagementManagerState
where
    D: OutputManagementHandler + 'static,
{
    fn request(
        state: &mut D,
        _client: &Client,
        resource: &ZwlrOutputHeadV1,
        request: <ZwlrOutputHeadV1 as Resource>::Request,
        _data: &Output,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            zwlr_output_head_v1::Request::Release => {
                for manager_data in state
                    .output_management_manager_state()
                    .managers
                    .values_mut()
                {
                    manager_data.heads.remove(resource);
                }
            }
            _ => unreachable!(),
        }
    }

    fn destroyed(state: &mut D, _client: ClientId, resource: &ZwlrOutputHeadV1, _data: &Output) {
        for manager_data in state
            .output_management_manager_state()
            .managers
            .values_mut()
        {
            manager_data.heads.remove(resource);
        }
    }
}

impl<D> Dispatch<ZwlrOutputModeV1, Mode, D> for OutputManagementManagerState
where
    D: OutputManagementHandler + 'static,
{
    fn request(
        state: &mut D,
        _client: &Client,
        resource: &ZwlrOutputModeV1,
        request: <ZwlrOutputModeV1 as Resource>::Request,
        _data: &Mode,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            zwlr_output_mode_v1::Request::Release => {
                for manager_data in state
                    .output_management_manager_state()
                    .managers
                    .values_mut()
                {
                    for modes in manager_data.heads.values_mut() {
                        modes.retain(|mode| mode != resource);
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    fn destroyed(state: &mut D, _client: ClientId, resource: &ZwlrOutputModeV1, _data: &Mode) {
        for manager_data in state
            .output_management_manager_state()
            .managers
            .values_mut()
        {
            for modes in manager_data.heads.values_mut() {
                modes.retain(|mode| mode != resource);
            }
        }
    }
}

impl<D> Dispatch<ZwlrOutputConfigurationV1, PendingOutputConfiguration, D>
    for OutputManagementManagerState
where
    D: Dispatch<ZwlrOutputManagerV1, ()>
        + Dispatch<ZwlrOutputConfigurationHeadV1, Mutex<PendingOutputHeadConfiguration>>
        + OutputManagementHandler,
{
    fn request(
        state: &mut D,
        _client: &Client,
        resource: &ZwlrOutputConfigurationV1,
        request: <ZwlrOutputConfigurationV1 as Resource>::Request,
        pending_data: &PendingOutputConfiguration,
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            zwlr_output_configuration_v1::Request::EnableHead { id, head } => {
                let config_head =
                    data_init.init(id, Mutex::new(PendingOutputHeadConfiguration::default()));

                let mut data = pending_data.inner.lock().unwrap();

                if data.cancelled {
                    return;
                }

                if data.used {
                    resource.post_error(
                        zwlr_output_configuration_v1::Error::AlreadyUsed,
                        "configuration has already been applied or tested",
                    );
                    return;
                }

                let manager_serial =
                    manager_for_configuration(state, resource).map(|(_, data)| data.serial);

                if manager_serial != Some(pending_data.serial) {
                    resource.cancelled();
                    data.cancelled = true;
                    return;
                }

                if let Some(pending_data) = data.pending_heads.get_mut(&head) {
                    if !matches!(pending_data, PendingHead::NotConfigured) {
                        head.post_error(
                            zwlr_output_configuration_v1::Error::AlreadyConfiguredHead,
                            "head has already been configured",
                        );
                        return;
                    }

                    *pending_data = PendingHead::Enabled(config_head);
                }
            }
            zwlr_output_configuration_v1::Request::DisableHead { head } => {
                let mut data = pending_data.inner.lock().unwrap();

                if data.cancelled {
                    return;
                }

                if data.used {
                    resource.post_error(
                        zwlr_output_configuration_v1::Error::AlreadyUsed,
                        "configuration has already been applied or tested",
                    );
                    return;
                }

                let manager_serial =
                    manager_for_configuration(state, resource).map(|(_, data)| data.serial);

                if manager_serial != Some(pending_data.serial) {
                    resource.cancelled();
                    data.cancelled = true;
                    return;
                }

                if let Some(pending_data) = data.pending_heads.get_mut(&head) {
                    if !matches!(pending_data, PendingHead::NotConfigured) {
                        head.post_error(
                            zwlr_output_configuration_v1::Error::AlreadyConfiguredHead,
                            "head has already been configured",
                        );
                        return;
                    }

                    *pending_data = PendingHead::Disabled;
                }
            }
            req @ (zwlr_output_configuration_v1::Request::Apply
            | zwlr_output_configuration_v1::Request::Test) => {
                let mut data = pending_data.inner.lock().unwrap();

                if data.cancelled {
                    return;
                }

                if data.used {
                    resource.post_error(
                        zwlr_output_configuration_v1::Error::AlreadyUsed,
                        "configuration has already been applied or tested",
                    );
                    return;
                }

                let manager_serial =
                    manager_for_configuration(state, resource).map(|(_, data)| data.serial);

                if manager_serial != Some(pending_data.serial) {
                    resource.cancelled();
                    data.cancelled = true;
                    return;
                }

                if data
                    .pending_heads
                    .values()
                    .any(|cfg| matches!(cfg, PendingHead::NotConfigured))
                {
                    resource.post_error(
                        zwlr_output_configuration_v1::Error::UnconfiguredHead,
                        "a head was unconfigured",
                    );
                    return;
                }

                #[allow(clippy::mutable_key_type)]
                let config = data
                    .pending_heads
                    .iter()
                    .map(|(head, head_cfg)| {
                        let output = head.data::<Output>().unwrap().clone();

                        let cfg = match head_cfg {
                            PendingHead::NotConfigured => unreachable!(),
                            PendingHead::Enabled(cfg_head) => {
                                let pending = cfg_head
                                    .data::<Mutex<PendingOutputHeadConfiguration>>()
                                    .unwrap()
                                    .lock()
                                    .unwrap();
                                OutputConfiguration::Enabled {
                                    mode: pending.mode,
                                    position: pending.position,
                                    transform: pending.transform,
                                    scale: pending.scale,
                                    adaptive_sync: pending.adaptive_sync,
                                }
                            }
                            PendingHead::Disabled => OutputConfiguration::Disabled,
                        };

                        (output, cfg)
                    })
                    .collect();

                let apply = matches!(req, zwlr_output_configuration_v1::Request::Apply);
                let success = if apply {
                    state.apply_configuration(config)
                } else {
                    state.test_configuration(config)
                };

                if success {
                    resource.succeeded();
                } else {
                    resource.failed();
                }

                data.used = true;
            }
            zwlr_output_configuration_v1::Request::Destroy => (),
            _ => unreachable!(),
        }
    }

    fn destroyed(
        state: &mut D,
        _client: ClientId,
        resource: &ZwlrOutputConfigurationV1,
        _data: &PendingOutputConfiguration,
    ) {
        for output_manager_data in state
            .output_management_manager_state()
            .managers
            .values_mut()
        {
            output_manager_data
                .configurations
                .retain(|config| config != resource);
        }
    }
}

impl<D> Dispatch<ZwlrOutputConfigurationHeadV1, Mutex<PendingOutputHeadConfiguration>, D>
    for OutputManagementManagerState
where
    D: Dispatch<ZwlrOutputModeV1, Mode> + 'static,
{
    fn request(
        _state: &mut D,
        _client: &Client,
        resource: &ZwlrOutputConfigurationHeadV1,
        request: <ZwlrOutputConfigurationHeadV1 as Resource>::Request,
        data: &Mutex<PendingOutputHeadConfiguration>,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            zwlr_output_configuration_head_v1::Request::SetMode { mode } => {
                let mut data = data.lock().unwrap();
                if data.mode.is_some() {
                    resource.post_error(
                        zwlr_output_configuration_head_v1::Error::AlreadySet,
                        "mode has already been set",
                    );
                    return;
                }

                let mode = mode.data::<Mode>().unwrap();

                let mode = (mode.size, NonZeroU32::new(mode.refresh as u32));

                data.mode = Some(mode);
            }
            zwlr_output_configuration_head_v1::Request::SetCustomMode {
                width,
                height,
                refresh,
            } => {
                let mut data = data.lock().unwrap();
                if data.mode.is_some() {
                    resource.post_error(
                        zwlr_output_configuration_head_v1::Error::AlreadySet,
                        "mode has already been set",
                    );
                    return;
                }

                if width <= 0 || height <= 0 || refresh < 0 {
                    resource.post_error(
                        zwlr_output_configuration_head_v1::Error::InvalidCustomMode,
                        "invalid custom mode",
                    );
                    return;
                }

                data.mode = Some(((width, height).into(), NonZeroU32::new(refresh as u32)));
            }
            zwlr_output_configuration_head_v1::Request::SetPosition { x, y } => {
                let mut data = data.lock().unwrap();
                if data.position.is_some() {
                    resource.post_error(
                        zwlr_output_configuration_head_v1::Error::AlreadySet,
                        "position has already been set",
                    );
                    return;
                }

                data.position = Some((x, y).into());
            }
            zwlr_output_configuration_head_v1::Request::SetTransform { transform } => {
                let mut data = data.lock().unwrap();
                if data.transform.is_some() {
                    resource.post_error(
                        zwlr_output_configuration_head_v1::Error::AlreadySet,
                        "transform has already been set",
                    );
                    return;
                }

                let transform = match transform {
                    WEnum::Value(transform) => transform,
                    WEnum::Unknown(val) => {
                        resource.post_error(
                            zwlr_output_configuration_head_v1::Error::InvalidTransform,
                            format!("transform has an invalid value of {val}"),
                        );
                        return;
                    }
                };

                data.transform = Some(transform.into());
            }
            zwlr_output_configuration_head_v1::Request::SetScale { scale } => {
                let mut data = data.lock().unwrap();
                if data.scale.is_some() {
                    resource.post_error(
                        zwlr_output_configuration_head_v1::Error::AlreadySet,
                        "scale has already been set",
                    );
                    return;
                }

                data.scale = Some(scale);
            }
            zwlr_output_configuration_head_v1::Request::SetAdaptiveSync { state } => {
                let mut data = data.lock().unwrap();
                if data.adaptive_sync.is_some() {
                    resource.post_error(
                        zwlr_output_configuration_head_v1::Error::AlreadySet,
                        "adaptive sync has already been set",
                    );
                    return;
                }

                let adaptive_sync = match state {
                    WEnum::Value(adaptive_sync) => match adaptive_sync {
                        AdaptiveSyncState::Disabled => false,
                        AdaptiveSyncState::Enabled => true,
                        _ => unreachable!(),
                    },
                    WEnum::Unknown(val) => {
                        resource.post_error(
                            zwlr_output_configuration_head_v1::Error::InvalidAdaptiveSyncState,
                            format!("adaptive sync has an invalid value of {val}"),
                        );
                        return;
                    }
                };

                data.adaptive_sync = Some(adaptive_sync);
            }
            _ => unreachable!(),
        }
    }
}

#[macro_export]
macro_rules! delegate_output_management {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        smithay::reexports::wayland_server::delegate_global_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::output_management::v1::server::zwlr_output_manager_v1::ZwlrOutputManagerV1: $crate::protocol::output_management::OutputManagementGlobalData
        ] => $crate::protocol::output_management::OutputManagementManagerState);

        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::output_management::v1::server::zwlr_output_manager_v1::ZwlrOutputManagerV1: ()
        ] => $crate::protocol::output_management::OutputManagementManagerState);

        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::output_management::v1::server::zwlr_output_head_v1::ZwlrOutputHeadV1: smithay::output::Output
        ] => $crate::protocol::output_management::OutputManagementManagerState);

        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::output_management::v1::server::zwlr_output_mode_v1::ZwlrOutputModeV1: smithay::output::Mode
        ] => $crate::protocol::output_management::OutputManagementManagerState);

        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::output_management::v1::server::zwlr_output_configuration_v1::ZwlrOutputConfigurationV1: $crate::protocol::output_management::PendingOutputConfiguration
        ] => $crate::protocol::output_management::OutputManagementManagerState);

        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::output_management::v1::server::zwlr_output_configuration_head_v1::ZwlrOutputConfigurationHeadV1: std::sync::Mutex<$crate::protocol::output_management::PendingOutputHeadConfiguration>
        ] => $crate::protocol::output_management::OutputManagementManagerState);
    };
}
