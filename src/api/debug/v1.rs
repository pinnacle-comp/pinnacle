use pinnacle_api_defs::pinnacle::{
    debug::{
        self,
        v1::{
            SetCursorPlaneScanoutRequest, SetDamageVisualizationRequest,
            SetOpaqueRegionVisualizationRequest, SetProcessPipingRequest,
        },
    },
    util::v1::SetOrToggle,
};
use tonic::{Request, Status};

use crate::api::{run_unary_no_response, TonicResult};

#[tonic::async_trait]
impl debug::v1::debug_service_server::DebugService for super::DebugService {
    async fn set_damage_visualization(
        &self,
        request: Request<SetDamageVisualizationRequest>,
    ) -> TonicResult<()> {
        let request = request.into_inner();
        let set_or_toggle = request.set_or_toggle();

        let set = match set_or_toggle {
            SetOrToggle::Set => Some(true),
            SetOrToggle::Unset => Some(false),
            SetOrToggle::Toggle => None,
            SetOrToggle::Unspecified => {
                return Err(Status::invalid_argument("no set or toggle specified"))
            }
        };

        run_unary_no_response(&self.sender, move |state| {
            state.pinnacle.config.debug.visualize_damage =
                set.unwrap_or(!state.pinnacle.config.debug.visualize_damage);
            tracing::debug!(
                "Damage visualization: {}",
                state.pinnacle.config.debug.visualize_damage
            );
        })
        .await
    }

    async fn set_opaque_region_visualization(
        &self,
        request: Request<SetOpaqueRegionVisualizationRequest>,
    ) -> TonicResult<()> {
        let request = request.into_inner();
        let set_or_toggle = request.set_or_toggle();

        let set = match set_or_toggle {
            SetOrToggle::Set => Some(true),
            SetOrToggle::Unset => Some(false),
            SetOrToggle::Toggle => None,
            SetOrToggle::Unspecified => {
                return Err(Status::invalid_argument("no set or toggle specified"))
            }
        };

        run_unary_no_response(&self.sender, move |state| {
            state.pinnacle.config.debug.visualize_opaque_regions =
                set.unwrap_or(!state.pinnacle.config.debug.visualize_opaque_regions);
            tracing::debug!(
                "Opaque region visualization: {}",
                state.pinnacle.config.debug.visualize_opaque_regions
            );
        })
        .await
    }

    async fn set_cursor_plane_scanout(
        &self,
        request: Request<SetCursorPlaneScanoutRequest>,
    ) -> TonicResult<()> {
        let request = request.into_inner();
        let set_or_toggle = request.set_or_toggle();

        let set = match set_or_toggle {
            SetOrToggle::Set => Some(true),
            SetOrToggle::Unset => Some(false),
            SetOrToggle::Toggle => None,
            SetOrToggle::Unspecified => {
                return Err(Status::invalid_argument("no set or toggle specified"))
            }
        };

        run_unary_no_response(&self.sender, move |state| {
            state.pinnacle.config.debug.disable_cursor_plane_scanout = set
                .map(|set| !set)
                .unwrap_or(!state.pinnacle.config.debug.disable_cursor_plane_scanout);
            tracing::debug!(
                "Cursor plane scanout: {}",
                !state.pinnacle.config.debug.disable_cursor_plane_scanout
            );
        })
        .await
    }

    async fn set_process_piping(
        &self,
        request: Request<SetProcessPipingRequest>,
    ) -> TonicResult<()> {
        let request = request.into_inner();
        let set_or_toggle = request.set_or_toggle();

        let set = match set_or_toggle {
            SetOrToggle::Set => Some(true),
            SetOrToggle::Unset => Some(false),
            SetOrToggle::Toggle => None,
            SetOrToggle::Unspecified => {
                return Err(Status::invalid_argument("no set or toggle specified"))
            }
        };

        run_unary_no_response(&self.sender, move |state| {
            state.pinnacle.config.debug.disable_process_piping = set
                .map(|set| !set)
                .unwrap_or(!state.pinnacle.config.debug.disable_process_piping);
            tracing::debug!(
                "Process piping: {}",
                !state.pinnacle.config.debug.disable_process_piping
            );
        })
        .await
    }
}
