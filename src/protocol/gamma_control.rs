use std::{collections::HashMap, fs::File, io::Read};

use smithay::{
    output::Output,
    reexports::{
        wayland_protocols_wlr::gamma_control::v1::server::{
            zwlr_gamma_control_manager_v1::{self, ZwlrGammaControlManagerV1},
            zwlr_gamma_control_v1::{self, ZwlrGammaControlV1},
        },
        wayland_server::{
            self, backend::ClientId, Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch,
            Resource,
        },
    },
};
use tracing::warn;

const VERSION: u32 = 1;

pub struct GammaControlManagerState {
    pub gamma_controls: HashMap<Output, ZwlrGammaControlV1>,
}

pub struct GammaControlManagerGlobalData {
    filter: Box<dyn Fn(&Client) -> bool + Send + Sync>,
}

impl GammaControlManagerState {
    pub fn new<D, F>(display: &DisplayHandle, filter: F) -> Self
    where
        D: GlobalDispatch<ZwlrGammaControlManagerV1, GammaControlManagerGlobalData>
            + Dispatch<ZwlrGammaControlManagerV1, ()>
            + Dispatch<ZwlrGammaControlV1, GammaControlState>
            + GammaControlHandler
            + 'static,
        F: Fn(&Client) -> bool + Send + Sync + 'static,
    {
        let global_data = GammaControlManagerGlobalData {
            filter: Box::new(filter),
        };
        display.create_global::<D, ZwlrGammaControlManagerV1, _>(VERSION, global_data);
        Self {
            gamma_controls: HashMap::new(),
        }
    }

    pub fn output_removed(&mut self, output: &Output) {
        if let Some(gamma_control) = self.gamma_controls.remove(output) {
            gamma_control.failed();
        }
    }
}

pub struct GammaControlState {
    gamma_size: u32,
}

impl<D> GlobalDispatch<ZwlrGammaControlManagerV1, GammaControlManagerGlobalData, D>
    for GammaControlManagerState
where
    D: GlobalDispatch<ZwlrGammaControlManagerV1, GammaControlManagerGlobalData>
        + Dispatch<ZwlrGammaControlManagerV1, ()>
        + Dispatch<ZwlrGammaControlV1, GammaControlState>
        + GammaControlHandler
        + 'static,
{
    fn bind(
        _state: &mut D,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: wayland_server::New<ZwlrGammaControlManagerV1>,
        _global_data: &GammaControlManagerGlobalData,
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(resource, ());
    }

    fn can_view(client: Client, global_data: &GammaControlManagerGlobalData) -> bool {
        (global_data.filter)(&client)
    }
}

impl<D> Dispatch<ZwlrGammaControlManagerV1, (), D> for GammaControlManagerState
where
    D: Dispatch<ZwlrGammaControlManagerV1, ()>
        + Dispatch<ZwlrGammaControlV1, GammaControlState>
        + GammaControlHandler
        + 'static,
{
    fn request(
        state: &mut D,
        _client: &Client,
        _manager: &ZwlrGammaControlManagerV1,
        request: <ZwlrGammaControlManagerV1 as wayland_server::Resource>::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        let (id, output) = match request {
            zwlr_gamma_control_manager_v1::Request::GetGammaControl { id, output } => (id, output),
            zwlr_gamma_control_manager_v1::Request::Destroy => return,
            _ => unreachable!(),
        };

        let output = Output::from_resource(&output).expect("no output for resource");

        match state
            .gamma_control_manager_state()
            .gamma_controls
            .contains_key(&output)
        {
            true => {
                // This wl_output already has exclusive access by another client
                let gamma_control_state = GammaControlState { gamma_size: 0 };
                let gamma_control = data_init.init(id, gamma_control_state);
                gamma_control.failed();
            }
            false => {
                let Some(gamma_size) = state.get_gamma_size(&output) else {
                    let gamma_control = data_init.init(id, GammaControlState { gamma_size: 0 });
                    gamma_control.failed();
                    return;
                };

                let gamma_control = data_init.init(id, GammaControlState { gamma_size });

                gamma_control.gamma_size(gamma_size);
                state
                    .gamma_control_manager_state()
                    .gamma_controls
                    .insert(output, gamma_control);
            }
        }
    }
}

