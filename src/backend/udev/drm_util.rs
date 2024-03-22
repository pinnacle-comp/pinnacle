use smithay::reexports::drm::control::{connector, property, Device, ResourceHandle};

// A bunch of this stuff is from cosmic-comp

#[derive(Debug, Clone)]
pub struct EdidInfo {
    pub model: String,
    pub manufacturer: String,
}

impl EdidInfo {
    pub fn try_from_device_and_connector(
        device: &impl Device,
        connector: connector::Handle,
    ) -> anyhow::Result<Self> {
        let edid_prop = get_prop(device, connector, "EDID")?;
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

        dbg!(info)
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
    })
}

fn get_prop(
    device: &impl Device,
    handle: impl ResourceHandle,
    name: &str,
) -> anyhow::Result<property::Handle> {
    let props = device.get_properties(handle)?;
    let (prop_handles, _) = props.as_props_and_values();
    for prop in prop_handles {
        let info = device.get_property(*prop)?;
        if Some(name) == info.name().to_str().ok() {
            return Ok(*prop);
        }
    }
    anyhow::bail!("No prop found for {}", name)
}

fn get_manufacturer(vendor: [char; 3]) -> String {
    match vendor {
        ['A', 'A', 'A'] => "Avolites Ltd".to_string(),
        ['A', 'C', 'I'] => "Ancor Communications Inc".to_string(),
        ['A', 'C', 'R'] => "Acer Technologies".to_string(),
        ['A', 'D', 'A'] => "Addi-Data GmbH".to_string(),
        ['A', 'P', 'P'] => "Apple Computer Inc".to_string(),
        ['A', 'S', 'K'] => "Ask A/S".to_string(),
        ['A', 'V', 'T'] => "Avtek (Electronics) Pty Ltd".to_string(),
        ['B', 'N', 'O'] => "Bang & Olufsen".to_string(),
        ['B', 'N', 'Q'] => "BenQ Corporation".to_string(),
        ['C', 'M', 'N'] => "Chimei Innolux Corporation".to_string(),
        ['C', 'M', 'O'] => "Chi Mei Optoelectronics corp.".to_string(),
        ['C', 'R', 'O'] => "Extraordinary Technologies PTY Limited".to_string(),
        ['D', 'E', 'L'] => "Dell Inc.".to_string(),
        ['D', 'G', 'C'] => "Data General Corporation".to_string(),
        ['D', 'O', 'N'] => "DENON, Ltd.".to_string(),
        ['E', 'N', 'C'] => "Eizo Nanao Corporation".to_string(),
        ['E', 'P', 'H'] => "Epiphan Systems Inc.".to_string(),
        ['E', 'X', 'P'] => "Data Export Corporation".to_string(),
        ['F', 'N', 'I'] => "Funai Electric Co., Ltd.".to_string(),
        ['F', 'U', 'S'] => "Fujitsu Siemens Computers GmbH".to_string(),
        ['G', 'S', 'M'] => "Goldstar Company Ltd".to_string(),
        ['H', 'I', 'Q'] => "Kaohsiung Opto Electronics Americas, Inc.".to_string(),
        ['H', 'S', 'D'] => "HannStar Display Corp".to_string(),
        ['H', 'T', 'C'] => "Hitachi Ltd".to_string(),
        ['H', 'W', 'P'] => "Hewlett Packard".to_string(),
        ['I', 'N', 'T'] => "Interphase Corporation".to_string(),
        ['I', 'N', 'X'] => "Communications Supply Corporation (A division of WESCO)".to_string(),
        ['I', 'T', 'E'] => "Integrated Tech Express Inc".to_string(),
        ['I', 'V', 'M'] => "Iiyama North America".to_string(),
        ['L', 'E', 'N'] => "Lenovo Group Limited".to_string(),
        ['M', 'A', 'X'] => "Rogen Tech Distribution Inc".to_string(),
        ['M', 'E', 'G'] => "Abeam Tech Ltd".to_string(),
        ['M', 'E', 'I'] => "Panasonic Industry Company".to_string(),
        ['M', 'T', 'C'] => "Mars-Tech Corporation".to_string(),
        ['M', 'T', 'X'] => "Matrox".to_string(),
        ['N', 'E', 'C'] => "NEC Corporation".to_string(),
        ['N', 'E', 'X'] => "Nexgen Mediatech Inc.".to_string(),
        ['O', 'N', 'K'] => "ONKYO Corporation".to_string(),
        ['O', 'R', 'N'] => "ORION ELECTRIC CO., LTD.".to_string(),
        ['O', 'T', 'M'] => "Optoma Corporation".to_string(),
        ['O', 'V', 'R'] => "Oculus VR, Inc.".to_string(),
        ['P', 'H', 'L'] => "Philips Consumer Electronics Company".to_string(),
        ['P', 'I', 'O'] => "Pioneer Electronic Corporation".to_string(),
        ['P', 'N', 'R'] => "Planar Systems, Inc.".to_string(),
        ['Q', 'D', 'S'] => "Quanta Display Inc.".to_string(),
        ['R', 'A', 'T'] => "Rent-A-Tech".to_string(),
        ['R', 'E', 'N'] => "Renesas Technology Corp.".to_string(),
        ['S', 'A', 'M'] => "Samsung Electric Company".to_string(),
        ['S', 'A', 'N'] => "Sanyo Electric Co., Ltd.".to_string(),
        ['S', 'E', 'C'] => "Seiko Epson Corporation".to_string(),
        ['S', 'H', 'P'] => "Sharp Corporation".to_string(),
        ['S', 'I', 'I'] => "Silicon Image, Inc.".to_string(),
        ['S', 'N', 'Y'] => "Sony".to_string(),
        ['S', 'T', 'D'] => "STD Computer Inc".to_string(),
        ['S', 'V', 'S'] => "SVSI".to_string(),
        ['S', 'Y', 'N'] => "Synaptics Inc".to_string(),
        ['T', 'C', 'L'] => "Technical Concepts Ltd".to_string(),
        ['T', 'O', 'P'] => "Orion Communications Co., Ltd.".to_string(),
        ['T', 'S', 'B'] => "Toshiba America Info Systems Inc".to_string(),
        ['T', 'S', 'T'] => "Transtream Inc".to_string(),
        ['U', 'N', 'K'] => "Unknown".to_string(),
        ['V', 'E', 'S'] => "Vestel Elektronik Sanayi ve Ticaret A. S.".to_string(),
        ['V', 'I', 'T'] => "Visitech AS".to_string(),
        ['V', 'I', 'Z'] => "VIZIO, Inc".to_string(),
        ['V', 'S', 'C'] => "ViewSonic Corporation".to_string(),
        ['Y', 'M', 'H'] => "Yamaha Corporation".to_string(),
        _ => vendor.iter().collect(),
    }
}
