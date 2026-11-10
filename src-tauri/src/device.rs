use btleplug::api::PeripheralProperties;
use serde::{Deserialize, Serialize};

use crate::ids;

/// A device is "active" if it was seen within this window.
pub const ACTIVE_WINDOW_MS: u64 = 10_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub word: String,
    pub name: String,
    pub company_id: Option<String>,
    pub company_name: Option<String>,
    pub manufacturer_data_hex: Option<String>,
    pub rssi: Option<i16>,
    pub tx_power: Option<i16>,
    pub address: Option<String>,
    pub services: Vec<String>,
    pub first_seen: u64,
    pub last_seen: u64,
    /// Number of advertisements received from this device (persisted).
    #[serde(default)]
    pub seen_count: u64,
    #[serde(default)]
    pub active: bool,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

impl Device {
    /// Builds a device from a fresh advertisement (port of `formatDeviceInfo`).
    /// `counted` says whether this sighting is one advertisement that should
    /// add to `seen_count` (btleplug also emits extra events per advertisement).
    pub fn from_properties(
        id: &str,
        props: &PeripheralProperties,
        now: u64,
        counted: bool,
    ) -> Device {
        let name = props
            .local_name
            .clone()
            .or_else(|| props.advertisement_name.clone())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "Unknown".to_string());

        // The CLI reads the company id from the first two bytes of the
        // manufacturer data; btleplug already splits it out as the map key.
        let manufacturer = props
            .manufacturer_data
            .iter()
            .min_by_key(|(company, _)| **company);
        let (company_id, company_name, manufacturer_data_hex) = match manufacturer {
            Some((company, payload)) => (
                Some(ids::company_id_hex(*company)),
                Some(ids::company_name(*company)),
                Some(hex(payload)),
            ),
            None => (None, None, None),
        };

        let address = props.address.to_string();
        let address = if address == "00:00:00:00:00:00" {
            None
        } else {
            Some(address)
        };

        Device {
            id: id.to_string(),
            word: ids::deterministic_word(id),
            name,
            company_id,
            company_name,
            manufacturer_data_hex,
            rssi: props.rssi,
            tx_power: props.tx_power_level,
            address,
            services: props.services.iter().map(|s| s.to_string()).collect(),
            first_seen: now,
            last_seen: now,
            seen_count: counted as u64,
            active: true,
        }
    }

    /// Merges a newer sighting into this record, keeping known details that the
    /// latest advertisement happened to omit.
    pub fn merge(&mut self, newer: Device) {
        if newer.name != "Unknown" {
            self.name = newer.name;
        }
        if newer.company_id.is_some() {
            self.company_id = newer.company_id;
            self.company_name = newer.company_name;
            self.manufacturer_data_hex = newer.manufacturer_data_hex;
        }
        if newer.rssi.is_some() {
            self.rssi = newer.rssi;
        }
        if newer.tx_power.is_some() {
            self.tx_power = newer.tx_power;
        }
        if newer.address.is_some() {
            self.address = newer.address;
        }
        if !newer.services.is_empty() {
            self.services = newer.services;
        }
        self.last_seen = newer.last_seen;
        self.seen_count += newer.seen_count;
        self.active = true;
    }
}