pub trait GammaControlHandler {
    fn gamma_control_manager_state(&mut self) -> &mut GammaControlManagerState;
    /// A new gamma control was requested on the given output.
    ///
    /// This should return the length of the gamma on the output, if available.
    fn get_gamma_size(&mut self, output: &Output) -> Option<u32>;
    /// A client requested that the gamma be set on the given output.
    ///
    /// `gammas` are the gammas for the red, green, and blue channels respectively.
    ///
    /// Returns whether or not the operation completed successfully.
    fn set_gamma(&mut self, output: &Output, gammas: [&[u16]; 3]) -> bool;
    /// A client destroyed its gamma control object for the given output.
    fn gamma_control_destroyed(&mut self, output: &Output);
}

#[allow(missing_docs)]
#[macro_export]
macro_rules! delegate_gamma_control {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        smithay::reexports::wayland_server::delegate_global_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::gamma_control::v1::server::zwlr_gamma_control_manager_v1::ZwlrGammaControlManagerV1: $crate::protocol::gamma_control::GammaControlManagerGlobalData
        ] => $crate::protocol::gamma_control::GammaControlManagerState);

        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::gamma_control::v1::server::zwlr_gamma_control_manager_v1::ZwlrGammaControlManagerV1: ()
        ] => $crate::protocol::gamma_control::GammaControlManagerState);

        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::gamma_control::v1::server::zwlr_gamma_control_v1::ZwlrGammaControlV1: $crate::protocol::gamma_control::GammaControlState
        ] => $crate::protocol::gamma_control::GammaControlManagerState);
    };
}

impl<D> Dispatch<ZwlrGammaControlV1, GammaControlState, D> for GammaControlManagerState
where
    D: Dispatch<ZwlrGammaControlV1, GammaControlState> + GammaControlHandler + 'static,
{
    fn request(
        state: &mut D,
        _client: &Client,
        resource: &ZwlrGammaControlV1,
        request: <ZwlrGammaControlV1 as Resource>::Request,
        data: &GammaControlState,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        if matches!(request, zwlr_gamma_control_v1::Request::Destroy) {
            return;
        }

        let Some(output) = state
            .gamma_control_manager_state()
            .gamma_controls
            .iter()
            .find(|(_, res)| *res == resource)
            .map(|(output, _)| output)
            .cloned()
        else {
            resource.failed();
            return;
        };

        let GammaControlState { gamma_size } = data;

        let gamma_size = *gamma_size as usize;

        let fd = match request {
            zwlr_gamma_control_v1::Request::SetGamma { fd } => fd,
            zwlr_gamma_control_v1::Request::Destroy => return,
            _ => unreachable!(),
        };

        let mut gammas = vec![0u16; gamma_size * 3];

        {
            let buf = bytemuck::cast_slice_mut(&mut gammas);

            let mut file = File::from(fd);

            let gamma_controls = &mut state.gamma_control_manager_state().gamma_controls;

            if let Err(err) = file.read_exact(buf) {
                warn!(
                    "Failed to read {} u16s from client gamma control fd: {err}",
                    gamma_size * 3
                );
                resource.failed();
                gamma_controls.remove(&output);
                state.gamma_control_destroyed(&output);
                return;
            }

            #[allow(clippy::unused_io_amount)]
            {
                match file.read(&mut [0]) {
                    Ok(0) => (),
                    Ok(_) => {
                        warn!(
                            "Client gamma control sent more data than expected (expected {} u16s)",
                            gamma_size * 3,
                        );
                        resource.failed();
                        gamma_controls.remove(&output);
                        state.gamma_control_destroyed(&output);
                        return;
                    }
                    Err(err) => {
                        warn!(
                            "Failed to ensure client gamma control fd was the correct size: {err}"
                        );
                        resource.failed();
                        gamma_controls.remove(&output);
                        state.gamma_control_destroyed(&output);
                        return;
                    }
                }
            }
        }

        assert_eq!(gammas.len(), gamma_size * 3);

        let gammas = gammas.chunks_exact(gamma_size).collect::<Vec<_>>();
        let [red_gamma, green_gamma, blue_gamma] = gammas.as_slice() else {
            unreachable!();
        };

        if !state.set_gamma(&output, [red_gamma, green_gamma, blue_gamma]) {
            resource.failed();
            state
                .gamma_control_manager_state()
                .gamma_controls
                .remove(&output);
            state.gamma_control_destroyed(&output);
        }
    }

    fn destroyed(
        state: &mut D,
        _client: ClientId,
        resource: &ZwlrGammaControlV1,
        _data: &GammaControlState,
    ) {
        let gamma_controls = &mut state.gamma_control_manager_state().gamma_controls;

        let Some(output) = gamma_controls
            .iter()
            .find(|(_, res)| *res == resource)
            .map(|(output, _)| output)
            .cloned()
        else {
            return;
        };

        gamma_controls.remove(&output);

        state.gamma_control_destroyed(&output);
    }
}
