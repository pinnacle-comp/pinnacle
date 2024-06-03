use std::{ffi::CString, io::Write, mem::MaybeUninit, num::NonZeroU32};

use anyhow::{bail, Context};
use drm_sys::{
    drm_mode_modeinfo, DRM_MODE_FLAG_NHSYNC, DRM_MODE_FLAG_NVSYNC, DRM_MODE_FLAG_PHSYNC,
    DRM_MODE_FLAG_PVSYNC, DRM_MODE_TYPE_USERDEF,
};
use libdisplay_info_sys::cvt::{
    di_cvt_compute, di_cvt_options, di_cvt_reduced_blanking_version_DI_CVT_REDUCED_BLANKING_NONE,
    di_cvt_timing,
};
use smithay::reexports::drm::{
    self,
    control::{connector, property, Device, ResourceHandle},
};

use super::edid_manus::get_manufacturer;

// A bunch of this stuff is from cosmic-comp

#[derive(Debug, Clone)]
pub struct EdidInfo {
    pub model: String,
    pub manufacturer: String,
    pub serial: Option<NonZeroU32>,
}

impl EdidInfo {
    pub fn try_from_connector(
        device: &impl Device,
        connector: connector::Handle,
    ) -> anyhow::Result<Self> {
        let edid_prop = get_drm_property(device, connector, "EDID")?;
        let edid_info = device.get_property(edid_prop)?;

        let mut info = Err(anyhow::anyhow!("No info"));

        let props = device.get_properties(connector)?;
        let (ids, vals) = props.as_props_and_values();
        for (&id, &val) in ids.iter().zip(vals.iter()) {
            if id == edid_prop {
                if let property::Value::Blob(edid_blob) = edid_info.value_type().convert_value(val)
                {
                    let blob = device.get_property_blob(edid_blob)?;
                    info = parse_edid(&blob);
                }
                break;
            }
        }

        info
    }
}

/// Minimally parse the model and manufacturer from the given EDID data buffer.
///
/// `edid-rs` does not properly parse manufacturer ids (it has the order of the id bytes reversed
/// and doesn't add 64 to map the byte to a character), and it additionally
/// fails to parse detailed timing descriptors with an hactive that's divisible by 256
/// (see https://github.com/tuomas56/edid-rs/pull/1).
///
/// Because of this, we're just rolling our own minimal parser instead.
fn parse_edid(buffer: &[u8]) -> anyhow::Result<EdidInfo> {
    // Manufacterer id is bytes 8-9, big endian
    let manu_id = u16::from_be_bytes(buffer[8..=9].try_into()?);

    // Characters are bits 14-10, 9-5, and 4-0.
    // They also map 0b00001..=0b11010 to A..=Z, so add 64 to get the character.
    let char1 = ((manu_id & 0b0111110000000000) >> 10) as u8 + 64;
    let char2 = ((manu_id & 0b0000001111100000) >> 5) as u8 + 64;
    let char3 = (manu_id & 0b0000000000011111) as u8 + 64;

    let manufacturer = get_manufacturer([char1 as char, char2 as char, char3 as char]);

    // INFO: This probably *isn't* completely unique between all monitors
    let serial = u32::from_le_bytes(buffer[12..=15].try_into()?);

    // Monitor names are inside of these display/monitor descriptors at bytes 72..=125.
    // Each descriptor is 18 bytes long.
    let descriptor1 = &buffer[72..=89];
    let descriptor2 = &buffer[90..=107];
    let descriptor3 = &buffer[108..=125];

    let descriptors = [descriptor1, descriptor2, descriptor3];

    let model = descriptors
        .into_iter()
        .find_map(|desc| {
            // The descriptor is a monitor descriptor if its first 2 bytes are 0.
            let is_monitor_descriptor = desc[0..=1] == [0, 0];
            // The descriptor describes a monitor name if it has the tag 0xfc at byte 3.
            let is_monitor_name = desc[3] == 0xfc;

            if is_monitor_descriptor && is_monitor_name {
                // Name is up to 13 bytes at bytes 5..=17 within the descriptor.
                let monitor_name = desc[5..=17]
                    .iter()
                    // Names are terminated with a newline if shorter than 13 bytes.
                    .take_while(|&&byte| byte != b'\n')
                    .map(|&byte| byte as char)
                    .collect::<String>();

                // NOTE: The EDID spec mandates that bytes after the newline are padded with
                // |     spaces (0x20), but we're just gonna ignore that haha

                Some(monitor_name)
            } else {
                None
            }
        })
        .or_else(|| {
            // Get the product code instead.
            // It's at bytes 10..=11, little-endian.
            let product_code = u16::from_le_bytes(buffer[10..=11].try_into().ok()?);
            Some(format!("{product_code:x}"))
        })
        .unwrap_or("Unknown".to_string());

    Ok(EdidInfo {
        model,
        manufacturer,
        serial: NonZeroU32::new(serial),
    })
}

pub(super) fn get_drm_property(
    device: &impl Device,
    handle: impl ResourceHandle,
    name: &str,
) -> anyhow::Result<property::Handle> {
    let props = device
        .get_properties(handle)
        .context("failed to get properties")?;
    let (prop_handles, _) = props.as_props_and_values();
    for prop in prop_handles {
        let info = device.get_property(*prop)?;
        if Some(name) == info.name().to_str().ok() {
            return Ok(*prop);
        }
    }
    anyhow::bail!("No prop found for {}", name)
}

