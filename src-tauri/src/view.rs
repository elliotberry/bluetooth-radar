//! What the UI receives: a persisted `Device` plus decoded text and values
//! derived from live (in-memory) tracking.

use std::collections::VecDeque;

use serde::Serialize;

use crate::decode::{self, Field};
use crate::device::{hex, Device, GattInfo, ACTIVE_WINDOW_MS};
use crate::ids;

const RSSI_HISTORY_LEN: usize = 120; // one sample per second, two minutes
const SIGHTING_WINDOW: usize = 30;

/// In-memory tracking that is not persisted.
#[derive(Default)]
pub struct Live {
    rssi: VecDeque<(u64, i16)>,
    sightings: VecDeque<u64>,
}

impl Live {
    pub fn record(&mut self, now: u64, rssi: Option<i16>, counted: bool) {
        if let Some(rssi) = rssi {
            let due = self.rssi.back().map_or(true, |(t, _)| now >= t + 1000);
            if due {
                self.rssi.push_back((now, rssi));
                if self.rssi.len() > RSSI_HISTORY_LEN {
                    self.rssi.pop_front();
                }
            }
        }
        if counted {
            self.sightings.push_back(now);
            if self.sightings.len() > SIGHTING_WINDOW {
                self.sightings.pop_front();
            }
        }
    }

    pub fn rssi_history(&self) -> Vec<i16> {
        self.rssi.iter().map(|(_, r)| *r).collect()
    }

