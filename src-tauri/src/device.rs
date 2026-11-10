use serde::{Deserialize, Serialize};

use crate::ids;

/// A device is "active" if it was seen within this window.
pub const ACTIVE_WINDOW_MS: u64 = 10_000;

/// One manufacturer-specific data entry from an advertisement.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mfr {
    pub company_id: u16,
    pub data_hex: String,
}

/// One service-data entry from an advertisement (uuid is normalized).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SvcData {
    pub uuid: String,
    pub data_hex: String,
}

/// What an opt-in GATT query read from the device.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GattInfo {
    pub queried_at: u64,
    pub device_name: Option<String>,
    pub manufacturer_name: Option<String>,
    pub model_number: Option<String>,
    pub hardware_revision: Option<String>,
    pub firmware_revision: Option<String>,
    pub software_revision: Option<String>,
    pub battery_percent: Option<u8>,
    pub appearance: Option<u16>,
    pub appearance_label: Option<String>,
}

/// One received advertisement (or a refresh of what we know), independent of
/// which backend produced it.
#[derive(Clone, Debug, Default)]
pub struct Sighting {
    pub id: String,
    pub local_name: Option<String>,
    pub advertisement_name: Option<String>,
    pub manufacturers: Vec<Mfr>,
    pub service_data: Vec<SvcData>,
    pub services: Vec<String>,
    pub rssi: Option<i16>,
    pub tx_power: Option<i16>,
    pub appearance: Option<u16>,
    pub address: Option<String>,
    pub address_type: Option<String>,
    pub connectable: Option<bool>,
}

/// The persisted record for a device.
#[derive(Clone, Debug)]
pub struct Device {
    pub id: String,
    pub word: String,
    /// Display name: local name, else advertised name, else "Unknown".
    pub name: String,
    pub local_name: Option<String>,
    pub advertisement_name: Option<String>,
    pub manufacturers: Vec<Mfr>,
    pub service_data: Vec<SvcData>,
    pub services: Vec<String>,
    pub rssi: Option<i16>,
    pub tx_power: Option<i16>,
    pub appearance: Option<u16>,
    pub address: Option<String>,
    pub address_type: Option<String>,
    pub connectable: Option<bool>,
    pub gatt: Option<GattInfo>,
    pub first_seen: u64,
    pub last_seen: u64,
    pub seen_count: u64,
}

pub fn display_name(local: &Option<String>, advertised: &Option<String>) -> String {
    local
        .as_ref()
        .or(advertised.as_ref())
        .filter(|n| !n.is_empty())
        .cloned()
        .unwrap_or_else(|| "Unknown".to_string())
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .filter_map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok())
        .collect()
}

fn non_empty(s: Option<String>) -> Option<String> {
    s.filter(|v| !v.is_empty())
}

impl Mfr {
    pub fn bytes(&self) -> Vec<u8> {
        unhex(&self.data_hex)
    }
}

impl SvcData {
    pub fn bytes(&self) -> Vec<u8> {
        unhex(&self.data_hex)
    }
}

impl Device {
    /// Builds a new record from the first sighting.
    pub fn from_sighting(s: Sighting, now: u64, counted: bool) -> Device {
        let local_name = non_empty(s.local_name);
        let advertisement_name = non_empty(s.advertisement_name);
        let mut manufacturers = s.manufacturers;
        manufacturers.sort_by_key(|m| m.company_id);
        let mut service_data = s.service_data;
        service_data.sort_by(|a, b| a.uuid.cmp(&b.uuid));
        Device {
            word: ids::deterministic_word(&s.id),
            name: display_name(&local_name, &advertisement_name),
            id: s.id,
            local_name,
            advertisement_name,
            manufacturers,
            service_data,
            services: s.services,
            rssi: s.rssi,
            tx_power: s.tx_power,
            appearance: s.appearance,
            address: non_empty(s.address),
            address_type: s.address_type,
            connectable: s.connectable,
            gatt: None,
            first_seen: now,
            last_seen: now,
            seen_count: counted as u64,
        }
    }

    /// Merges a newer sighting, keeping known details that the latest
    /// advertisement happened to omit.
    pub fn merge(&mut self, s: Sighting, now: u64, counted: bool) {
        if let Some(name) = non_empty(s.local_name) {
            self.local_name = Some(name);
        }
        if let Some(name) = non_empty(s.advertisement_name) {
            self.advertisement_name = Some(name);
        }
        self.name = display_name(&self.local_name, &self.advertisement_name);

        for m in s.manufacturers {
            match self.manufacturers.iter_mut().find(|e| e.company_id == m.company_id) {
                Some(existing) => *existing = m,
                None => self.manufacturers.push(m),
            }
        }
        self.manufacturers.sort_by_key(|m| m.company_id);

        for d in s.service_data {
            match self.service_data.iter_mut().find(|e| e.uuid == d.uuid) {
                Some(existing) => *existing = d,
                None => self.service_data.push(d),
            }
        }
        self.service_data.sort_by(|a, b| a.uuid.cmp(&b.uuid));

        if !s.services.is_empty() {
            self.services = s.services;
        }
        if s.rssi.is_some() {
            self.rssi = s.rssi;
        }
        if s.tx_power.is_some() {
            self.tx_power = s.tx_power;
        }
        if s.appearance.is_some() {
            self.appearance = s.appearance;
        }
        if let Some(address) = non_empty(s.address) {
            self.address = Some(address);
        }
        if s.address_type.is_some() {
            self.address_type = s.address_type;
        }
        if s.connectable.is_some() {
            self.connectable = s.connectable;
        }
        self.last_seen = now;
        self.seen_count += counted as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sighting(id: &str) -> Sighting {
        Sighting {
            id: id.into(),
            ..Default::default()
        }
    }

    #[test]
    fn hex_round_trip() {
        assert_eq!(hex(&[0x0c, 0xff, 0x01]), "0cff01");
        assert_eq!(unhex("0cff01"), vec![0x0c, 0xff, 0x01]);
    }

    #[test]
    fn merge_keeps_known_details() {
        let mut first = sighting("a");
        first.local_name = Some("Thing".into());
        first.advertisement_name = Some("Cached".into());
        first.manufacturers = vec![Mfr { company_id: 0x4c, data_hex: "10".into() }];
        let mut d = Device::from_sighting(first, 100, true);
        assert_eq!(d.name, "Thing");

        // A later advertisement without a name or manufacturer data.
        let mut second = sighting("a");
        second.rssi = Some(-70);
        second.manufacturers = vec![Mfr { company_id: 0x06, data_hex: "01".into() }];
        d.merge(second, 200, true);

        assert_eq!(d.name, "Thing");
        assert_eq!(d.advertisement_name.as_deref(), Some("Cached"));
        assert_eq!(d.manufacturers.len(), 2);
        assert_eq!(d.manufacturers[0].company_id, 0x06);
        assert_eq!(d.rssi, Some(-70));
        assert_eq!(d.seen_count, 2);
        assert_eq!((d.first_seen, d.last_seen), (100, 200));
    }

    #[test]
    fn uncounted_sightings_do_not_add() {
        let mut d = Device::from_sighting(sighting("a"), 1, true);
        d.merge(sighting("a"), 2, false);
        assert_eq!(d.seen_count, 1);
        assert_eq!(d.name, "Unknown");
    }
}