// From https://github.com/swaywm/sway/blob/2e9139df664f1e2dbe14b5df4a9646411b924c66/sway/commands/output/mode.c#L64
fn parse_modeline_string(modeline: &str) -> anyhow::Result<drm_mode_modeinfo> {
    let mut args = modeline.split_whitespace();

    let clock = args
        .next()
        .context("no clock specified")?
        .parse::<u32>()
        .context("failed to parse clock")?
        * 1000;
    let hdisplay = args
        .next()
        .context("no hdisplay specified")?
        .parse()
        .context("failed to parse hdisplay")?;
    let hsync_start = args
        .next()
        .context("no hsync_start specified")?
        .parse()
        .context("failed to parse hsync_start")?;
    let hsync_end = args
        .next()
        .context("no hsync_end specified")?
        .parse()
        .context("failed to parse hsync_end")?;
    let htotal = args
        .next()
        .context("no htotal specified")?
        .parse()
        .context("failed to parse htotal")?;
    let vdisplay = args
        .next()
        .context("no vdisplay specified")?
        .parse()
        .context("failed to parse vdisplay")?;
    let vsync_start = args
        .next()
        .context("no vsync_start specified")?
        .parse()
        .context("failed to parse vsync_start")?;
    let vsync_end = args
        .next()
        .context("no vsync_end specified")?
        .parse()
        .context("failed to parse vsync_end")?;
    let vtotal = args
        .next()
        .context("no vtotal specified")?
        .parse()
        .context("failed to parse vtotal")?;
    let vrefresh = clock * 1000 * 1000 / htotal as u32 / vtotal as u32;

    let mut flags = 0;
    match args.next().context("no +/-hsync specified")? {
        "+hsync" => flags |= DRM_MODE_FLAG_PHSYNC,
        "-hsync" => flags |= DRM_MODE_FLAG_NHSYNC,
        _ => bail!("invalid hsync specifier"),
    };
    match args.next().context("no +/-vsync specified")? {
        "+vsync" => flags |= DRM_MODE_FLAG_PVSYNC,
        "-vsync" => flags |= DRM_MODE_FLAG_NVSYNC,
        _ => bail!("invalid vsync specifier"),
    };

    let type_ = DRM_MODE_TYPE_USERDEF;

    let name = CString::new(format!("{}x{}@{}", hdisplay, vdisplay, vrefresh / 1000)).unwrap();
    let mut name_buf = [0u8; 32];
    let _ = name_buf.as_mut_slice().write_all(name.as_bytes_with_nul());
    let name: [i8; 32] = bytemuck::cast(name_buf);

    Ok(drm_mode_modeinfo {
        clock,
        hdisplay,
        hsync_start,
        hsync_end,
        htotal,
        hskew: 0,
        vdisplay,
        vsync_start,
        vsync_end,
        vtotal,
        vscan: 0,
        vrefresh,
        flags,
        type_,
        name,
    })
}

/// Create a new drm mode from a given width, height, and optional refresh rate (defaults to 60Hz).
pub fn create_drm_mode(width: i32, height: i32, refresh_mhz: Option<u32>) -> drm::control::Mode {
    drm::control::Mode::from(generate_cvt_mode(
        width,
        height,
        refresh_mhz.map(|refresh| refresh as f64 / 1000.0),
    ))
}

// From https://gitlab.freedesktop.org/wlroots/wlroots/-/blob/95ac3e99242b4e7f59f00dd073ede405ff8e9e26/backend/drm/util.c#L247
fn generate_cvt_mode(hdisplay: i32, vdisplay: i32, vrefresh: Option<f64>) -> drm_mode_modeinfo {
    let options: di_cvt_options = di_cvt_options {
        red_blank_ver: di_cvt_reduced_blanking_version_DI_CVT_REDUCED_BLANKING_NONE,
        h_pixels: hdisplay,
        v_lines: vdisplay,
        ip_freq_rqd: vrefresh.unwrap_or(60.0),
        video_opt: false,
        vblank: 0.0,
        additional_hblank: 0,
        early_vsync_rqd: false,
        int_rqd: false,
        margins_rqd: false,
    };

    let mut timing = MaybeUninit::<di_cvt_timing>::zeroed();
    // SAFETY: is an ffi function
    unsafe { di_cvt_compute(timing.as_mut_ptr(), &options as *const _) };

    // SAFETY: Initialized in the function above
    let timing = unsafe { timing.assume_init() };

    let hsync_start = (hdisplay + timing.h_front_porch as i32) as u16;
    let vsync_start = (timing.v_lines_rnd + timing.v_front_porch) as u16;
    let hsync_end = hsync_start + timing.h_sync as u16;
    let vsync_end = vsync_start + timing.v_sync as u16;

    let name = CString::new(format!("{}x{}", hdisplay, vdisplay)).unwrap();
    let mut name_buf = [0u8; 32];
    let _ = name_buf.as_mut_slice().write_all(name.as_bytes_with_nul());
    let name: [i8; 32] = bytemuck::cast(name_buf);

    drm_mode_modeinfo {
        clock: f64::round(timing.act_pixel_freq * 1000.0) as u32,
        hdisplay: hdisplay as u16,
        hsync_start,
        hsync_end,
        htotal: hsync_end + timing.h_back_porch as u16,
        hskew: 0,
        vdisplay: timing.v_lines_rnd as u16,
        vsync_start,
        vsync_end,
        vtotal: vsync_end + timing.v_back_porch as u16,
        vscan: 0,
        vrefresh: f64::round(timing.act_frame_rate) as u32,
        flags: DRM_MODE_FLAG_NHSYNC | DRM_MODE_FLAG_PVSYNC,
        type_: DRM_MODE_TYPE_USERDEF,
        name,
    }
}
