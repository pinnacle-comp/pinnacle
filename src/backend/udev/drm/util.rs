use std::num::NonZeroU32;

use anyhow::Context;
use smithay::reexports::drm::control::{connector, property, Device, ResourceHandle};

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
