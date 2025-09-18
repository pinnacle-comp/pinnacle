use smithay::output::Output;

use crate::delegate_ext_workspace;
use crate::output::OutputName;
use crate::protocol::ext_workspace::{ExtWorkspaceHandler, ExtWorkspaceManagerState};
use crate::state::State;
use crate::tag::TagId;

impl ExtWorkspaceHandler for State {
    fn ext_workspace_manager_state(&mut self) -> &mut ExtWorkspaceManagerState {
        &mut self.pinnacle.ext_workspace_state
    }

    fn activate_workspace(&mut self, id: TagId) {
        if let Some(tag) = id.tag(&self.pinnacle) {
            crate::api::tag::switch_to(self, &tag);
        }
    }

    fn assign_workspace(&mut self, id: TagId, output: Output) {
        if let Some(tag) = id.tag(&self.pinnacle) {
            crate::api::tag::add(self, Some(tag.name()), OutputName(output.name()));
        }
    }
}

delegate_ext_workspace!(State);
