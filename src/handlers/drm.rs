use smithay::{backend::allocator::dmabuf::Dmabuf, wayland::dmabuf::DmabufGlobal};

use crate::{
    protocol::drm::{DrmHandler, ImportError, delegate_wl_drm},
    state::State,
};

impl DrmHandler for State {
    fn dmabuf_imported(
        &mut self,
        _global: &DmabufGlobal,
        dmabuf: Dmabuf,
    ) -> Result<(), ImportError> {
        self.backend
            .dmabuf_imported(dmabuf)
            .map_err(|_| ImportError::Failed)
    }
}
delegate_wl_drm!(State);
