use crate::delegate_ext_workspace;
use crate::protocol::ext_workspace::{ExtWorkspaceHandler, ExtWorkspaceManagerState};
use crate::state::State;
use crate::tag::TagId;

impl ExtWorkspaceHandler for State {
    fn ext_workspace_manager_state(&mut self) -> &mut ExtWorkspaceManagerState {
        &mut self.pinnacle.ext_workspace_state
    }

    fn activate_workspace(&mut self, id: TagId) {
        if let Some(tag) = id.tag(&self.pinnacle) {
            crate::api::tag::set_active(self, &tag, Some(true));
        }
    }

    fn deactivate_workspace(&mut self, id: TagId) {
        if let Some(tag) = id.tag(&self.pinnacle) {
            crate::api::tag::set_active(self, &tag, Some(false));
        }
    }

    fn remove_workspace(&mut self, id: TagId) {
        crate::api::tag::remove(self, Vec::from_iter(id.tag(&self.pinnacle)));
    }
}

delegate_ext_workspace!(State);
