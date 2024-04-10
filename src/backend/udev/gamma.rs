// Parts ripped out from Niri like that time Omni-man almost ripped out Donald's spine
// Not that I'm Omni-man, of course.

use anyhow::{ensure, Context};
use smithay::output::Output;
use smithay::reexports::drm::control::Device;

use super::{Udev, UdevOutputData};

impl Udev {
    // TODO: gamma sets when session is inactive
    pub fn set_gamma(&self, output: &Output, gamma: Option<[&[u16]; 3]>) -> anyhow::Result<()> {
        let UdevOutputData { device_id, crtc } = output
            .user_data()
            .get()
            .context("no udev output data for output")?;

        let drm_device = &self
            .backends
            .get(device_id)
            .context("no udev backend data for output")?
            .drm;

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

        let drm_device = &self
            .backends
            .get(device_id)
            .context("no udev backend data for output")?
            .drm;

        let crtc_info = drm_device.get_crtc(*crtc)?;
        Ok(crtc_info.gamma_length())
    }
}
