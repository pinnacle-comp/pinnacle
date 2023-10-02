// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::Duration;

use smithay::reexports::drm::control::{Mode, ModeFlags};

// From niri:
// https://github.com/YaLTeR/niri/blob/ba0a6d6b8868cc6348ad1b20f683a95d5909df6b/src/backend/tty.rs#L900-L922
pub fn refresh_time(mode: Mode) -> Duration {
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

    let refresh_interval = (numerator + denominator / 2) / denominator;
    Duration::from_nanos(refresh_interval)
}