    /// Closer / farther / steady, comparing the latest few samples to the
    /// few before them.
    pub fn trend(&self) -> Option<&'static str> {
        const N: usize = 5;
        if self.rssi.len() < N * 2 {
            return None;
        }
        let values: Vec<f64> = self.rssi.iter().map(|(_, r)| *r as f64).collect();
        let recent = &values[values.len() - N..];
        let before = &values[values.len() - 2 * N..values.len() - N];
        let avg = |s: &[f64]| s.iter().sum::<f64>() / s.len() as f64;
        let diff = avg(recent) - avg(before);
        Some(if diff > 3.0 {
            "closer"
        } else if diff < -3.0 {
            "farther"
        } else {
            "steady"
        })
    }

    /// Median gap between recent advertisements, in milliseconds.
    pub fn advertising_interval_ms(&self) -> Option<u64> {
        if self.sightings.len() < 4 {
            return None;
        }
        let mut gaps: Vec<u64> = self
            .sightings
            .iter()
            .zip(self.sightings.iter().skip(1))
            .map(|(a, b)| b - a)
            .collect();
        gaps.sort_unstable();
        Some(gaps[gaps.len() / 2])
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MfrView {
    pub company_id: String,
    pub company_name: String,
    pub data_hex: String,
    pub ascii: Option<String>,
    pub decoded: Vec<Field>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SvcDataView {
    pub uuid: String,
    pub name: Option<String>,
    pub data_hex: String,
    pub ascii: Option<String>,
    pub decoded: Vec<Field>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ServiceView {
    pub uuid: String,
    pub name: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceView {
    pub id: String,
    pub word: String,
    pub name: String,
    pub local_name: Option<String>,
    pub advertisement_name: Option<String>,
    pub manufacturers: Vec<MfrView>,
    pub service_data: Vec<SvcDataView>,
    pub services: Vec<ServiceView>,
    pub rssi: Option<i16>,
    pub tx_power: Option<i16>,
    pub appearance: Option<u16>,
    pub appearance_label: Option<String>,
    pub address: Option<String>,
    pub address_type: Option<String>,
    pub connectable: Option<bool>,
    pub gatt: Option<GattInfo>,
    pub first_seen: u64,
    pub last_seen: u64,
    pub seen_count: u64,
    pub active: bool,
    pub distance_m: Option<f64>,
    pub adv_interval_ms: Option<u64>,
    pub rssi_history: Vec<i16>,
    pub rssi_trend: Option<String>,
    pub likely_random: bool,
    pub random_reason: Option<String>,
}

pub fn service_view(uuid: &str) -> ServiceView {
    let uuid = ids::normalize_uuid(uuid);
    ServiceView {
        name: ids::service_name(&uuid),
        uuid,
    }
}

impl DeviceView {
    pub fn build(d: &Device, live: Option<&Live>, now: u64) -> DeviceView {
        let manufacturers: Vec<MfrView> = d
            .manufacturers
            .iter()
            .map(|m| {
                let bytes = m.bytes();
                MfrView {
                    company_id: ids::company_id_hex(m.company_id),
                    company_name: ids::company_name(m.company_id),
                    data_hex: m.data_hex.clone(),
                    ascii: decode::ascii_view(&bytes),
                    decoded: decode::decode_manufacturer(m.company_id, &bytes),
                }
            })
            .collect();

        let service_data: Vec<SvcDataView> = d
            .service_data
            .iter()
            .map(|s| {
                let bytes = s.bytes();
                SvcDataView {
                    name: ids::service_name(&s.uuid),
                    data_hex: hex(&bytes),
                    ascii: decode::ascii_view(&bytes),
                    decoded: decode::decode_service_data(&s.uuid, &bytes),
                    uuid: s.uuid.clone(),
                }
            })
            .collect();

        let ibeacon = d
            .manufacturers
            .iter()
            .find_map(|m| decode::ibeacon_power(m.company_id, &m.bytes()));
        let active = now.saturating_sub(d.last_seen) < ACTIVE_WINDOW_MS;
        let appearance = d.appearance.or(d.gatt.as_ref().and_then(|g| g.appearance));
        let random_reason = decode::likely_random(d);

        DeviceView {
            id: d.id.clone(),
            word: d.word.clone(),
            name: d.name.clone(),
            local_name: d.local_name.clone(),
            advertisement_name: d.advertisement_name.clone(),
            manufacturers,
            service_data,
            services: d.services.iter().map(|s| service_view(s)).collect(),
            rssi: d.rssi,
            tx_power: d.tx_power,
            appearance,
            appearance_label: appearance.and_then(decode::appearance_label),
            address: d.address.clone(),
            address_type: d.address_type.clone(),
            connectable: d.connectable,
            gatt: d.gatt.clone(),
            first_seen: d.first_seen,
            last_seen: d.last_seen,
            seen_count: d.seen_count,
            active,
            // An estimate is only meaningful for a device we can hear right now.
            distance_m: d
                .rssi
                .filter(|_| active)
                .map(|r| decode::estimate_distance_m(r, d.tx_power, ibeacon)),
            adv_interval_ms: live.and_then(|l| l.advertising_interval_ms()),
            rssi_history: live.map(|l| l.rssi_history()).unwrap_or_default(),
            rssi_trend: live.and_then(|l| l.trend()).map(str::to_string),
            likely_random: random_reason.is_some(),
            random_reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rssi_history_is_throttled_to_one_per_second() {
        let mut live = Live::default();
        for t in 0..10 {
            live.record(1000 + t * 100, Some(-60), true);
        }
        assert_eq!(live.rssi_history().len(), 1);
        live.record(2500, Some(-61), true);
        assert_eq!(live.rssi_history().len(), 2);
    }

    #[test]
    fn trend_detects_approach() {
        let mut live = Live::default();
        for i in 0..5 {
            live.record(i * 1000, Some(-80), false);
        }
        for i in 5..10 {
            live.record(i * 1000, Some(-60), false);
        }
        assert_eq!(live.trend(), Some("closer"));
    }

    #[test]
    fn interval_is_the_median_gap() {
        let mut live = Live::default();
        for t in [0, 100, 200, 300, 1300] {
            live.record(t, None, true);
        }
        assert_eq!(live.advertising_interval_ms(), Some(100));
    }
}
