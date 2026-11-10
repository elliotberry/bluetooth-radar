//! Cross-platform scanner built on btleplug. Used on Windows and Linux;
//! macOS uses the direct CoreBluetooth scanner in `scan_native` instead (this
//! module still compiles there, so it stays type-checked, but is unused).
#![cfg_attr(target_os = "macos", allow(dead_code))]

use std::time::Duration;

use btleplug::api::{
    AddressType, Central, CentralEvent, CentralState, Manager as _, Peripheral as _,
    PeripheralProperties, ScanFilter,
};
use btleplug::platform::{Adapter, Manager, PeripheralId};
use futures::StreamExt;
use tauri::AppHandle;

use crate::device::{hex, Mfr, Sighting, SvcData};
use crate::ids;
use crate::scanner::{emit_and_persist, record, set_status, ScanStatus, SharedStatus, SharedStore};

fn sighting_from(id: &PeripheralId, props: &PeripheralProperties) -> Sighting {
    let address = props.address.to_string();
    let address = (address != "00:00:00:00:00:00").then_some(address);
    Sighting {
        id: id.to_string().to_lowercase(),
        local_name: props.local_name.clone(),
        advertisement_name: props.advertisement_name.clone(),
        manufacturers: props
            .manufacturer_data
            .iter()
            .map(|(company, data)| Mfr {
                company_id: *company,
                data_hex: hex(data),
            })
            .collect(),
        service_data: props
            .service_data
            .iter()
            .map(|(uuid, data)| SvcData {
                uuid: ids::normalize_uuid(&uuid.to_string()),
                data_hex: hex(data),
            })
            .collect(),
        services: props
            .services
            .iter()
            .map(|s| ids::normalize_uuid(&s.to_string()))
            .collect(),
        rssi: props.rssi,
        tx_power: props.tx_power_level,
        appearance: props.appearance,
        address,
        address_type: props.address_type.map(|t| match t {
            AddressType::Public => "public".to_string(),
            AddressType::Random => "random".to_string(),
        }),
        connectable: None,
    }
}

async fn record_peripheral(adapter: &Adapter, id: &PeripheralId, store: &SharedStore, counted: bool) {
    let Ok(peripheral) = adapter.peripheral(id).await else {
        return;
    };
    let Ok(Some(props)) = peripheral.properties().await else {
        return;
    };
    record(store, sighting_from(id, &props), counted);
}

/// Scans on the first adapter until its event stream ends or an error occurs.
pub async fn scan_once(
    app: &AppHandle,
    store: &SharedStore,
    status: &SharedStatus,
) -> Result<(), String> {
    let manager = Manager::new().await.map_err(|e| e.to_string())?;
    let adapters = manager.adapters().await.map_err(|e| e.to_string())?;
    let Some(adapter) = adapters.into_iter().next() else {
        set_status(
            app,
            status,
            ScanStatus::new("no-adapter", Some("No Bluetooth adapter found".into())),
        );
        return Ok(());
    };

    // Subscribe before scanning so no early events are missed.
    let mut events = adapter.events().await.map_err(|e| e.to_string())?;

    match adapter.adapter_state().await {
        Ok(CentralState::PoweredOff) => set_status(
            app,
            status,
            ScanStatus::new("bluetooth-off", Some("Bluetooth is turned off".into())),
        ),
        _ => {
            adapter
                .start_scan(ScanFilter::default())
                .await
                .map_err(|e| e.to_string())?;
            set_status(app, status, ScanStatus::new("scanning", None));
        }
    }

    let mut tick = tokio::time::interval(Duration::from_secs(1));
    loop {
        tokio::select! {
            event = events.next() => {
                let Some(event) = event else { return Ok(()) };
                match event {
                    // One of these fires per advertisement; only they count as a sighting.
                    CentralEvent::DeviceDiscovered(id) | CentralEvent::DeviceUpdated(id) => {
                        record_peripheral(&adapter, &id, store, true).await;
                    }
                    // Extra detail about an advertisement already counted above.
                    CentralEvent::ManufacturerDataAdvertisement { id, .. }
                    | CentralEvent::ServiceDataAdvertisement { id, .. }
                    | CentralEvent::RssiUpdate { id, .. } => {
                        record_peripheral(&adapter, &id, store, false).await;
                    }
                    CentralEvent::StateUpdate(CentralState::PoweredOn) => {
                        adapter.start_scan(ScanFilter::default()).await.map_err(|e| e.to_string())?;
                        set_status(app, status, ScanStatus::new("scanning", None));
                    }
                    CentralEvent::StateUpdate(CentralState::PoweredOff) => {
                        let _ = adapter.stop_scan().await;
                        set_status(
                            app,
                            status,
                            ScanStatus::new("bluetooth-off", Some("Bluetooth is turned off".into())),
                        );
                    }
                    _ => {}
                }
            }
            _ = tick.tick() => emit_and_persist(app, store),
        }
    }
}
