//! Pure decoders that turn raw advertisement bytes into readable text.
//! Everything here works on data we already receive passively.

use serde::Serialize;

use crate::device::{hex, Device};

/// A labelled, human-readable value.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Field {
    pub label: String,
    pub value: String,
}

fn field(label: &str, value: impl Into<String>) -> Field {
    Field {
        label: label.to_string(),
        value: value.into(),
    }
}

// ---------------------------------------------------------------- ASCII view

/// Readable text found inside a payload, if it plausibly contains any.
/// Random bytes produce short accidental runs, so a payload only qualifies if
/// it has a long run or is mostly text.
pub fn ascii_view(bytes: &[u8]) -> Option<String> {
    let mut runs: Vec<String> = Vec::new();
    let mut current = String::new();
    for &b in bytes {
        if (0x20..=0x7e).contains(&b) {
            current.push(b as char);
        } else {
            if current.len() >= 4 {
                runs.push(std::mem::take(&mut current));
            }
            current.clear();
        }
    }
    if current.len() >= 4 {
        runs.push(current);
    }
    let covered: usize = runs.iter().map(|r| r.len()).sum();
    let longest = runs.iter().map(|r| r.len()).max().unwrap_or(0);
    if !runs.is_empty() && (longest >= 8 || covered * 2 >= bytes.len()) {
        Some(runs.join(" "))
    } else {
        None
    }
}

// --------------------------------------------------------------------- Apple

fn apple_message_name(t: u8) -> Option<&'static str> {
    Some(match t {
        0x02 => "iBeacon",
        0x03 => "AirPrint",
        0x05 => "AirDrop",
        0x06 => "HomeKit",
        0x07 => "Proximity Pairing",
        0x08 => "Hey Siri",
        0x09 => "AirPlay target",
        0x0a => "AirPlay source",
        0x0b => "Magic Switch",
        0x0c => "Handoff",
        0x0d => "Tethering target",
        0x0e => "Tethering source",
        0x0f => "Nearby Action",
        0x10 => "Nearby Info",
        0x12 => "Find My",
        _ => return None,
    })
}

fn nearby_info_activity(code: u8) -> &'static str {
    match code {
        0x0 => "Activity unknown",
        0x1 => "Activity reporting disabled",
        0x3 => "Idle",
        0x5 => "Audio playing, screen off",
        0x7 => "Screen on",
        0x9 => "Screen on, video playing",
        0xa => "Watch on wrist and unlocked",
        0xb => "Recent user interaction",
        0xd => "Driving",
        0xe => "Phone or FaceTime call",
        _ => "Other activity",
    }
}

fn airpods_model(model: &str) -> Option<&'static str> {
    Some(match model {
        "0220" => "AirPods (1st gen)",
        "0f20" => "AirPods (2nd gen)",
        "1320" => "AirPods (3rd gen)",
        "0e20" => "AirPods Pro",
        "1420" => "AirPods Pro (2nd gen)",
        "0a20" => "AirPods Max",
        "0520" => "BeatsX",
        "0620" => "Beats Solo3",
        "0b20" => "Powerbeats Pro",
        "0320" => "Powerbeats3",
        "0c20" => "Beats Solo Pro",
        _ => return None,
    })
}

fn battery_nibble(n: u8) -> Option<String> {
    (n <= 10).then(|| format!("{}%", n as u32 * 10))
}

