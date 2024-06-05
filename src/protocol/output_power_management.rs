use std::collections::HashMap;

use smithay::{
    output::Output,
    reexports::{
        wayland_protocols_wlr::output_power_management::v1::server::{
            zwlr_output_power_manager_v1::{self, ZwlrOutputPowerManagerV1},
            zwlr_output_power_v1::{self, ZwlrOutputPowerV1},
        },
        wayland_server::{
            self, backend::ClientId, Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch,
            Resource, WEnum,
        },
    },
};
use tracing::warn;

use crate::state::WithState;

const VERSION: u32 = 1;

pub struct OutputPowerManagementState {
    clients: HashMap<Output, ZwlrOutputPowerV1>,
}

pub struct OutputPowerManagementGlobalData {
    filter: Box<dyn Fn(&Client) -> bool + Send + Sync + 'static>,
}

pub trait OutputPowerManagementHandler {
    fn output_power_management_state(&mut self) -> &mut OutputPowerManagementState;
    fn set_mode(&mut self, output: &Output, powered: bool);
}

impl OutputPowerManagementState {
    pub fn new<D, F>(display: &DisplayHandle, filter: F) -> Self
    where
        D: GlobalDispatch<ZwlrOutputPowerManagerV1, OutputPowerManagementGlobalData> + 'static,
        F: Fn(&Client) -> bool + Send + Sync + 'static,
    {
        let data = OutputPowerManagementGlobalData {
            filter: Box::new(filter),
        };

        display.create_global::<D, ZwlrOutputPowerManagerV1, _>(VERSION, data);

        Self {
            clients: HashMap::new(),
        }
    }

    pub fn output_removed(&mut self, output: &Output) {
        if let Some(power) = self.clients.remove(output) {
            power.failed();
        }
    }
}

impl<D> GlobalDispatch<ZwlrOutputPowerManagerV1, OutputPowerManagementGlobalData, D>
    for OutputPowerManagementState
where
    D: Dispatch<ZwlrOutputPowerManagerV1, ()> + OutputPowerManagementHandler,
{
    fn bind(
        _state: &mut D,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: wayland_server::New<ZwlrOutputPowerManagerV1>,
        _global_data: &OutputPowerManagementGlobalData,
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(resource, ());
    }

    fn can_view(client: Client, global_data: &OutputPowerManagementGlobalData) -> bool {
        (global_data.filter)(&client)
    }
}

impl<D> Dispatch<ZwlrOutputPowerManagerV1, (), D> for OutputPowerManagementState
where
    D: Dispatch<ZwlrOutputPowerV1, ()> + OutputPowerManagementHandler,
{
    fn request(
        state: &mut D,
        _client: &Client,
        _resource: &ZwlrOutputPowerManagerV1,
        request: <ZwlrOutputPowerManagerV1 as wayland_server::Resource>::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            zwlr_output_power_manager_v1::Request::GetOutputPower { id, output } => {
                let Some(output) = Output::from_resource(&output) else {
                    warn!("wlr-output-power-management: no output for wl_output {output:?}");
                    let power = data_init.init(id, ());
                    power.failed();
                    return;
                };

                if state
                    .output_power_management_state()
                    .clients
                    .contains_key(&output)
                {
                    warn!(
                        "wlr-output-power-management: {} already has an active power manager",
                        output.name()
                    );
                    let power = data_init.init(id, ());
                    power.failed();
                    return;
                }

                let power = data_init.init(id, ());
                let is_powered = output.with_state(|state| state.powered);
                power.mode(match is_powered {
                    true => zwlr_output_power_v1::Mode::On,
                    false => zwlr_output_power_v1::Mode::Off,
                });

                state
                    .output_power_management_state()
                    .clients
                    .insert(output, power);
            }
            zwlr_output_power_manager_v1::Request::Destroy => (),
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch<ZwlrOutputPowerV1, (), D> for OutputPowerManagementState
where
    D: Dispatch<ZwlrOutputPowerV1, ()> + OutputPowerManagementHandler,
{
    fn request(
        state: &mut D,
        _client: &Client,
        resource: &ZwlrOutputPowerV1,
        request: <ZwlrOutputPowerV1 as wayland_server::Resource>::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            zwlr_output_power_v1::Request::SetMode { mode } => {
                let Some(output) = state
                    .output_power_management_state()
                    .clients
                    .iter()
                    .find_map(|(output, power)| (power == resource).then_some(output.clone()))
                else {
                    return;
                };

                state.set_mode(
                    &output,
                    match mode {
                        WEnum::Value(zwlr_output_power_v1::Mode::On) => true,
                        WEnum::Value(zwlr_output_power_v1::Mode::Off) => false,
                        mode => {
                            resource.post_error(
                                zwlr_output_power_v1::Error::InvalidMode,
                                format!("invalid mode {mode:?}"),
                            );
                            return;
                        }
                    },
                );
            }
            zwlr_output_power_v1::Request::Destroy => {
                state
                    .output_power_management_state()
                    .clients
                    .retain(|_, power| power == resource);
            }
            _ => todo!(),
        }
    }

    fn destroyed(state: &mut D, _client: ClientId, resource: &ZwlrOutputPowerV1, _data: &()) {
        state
            .output_power_management_state()
            .clients
            .retain(|_, power| power == resource);
    }
}

#[macro_export]
macro_rules! delegate_output_power_management {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        smithay::reexports::wayland_server::delegate_global_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::output_power_management::v1::server::zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1: $crate::protocol::output_power_management::OutputPowerManagementGlobalData
        ] => $crate::protocol::output_power_management::OutputPowerManagementState);

        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::output_power_management::v1::server::zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1: ()
        ] => $crate::protocol::output_power_management::OutputPowerManagementState);

        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::output_power_management::v1::server::zwlr_output_power_v1::ZwlrOutputPowerV1: ()
        ] => $crate::protocol::output_power_management::OutputPowerManagementState);
    };
}
