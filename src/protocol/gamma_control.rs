use std::{collections::HashMap, fs::File, io::Read, ops::Deref};

use smithay::{
    output::{Output, WeakOutput},
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
    gamma_controls: HashMap<WeakOutput, GammaControl>,
}

struct GammaControl {
    control: ZwlrGammaControlV1,
    destroyed: bool,
}

impl Deref for GammaControl {
    type Target = ZwlrGammaControlV1;

    fn deref(&self) -> &Self::Target {
        &self.control
    }
}

impl Drop for GammaControl {
    fn drop(&mut self) {
        if !self.destroyed {
            self.control.failed();
        }
    }
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
        self.gamma_controls.remove(&output.downgrade());
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

        let Some(output) = Output::from_resource(&output) else {
            warn!("wlr-gamma-control: no output for wl_output {output:?}");
            let gamma_control = data_init.init(id, GammaControlState { gamma_size: 0 });
            gamma_control.failed();
            return;
        };

        match state
            .gamma_control_manager_state()
            .gamma_controls
            .contains_key(&output.downgrade())
        {
            true => {
                // This wl_output already has exclusive access by another client
                let gamma_control = data_init.init(id, GammaControlState { gamma_size: 0 });
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

                state.gamma_control_manager_state().gamma_controls.insert(
                    output.downgrade(),
                    GammaControl {
                        control: gamma_control,
                        destroyed: false,
                    },
                );
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
            .find_map(|(output, res)| (res.control == *resource).then_some(output.clone()))
        else {
            resource.failed();
            return;
        };

        let Some(output) = output.upgrade() else {
            state
                .gamma_control_manager_state()
                .gamma_controls
                .remove(&output);
            return;
        };

        let GammaControlState { gamma_size } = data;

        let gamma_size = *gamma_size as usize;

        let fd = match request {
            zwlr_gamma_control_v1::Request::SetGamma { fd } => fd,
            zwlr_gamma_control_v1::Request::Destroy => unreachable!(),
            _ => return,
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
                gamma_controls.remove(&output.downgrade());
                state.gamma_control_destroyed(&output);
                return;
            }

            match file.read(&mut [0]) {
                Ok(0) => (),
                Ok(_) => {
                    warn!(
                        "Client gamma control sent more data than expected (expected {} u16s)",
                        gamma_size * 3,
                    );
                    gamma_controls.remove(&output.downgrade());
                    state.gamma_control_destroyed(&output);
                    return;
                }
                Err(err) => {
                    warn!("Failed to ensure client gamma control fd was the correct size: {err}");
                    gamma_controls.remove(&output.downgrade());
                    state.gamma_control_destroyed(&output);
                    return;
                }
            }
        }

        assert_eq!(gammas.len(), gamma_size * 3);

        let gammas = gammas.chunks_exact(gamma_size).collect::<Vec<_>>();
        let [red_gamma, green_gamma, blue_gamma] = gammas.as_slice() else {
            unreachable!();
        };

        if !state.set_gamma(&output, [red_gamma, green_gamma, blue_gamma]) {
            state
                .gamma_control_manager_state()
                .gamma_controls
                .remove(&output.downgrade());
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
            .find_map(|(output, res)| (res.control == *resource).then_some(output.clone()))
        else {
            return;
        };

        if let Some(mut control) = gamma_controls.remove(&output) {
            // Inhibit sending failed on drop for destroyed controls
            control.destroyed = true;
        }

        if let Some(output) = output.upgrade() {
            state.gamma_control_destroyed(&output);
        }
    }
}