/// Apple Continuity messages: a sequence of type/length/value records.
pub fn decode_apple(data: &[u8]) -> Vec<Field> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 2 <= data.len() {
        let t = data[i];
        let len = data[i + 1] as usize;
        let end = (i + 2 + len).min(data.len());
        let v = &data[(i + 2).min(data.len())..end];
        i += 2 + len;

        let name = apple_message_name(t)
            .map(str::to_string)
            .unwrap_or_else(|| format!("Message type 0x{t:02X}"));
        match t {
            0x02 if v.len() >= 21 => {
                let uuid = hex(&v[..16]);
                let uuid = format!(
                    "{}-{}-{}-{}-{}",
                    &uuid[0..8],
                    &uuid[8..12],
                    &uuid[12..16],
                    &uuid[16..20],
                    &uuid[20..32]
                );
                out.push(field("Apple iBeacon", uuid));
                out.push(field("iBeacon major / minor", format!(
                    "{} / {}",
                    u16::from_be_bytes([v[16], v[17]]),
                    u16::from_be_bytes([v[18], v[19]])
                )));
                out.push(field("iBeacon power at 1 m", format!("{} dBm", v[20] as i8)));
            }
            0x10 if !v.is_empty() => {
                out.push(field("Apple Nearby Info", nearby_info_activity(v[0] & 0x0f)));
            }
            0x07 if v.len() >= 6 => {
                let model = hex(&v[1..3]);
                let device = airpods_model(&model)
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("Unknown model {model}"));
                out.push(field("Apple Proximity Pairing", device));
                let (a, b) = (battery_nibble(v[4] >> 4), battery_nibble(v[4] & 0x0f));
                if let (Some(a), Some(b)) = (a, b) {
                    out.push(field("Earbud batteries", format!("{a} / {b}")));
                }
                if let Some(case) = battery_nibble(v[5] & 0x0f) {
                    out.push(field("Case battery", case));
                }
            }
            0x12 if !v.is_empty() => {
                let battery = match (v[0] >> 6) & 0x03 {
                    0 => "full",
                    1 => "medium",
                    2 => "low",
                    _ => "critical",
                };
                out.push(field("Apple Find My", "Accessory on the Find My network"));
                out.push(field("Find My battery", battery));
            }
            0x0c => out.push(field("Apple Handoff", "Continuity handoff (encrypted)")),
            0x0f => out.push(field("Apple Nearby Action", "Nearby action request")),
            _ => out.push(field("Apple message", format!("{name} ({} bytes)", v.len()))),
        }
    }
    out
}

/// Apple message types in a payload, used for the random-address heuristic.
fn apple_types(data: &[u8]) -> Vec<u8> {
    let mut types = Vec::new();
    let mut i = 0;
    while i + 2 <= data.len() {
        types.push(data[i]);
        i += 2 + data[i + 1] as usize;
    }
    types
}

// ----------------------------------------------------------------- Microsoft

fn windows_device_type(code: u8) -> &'static str {
    match code {
        0x01 => "Xbox One",
        0x06 => "Apple iPhone",
        0x07 => "Apple iPad",
        0x08 => "Android device",
        0x09 => "Windows 10 desktop",
        0x0b => "Windows 10 phone",
        0x0c => "Linux device",
        0x0d => "Windows IoT",
        0x0e => "Surface Hub",
        0x0f => "Windows laptop",
        0x10 => "Windows tablet",
        _ => "Unknown device type",
    }
}

pub fn decode_microsoft(data: &[u8]) -> Vec<Field> {
    // Microsoft Connected Devices Platform beacon: scenario type, then
    // version (high 3 bits) and device type (low 5 bits).
    if data.len() >= 2 && data[0] == 0x01 {
        vec![field(
            "Microsoft CDP beacon",
            windows_device_type(data[1] & 0x1f),
        )]
    } else {
        Vec::new()
    }
}

// ------------------------------------------------------------- Sensor makers

/// Ruuvi tag, data format 5 ("RAWv2").
pub fn decode_ruuvi(data: &[u8]) -> Vec<Field> {
    if data.len() < 18 || data[0] != 5 {
        return Vec::new();
    }
    let mut out = vec![field("Ruuvi", "RuuviTag sensor (format 5)")];
    let temp = i16::from_be_bytes([data[1], data[2]]);
    if temp != i16::MIN {
        out.push(field("Temperature", format!("{:.2} °C", temp as f64 * 0.005)));
    }
    let humidity = u16::from_be_bytes([data[3], data[4]]);
    if humidity != u16::MAX {
        out.push(field("Humidity", format!("{:.1} %", humidity as f64 * 0.0025)));
    }
    let pressure = u16::from_be_bytes([data[5], data[6]]);
    if pressure != u16::MAX {
        out.push(field("Pressure", format!("{:.2} hPa", (pressure as f64 + 50000.0) / 100.0)));
    }
    let power = u16::from_be_bytes([data[13], data[14]]);
    if power >> 5 != 0x7ff {
        out.push(field("Battery voltage", format!("{} mV", (power >> 5) as u32 + 1600)));
    }
    out
}

