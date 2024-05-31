use smithay::reexports::drm::control::{property, Device};
use smithay::{backend::drm::DrmDevice, reexports::drm::control::crtc};
use tracing::warn;
use util::get_drm_property;

pub mod edid_manus;
pub mod util;

const DRM_CRTC_ACTIVE: &str = "ACTIVE";

pub(super) fn set_crtc_active(device: &DrmDevice, crtc: crtc::Handle, active: bool) {
    let prop = match get_drm_property(device, crtc, DRM_CRTC_ACTIVE) {
        Ok(prop) => prop,
        Err(err) => {
            warn!("Failed to get crtc ACTIVE property: {err}");
            return;
        }
    };

    let value = property::Value::Boolean(active);
    if let Err(err) = device.set_property(crtc, prop, value.into()) {
        warn!("Failed to set crtc ACTIVE to {active}: {err}");
    }
}
