//! Debugging utilities.
//!
//! WARNING: This module is not governed by the API stability guarantees.

use pinnacle_api_defs::pinnacle::{
    debug::v1::{SetDamageVisualizationRequest, SetOpaqueRegionVisualizationRequest},
    util::v1::SetOrToggle,
};

use crate::{client::Client, BlockOnTokio};

/// Sets damage visualization.
///
/// When on, parts of the screen that are damaged after rendering will have
/// red rectangles drawn where the damage is.
pub fn set_damage_visualization(set: bool) {
    Client::debug()
        .set_damage_visualization(SetDamageVisualizationRequest {
            set_or_toggle: match set {
                true => SetOrToggle::Set,
                false => SetOrToggle::Unset,
            }
            .into(),
        })
        .block_on_tokio()
        .unwrap();
}

/// Toggles damage visualization.
///
/// When on, parts of the screen that are damaged after rendering will have
/// red rectangles drawn where the damage is.
pub fn toggle_damage_visualization() {
    Client::debug()
        .set_damage_visualization(SetDamageVisualizationRequest {
            set_or_toggle: SetOrToggle::Toggle.into(),
        })
        .block_on_tokio()
        .unwrap();
}

/// Sets opaque region visualization.
///
/// When on, parts of the screen that are opaque will have a transparent blue rectangle
/// drawn over it, while parts that are not opaque will have a transparent red rectangle
/// drawn.
pub fn set_opaque_region_visualization(set: bool) {
    Client::debug()
        .set_opaque_region_visualization(SetOpaqueRegionVisualizationRequest {
            set_or_toggle: match set {
                true => SetOrToggle::Set,
                false => SetOrToggle::Unset,
            }
            .into(),
        })
        .block_on_tokio()
        .unwrap();
}

/// Toggles opaque region visualization.
///
/// When on, parts of the screen that are opaque will have a transparent blue rectangle
/// drawn over it, while parts that are not opaque will have a transparent red rectangle
/// drawn.
pub fn toggle_opaque_region_visualization() {
    Client::debug()
        .set_opaque_region_visualization(SetOpaqueRegionVisualizationRequest {
            set_or_toggle: SetOrToggle::Toggle.into(),
        })
        .block_on_tokio()
        .unwrap();
}