/// Govee H5072 / H5075 thermometer-hygrometers.
pub fn decode_govee(data: &[u8]) -> Vec<Field> {
    if data.len() != 6 {
        return Vec::new();
    }
    let raw = ((data[1] as u32) << 16) | ((data[2] as u32) << 8) | data[3] as u32;
    let negative = raw & 0x80_0000 != 0;
    let raw = raw & 0x7f_ffff;
    let mut temp = (raw / 1000) as f64 / 10.0;
    if negative {
        temp = -temp;
    }
    let humidity = (raw % 1000) as f64 / 10.0;
    vec![
        field("Govee", "Govee thermometer/hygrometer"),
        field("Temperature", format!("{temp:.1} °C")),
        field("Humidity", format!("{humidity:.1} %")),
        field("Sensor battery", format!("{}%", data[4])),
    ]
}

const COMPANY_APPLE: u16 = 0x004c;
const COMPANY_MICROSOFT: u16 = 0x0006;
const COMPANY_RUUVI: u16 = 0x0499;
const COMPANY_GOVEE: u16 = 0xec88;

/// Readable fields for one manufacturer-data entry.
pub fn decode_manufacturer(company_id: u16, data: &[u8]) -> Vec<Field> {
    match company_id {
        COMPANY_APPLE => decode_apple(data),
        COMPANY_MICROSOFT => decode_microsoft(data),
        COMPANY_RUUVI => decode_ruuvi(data),
        COMPANY_GOVEE => decode_govee(data),
        _ => Vec::new(),
    }
}

/// iBeacon measured power at 1 m, if this payload is an iBeacon.
pub fn ibeacon_power(company_id: u16, data: &[u8]) -> Option<i16> {
    if company_id != COMPANY_APPLE {
        return None;
    }
    let mut i = 0;
    while i + 2 <= data.len() {
        let (t, len) = (data[i], data[i + 1] as usize);
        if t == 0x02 && len >= 21 && i + 2 + 21 <= data.len() {
            return Some(data[i + 2 + 20] as i8 as i16);
        }
        i += 2 + len;
    }
    None
}

// -------------------------------------------------------------- Service data

fn eddystone_url_scheme(b: u8) -> &'static str {
    match b {
        0 => "http://www.",
        1 => "https://www.",
        2 => "http://",
        3 => "https://",
        _ => "",
    }
}

fn eddystone_url_expansion(b: u8) -> Option<&'static str> {
    Some(match b {
        0 => ".com/",
        1 => ".org/",
        2 => ".edu/",
        3 => ".net/",
        4 => ".info/",
        5 => ".biz/",
        6 => ".gov/",
        7 => ".com",
        8 => ".org",
        9 => ".edu",
        10 => ".net",
        11 => ".info",
        12 => ".biz",
        13 => ".gov",
        _ => return None,
    })
}

pub fn decode_eddystone(data: &[u8]) -> Vec<Field> {
    let Some((&frame, rest)) = data.split_first() else {
        return Vec::new();
    };
    match frame {
        0x00 if rest.len() >= 17 => vec![
            field("Eddystone-UID namespace", hex(&rest[1..11])),
            field("Eddystone-UID instance", hex(&rest[11..17])),
        ],
        0x10 if rest.len() >= 2 => {
            let mut url = eddystone_url_scheme(rest[1]).to_string();
            for &b in &rest[2..] {
                match eddystone_url_expansion(b) {
                    Some(e) => url.push_str(e),
                    None => url.push(b as char),
                }
            }
            vec![field("Eddystone-URL", url)]
        }
        0x20 if rest.len() >= 13 => {
            let voltage = u16::from_be_bytes([rest[1], rest[2]]);
            let temp_raw = i16::from_be_bytes([rest[3], rest[4]]);
            let adv_count = u32::from_be_bytes([rest[5], rest[6], rest[7], rest[8]]);
            let uptime = u32::from_be_bytes([rest[9], rest[10], rest[11], rest[12]]) as f64 / 10.0;
            let mut out = vec![field("Eddystone-TLM", "Beacon telemetry")];
            if voltage != 0 {
                out.push(field("Beacon battery", format!("{voltage} mV")));
            }
            if temp_raw != -0x8000 {
                out.push(field("Beacon temperature", format!("{:.1} °C", temp_raw as f64 / 256.0)));
            }
            out.push(field("Beacon advertisements sent", adv_count.to_string()));
            out.push(field("Beacon uptime", format!("{uptime:.0} s")));
            out
        }
        _ => Vec::new(),
    }
}

