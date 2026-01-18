use std::{ffi::CString, io::Write, time::Duration};

use drm_sys::{
    DRM_MODE_FLAG_NHSYNC, DRM_MODE_FLAG_NVSYNC, DRM_MODE_FLAG_PHSYNC, DRM_MODE_FLAG_PVSYNC,
    DRM_MODE_TYPE_USERDEF, drm_mode_modeinfo,
};
use libdisplay_info::cvt::{self, ReducedBlankingVersion};
use smithay::reexports::drm::{self, control::ModeFlags};

// A bunch of this stuff is from cosmic-comp

pub fn drm_mode_from_modeinfo(
    clock: f32,
    hdisplay: u32,
    hsync_start: u32,
    hsync_end: u32,
    htotal: u32,
    vdisplay: u32,
    vsync_start: u32,
    vsync_end: u32,
    vtotal: u32,
    hsync: bool,
    vsync: bool,
) -> drm::control::Mode {
    let clock = clock * 1000.0;

    let vrefresh = (clock * 1000.0 * 1000.0 / htotal as f32 / vtotal as f32) as u32;

    let mut flags = 0;
    match hsync {
        true => flags |= DRM_MODE_FLAG_PHSYNC,
        false => flags |= DRM_MODE_FLAG_NHSYNC,
    };
    match vsync {
        true => flags |= DRM_MODE_FLAG_PVSYNC,
        false => flags |= DRM_MODE_FLAG_NVSYNC,
    };

    let type_ = DRM_MODE_TYPE_USERDEF;

    let name = CString::new(format!(
        "{}x{}@{:.3}",
        hdisplay,
        vdisplay,
        vrefresh as f64 / 1000.0
    ))
    .unwrap();
    let mut name_buf = [0u8; 32];
    let _ = name_buf.as_mut_slice().write_all(name.as_bytes_with_nul());
    let name = bytemuck::cast(name_buf);

    drm_mode_modeinfo {
        clock: clock as u32,
        hdisplay: hdisplay as u16,
        hsync_start: hsync_start as u16,
        hsync_end: hsync_end as u16,
        htotal: htotal as u16,
        hskew: 0,
        vdisplay: vdisplay as u16,
        vsync_start: vsync_start as u16,
        vsync_end: vsync_end as u16,
        vtotal: vtotal as u16,
        vscan: 0,
        vrefresh,
        flags,
        type_,
        name,
    }
    .into()
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
    let options = cvt::Options {
        red_blank_ver: ReducedBlankingVersion::None,
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

    let timing = cvt::Timing::compute(options);

    let hsync_start = (hdisplay + timing.h_front_porch as i32) as u16;
    let vsync_start = (timing.v_lines_rnd + timing.v_front_porch) as u16;
    let hsync_end = hsync_start + timing.h_sync as u16;
    let vsync_end = vsync_start + timing.v_sync as u16;

    let name = CString::new(format!("{hdisplay}x{vdisplay}")).unwrap();
    let mut name_buf = [0u8; 32];
    let _ = name_buf.as_mut_slice().write_all(name.as_bytes_with_nul());
    let name = bytemuck::cast(name_buf);

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

pub fn refresh_interval(mode: drm::control::Mode) -> Duration {
    let clock = mode.clock() as u64;
    let htotal = mode.hsync().2 as u64;
    let vtotal = mode.vsync().2 as u64;

    let mut numerator = htotal * vtotal * 1_000_000;
    let mut denominator = clock;

    if mode.flags().contains(ModeFlags::INTERLACE) {
        denominator *= 2;
    }

    if mode.flags().contains(ModeFlags::DBLSCAN) {
        numerator *= 2;
    }

    if mode.vscan() > 1 {
        numerator *= mode.vscan() as u64;
    }

    let refresh_interval_ns = (numerator + denominator / 2) / denominator;
    Duration::from_nanos(refresh_interval_ns)
}
