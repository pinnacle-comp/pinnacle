use std::collections::HashMap;

use smithay::{
    backend::{
        allocator::gbm::{GbmAllocator, GbmDevice},
        drm::{compositor::DrmCompositor, DrmDeviceFd, DrmNode, GbmBufferedSurface},
        renderer::{
            damage::OutputDamageTracker,
            gles::GlesRenderer,
            multigpu::{gbm::GbmGlesBackend, GpuManager},
        },
        session::libseat::LibSeatSession,
    },
    desktop::utils::OutputPresentationFeedback,
    output::Output,
    reexports::{
        drm::control::crtc,
        wayland_server::{backend::GlobalId, protocol::wl_surface::WlSurface, DisplayHandle},
    },
};

use crate::state::State;

use super::Backend;

pub struct UdevData {
    session: LibSeatSession,
    display_handle: DisplayHandle,
    primary_gpu: DrmNode,
    gpu_manager: GpuManager<GbmGlesBackend<GlesRenderer>>,
    backends: HashMap<DrmNode, BackendData>,
}

impl Backend for UdevData {
    fn seat_name(&self) -> String {
        todo!()
    }

    fn reset_buffers(&mut self, output: &Output) {
        todo!()
    }

    fn early_import(&mut self, surface: &WlSurface) {
        todo!()
    }
}

struct BackendData {
    surfaces: HashMap<crtc::Handle, SurfaceData>,
    gbm_device: GbmDevice<DrmDeviceFd>,
}

struct SurfaceData {
    global: Option<GlobalId>,
    display_handle: DisplayHandle,
    device_id: DrmNode,
    render_node: DrmNode,
    compositor: SurfaceComposition,
    // TODO: dmabuf_feedback
}

impl Drop for SurfaceData {
    fn drop(&mut self) {
        if let Some(global) = self.global.take() {
            self.display_handle.remove_global::<State<UdevData>>(global);
        }
    }
}

type RenderSurface =
    GbmBufferedSurface<GbmAllocator<DrmDeviceFd>, Option<OutputPresentationFeedback>>;

type GbmDrmCompositor = DrmCompositor<
    GbmAllocator<DrmDeviceFd>,
    GbmDevice<DrmDeviceFd>,
    Option<OutputPresentationFeedback>,
    DrmDeviceFd,
>;

enum SurfaceComposition {
    Surface {
        surface: RenderSurface,
        damage_tracker: OutputDamageTracker,
    },
    Compositor(GbmDrmCompositor),
}