/// Readable fields for one service-data entry (uuid is normalized).
pub fn decode_service_data(uuid: &str, data: &[u8]) -> Vec<Field> {
    match uuid {
        "180F" if !data.is_empty() => vec![field("Battery", format!("{}%", data[0]))],
        "FEAA" => decode_eddystone(data),
        "FE2C" => {
            if data.len() == 3 {
                vec![field("Google Fast Pair", format!("Pairing mode, model ID 0x{}", hex(data).to_uppercase()))]
            } else {
                vec![field("Google Fast Pair", "Not in pairing mode")]
            }
        }
        "FD6F" => vec![field("Exposure Notification", "Contact-tracing beacon (rotating identifier)")],
        "FEED" | "FEEC" => vec![field("Tile", "Tile tracker")],
        "FD5A" => vec![field("Samsung SmartTag", "Samsung SmartTag tracker")],
        "FE95" => vec![field("Xiaomi", "Xiaomi device (payload not documented)")],
        _ => Vec::new(),
    }
}

// ---------------------------------------------------------------- Appearance

/// Category of a GAP appearance value (the top 10 bits), as plain text.
pub fn appearance_label(value: u16) -> Option<String> {
    let category = value >> 6;
    Some(
        match category {
            0 => return None,
            1 => "Phone",
            2 => "Computer",
            3 => "Watch",
            4 => "Clock",
            5 => "Display",
            6 => "Remote control",
            7 => "Eyeglasses",
            8 => "Tag",
            9 => "Keyring",
            10 => "Media player",
            11 => "Barcode scanner",
            12 => "Thermometer",
            13 => "Heart rate sensor",
            14 => "Blood pressure monitor",
            15 => "Human interface device",
            16 => "Glucose meter",
            17 => "Running/walking sensor",
            18 => "Cycling sensor",
            49 => "Pulse oximeter",
            50 => "Weight scale",
            51 => "Personal mobility device",
            52 => "Continuous glucose monitor",
            53 => "Insulin pump",
            54 => "Medication delivery",
            81 => "Outdoor sports device",
            _ => return Some(format!("Appearance 0x{value:04X}")),
        }
        .to_string(),
    )
}

// ------------------------------------------------------------------ Distance

/// Rough distance estimate from RSSI using a log-distance path-loss model.
/// The reference signal at 1 m comes from, in order: an iBeacon's calibrated
/// power, the advertised TX power minus 41 dB (free-space loss at 1 m on
/// 2.4 GHz), or a typical -59 dBm. Indoor exponent 2.5. Treat as an estimate.
pub fn estimate_distance_m(rssi: i16, tx_power: Option<i16>, ibeacon: Option<i16>) -> f64 {
    let reference = ibeacon
        .map(|p| p as f64)
        .or(tx_power.map(|t| t as f64 - 41.0))
        .unwrap_or(-59.0);
    let d = 10f64.powf((reference - rssi as f64) / (10.0 * 2.5));
    (d.clamp(0.1, 100.0) * 10.0).round() / 10.0
}

// ------------------------------------------------------------ Random address

