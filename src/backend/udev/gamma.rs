// Parts ripped out from Niri like that time Omni-man almost ripped out Donald's spine
// Not that I'm Omni-man, of course.

use anyhow::{Context, ensure};
use smithay::backend::drm::DrmDevice;
use smithay::reexports::drm::control::{Device, crtc};
use smithay::{backend::session::Session, output::Output};

use crate::backend::udev::{PendingGammaChange, render_surface_for_output};

use super::{Udev, UdevOutputData};

impl Udev {
    pub fn set_gamma(&mut self, output: &Output, gamma: Option<[&[u16]; 3]>) -> anyhow::Result<()> {
        if !self.session.is_active() {
            render_surface_for_output(output, &mut self.devices)
                .context("no render surface for output")?
                .pending_gamma_change = match gamma {
                Some([r, g, b]) => {
                    PendingGammaChange::Change([Box::from(r), Box::from(g), Box::from(b)])
                }
                None => PendingGammaChange::Restore,
            };
            return Ok(());
        }

        let UdevOutputData { device_id, crtc } = output
            .user_data()
            .get()
            .context("no udev output data for output")?;

        let drm_device = self
            .devices
            .get(device_id)
            .context("no udev backend data for output")?
            .drm_output_manager
            .device();

        let ret = Udev::set_gamma_internal(drm_device, crtc, gamma);

        render_surface_for_output(output, &mut self.devices)
            .context("no render surface for output")?
            .previous_gamma = match ret.is_ok() {
            true => gamma.map(|[r, g, b]| [r.into(), g.into(), b.into()]),
            false => None,
        };

        ret
    }

    pub(super) fn set_gamma_internal(
        drm_device: &DrmDevice,
        crtc: &crtc::Handle,
        gamma: Option<[impl AsRef<[u16]>; 3]>,
    ) -> anyhow::Result<()> {
        let gamma = gamma
            .as_ref()
            .map(|[r, g, b]| [r.as_ref(), g.as_ref(), b.as_ref()]);

        let crtc_info = drm_device.get_crtc(*crtc)?;
        let gamma_size = crtc_info.gamma_length() as usize;

        ensure!(gamma_size != 0, "setting gamma is not supported");

        let mut temp_red;
        let mut temp_green;
        let mut temp_blue;

        let [red, green, blue] = match gamma {
            Some([red, green, blue]) => {
                ensure!(red.len() == gamma_size, "wrong red gamma size");
                ensure!(green.len() == gamma_size, "wrong green gamma size");
                ensure!(blue.len() == gamma_size, "wrong blue gamma size");
                [red, green, blue]
            }
            None => {
                temp_red = vec![0u16; gamma_size];
                temp_green = vec![0u16; gamma_size];
                temp_blue = vec![0u16; gamma_size];

                let denom = gamma_size as u64 - 1;

                for i in 0..gamma_size {
                    let value = (0xFFFF * i as u64 / denom) as u16;
                    temp_red[i] = value;
                    temp_green[i] = value;
                    temp_blue[i] = value;
                }

                [
                    temp_red.as_slice(),
                    temp_green.as_slice(),
                    temp_blue.as_slice(),
                ]
            }
        };

        drm_device.set_gamma(*crtc, red, green, blue)?;

        Ok(())
    }

    pub fn gamma_size(&self, output: &Output) -> anyhow::Result<u32> {
        let UdevOutputData { device_id, crtc } = output
            .user_data()
            .get()
            .context("no udev output data for output")?;

        let drm_device = self
            .devices
            .get(device_id)
            .context("no udev backend data for output")?
            .drm_output_manager
            .device();

        let crtc_info = drm_device.get_crtc(*crtc)?;
        Ok(crtc_info.gamma_length())
    }
}