/// Whether the device likely uses a rotating (randomized) address, and why.
/// macOS hides real addresses, so this also looks at payload types that are
/// known to be broadcast from rotating addresses.
pub fn likely_random(device: &Device) -> Option<String> {
    if device.address_type.as_deref() == Some("random") {
        return Some("Address type is random".to_string());
    }
    if device.address_type.as_deref() == Some("public") {
        return None;
    }
    for m in &device.manufacturers {
        let bytes = m.bytes();
        if m.company_id == COMPANY_APPLE {
            let rotating = apple_types(&bytes)
                .iter()
                .any(|t| matches!(t, 0x07 | 0x0c | 0x0f | 0x10 | 0x12));
            if rotating {
                return Some("Apple Continuity messages use rotating addresses".to_string());
            }
        }
        if m.company_id == COMPANY_MICROSOFT && bytes.first() == Some(&0x01) {
            return Some("Windows CDP beacons use rotating addresses".to_string());
        }
    }
    if device.service_data.iter().any(|d| d.uuid == "FD6F") {
        return Some("Exposure Notification beacons rotate their address".to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::unhex;

    #[test]
    fn apple_nearby_info_from_real_capture() {
        // From a capture: 10 06 05 19 d3 12 c7 28
        let fields = decode_apple(&unhex("10060519d312c728"));
        assert_eq!(fields[0], field("Apple Nearby Info", "Audio playing, screen off"));
    }

    #[test]
    fn apple_find_my_and_handoff() {
        let fields = decode_apple(&unhex("121920a62ed401097ea5c4d225e85f107722802e202b9b10ad035b"));
        assert_eq!(fields[0].label, "Apple Find My");
        assert_eq!(fields[1], field("Find My battery", "full"));
        let fields = decode_apple(&unhex("0c0e0048a2fe84a0e890e5685a96f1141006741d79fde608"));
        assert_eq!(fields[0].label, "Apple Handoff");
        // Then continues with the trailing Nearby Info message.
        assert_eq!(fields[1].label, "Apple Nearby Info");
    }

    #[test]
    fn ibeacon() {
        let mut payload = vec![0x02, 0x15];
        payload.extend(0u8..16);
        payload.extend([0x00, 0x07, 0x00, 0x09, 0xc5]);
        let fields = decode_apple(&payload);
        assert_eq!(fields[1], field("iBeacon major / minor", "7 / 9"));
        assert_eq!(fields[2], field("iBeacon power at 1 m", "-59 dBm"));
        assert_eq!(ibeacon_power(COMPANY_APPLE, &payload), Some(-59));
    }

    #[test]
    fn airpods() {
        // type 07, len 0x19-ish (truncated is fine): prefix, model 0e20, status, batteries 8/9, case 10
        let fields = decode_apple(&unhex("070801 0e20 00 89 0a".replace(' ', "").as_str()));
        assert_eq!(fields[0], field("Apple Proximity Pairing", "AirPods Pro"));
        assert_eq!(fields[1], field("Earbud batteries", "80% / 90%"));
        assert_eq!(fields[2], field("Case battery", "100%"));
    }

    #[test]
    fn microsoft_cdp() {
        assert_eq!(decode_microsoft(&[0x01, 0x09, 0x20])[0].value, "Windows 10 desktop");
        assert!(decode_microsoft(&[0x02]).is_empty());
    }

    #[test]
    fn ruuvi_spec_example() {
        // Example from the Ruuvi data format 5 spec.
        let data = unhex("0512FC5394C37C0004FFFC040CAC364200CDCBB8334C884F");
        let f = decode_ruuvi(&data);
        assert_eq!(f[1], field("Temperature", "24.30 °C"));
        assert_eq!(f[2], field("Humidity", "53.5 %"));
        assert_eq!(f[3], field("Pressure", "1000.44 hPa"));
        assert_eq!(f[4], field("Battery voltage", "2977 mV"));
    }

    #[test]
    fn govee() {
        // 0x0332 -> temp/humidity packed value 0x03_20_0d?  Build one: 23.4 C, 51.2 %
        let raw: u32 = 234 * 1000 + 512;
        let data = [0x00, (raw >> 16) as u8, (raw >> 8) as u8, raw as u8, 87, 0x00];
        let f = decode_govee(&data);
        assert_eq!(f[1], field("Temperature", "23.4 °C"));
        assert_eq!(f[2], field("Humidity", "51.2 %"));
        assert_eq!(f[3], field("Sensor battery", "87%"));
    }

    #[test]
    fn battery_and_eddystone_url() {
        assert_eq!(decode_service_data("180F", &[100])[0], field("Battery", "100%"));
        // 0x10, tx power, scheme https://, "example", ".com/"
        let mut data = vec![0x10, 0xe7, 0x03];
        data.extend(b"example");
        data.push(0x00);
        assert_eq!(decode_eddystone(&data)[0], field("Eddystone-URL", "https://example.com/"));
    }

    #[test]
    fn ascii() {
        assert_eq!(ascii_view(&[0, 0x31, 0x32, 0x33, 0x34, 0x35, 0]), Some("12345".into()));
        // Short accidental run inside binary data is ignored.
        assert_eq!(ascii_view(&unhex("0c0e0048a2fe84a0e890e5685a96f1141006741d79fde608")), None);
    }

    #[test]
    fn appearance() {
        assert_eq!(appearance_label(0x00C0).as_deref(), Some("Watch"));
        assert_eq!(appearance_label(0x03C1).as_deref(), Some("Human interface device"));
        assert_eq!(appearance_label(0), None);
    }

    #[test]
    fn distance_is_monotonic() {
        let near = estimate_distance_m(-50, None, None);
        let far = estimate_distance_m(-90, None, None);
        assert!(near < far);
        assert_eq!(estimate_distance_m(-59, None, None), 1.0);
        assert_eq!(estimate_distance_m(-59, None, Some(-59)), 1.0);
    }
}
