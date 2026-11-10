use std::sync::{Arc, Mutex};
use std::time::Duration;

use btleplug::api::{Central, CentralEvent, CentralState, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager};
use futures::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::device::Device;
use crate::store::{now_ms, Store};

pub const EVENT_DEVICES: &str = "devices-updated";
pub const EVENT_STATUS: &str = "scan-status";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStatus {
    /// "starting", "scanning", "bluetooth-off", "no-adapter" or "error"
    pub state: String,
    pub message: Option<String>,
}

impl ScanStatus {
    fn new(state: &str, message: Option<String>) -> Self {
        ScanStatus {
            state: state.to_string(),
            message,
        }
    }
}

pub type SharedStore = Arc<Mutex<Store>>;
pub type SharedStatus = Arc<Mutex<ScanStatus>>;

pub fn initial_status() -> ScanStatus {
    ScanStatus::new("starting", None)
}

fn set_status(app: &AppHandle, status: &SharedStatus, next: ScanStatus) {
    let mut current = status.lock().unwrap();
    if current.state != next.state || current.message != next.message {
        *current = next.clone();
        let _ = app.emit(EVENT_STATUS, next);
    }
}

fn emit_and_persist(app: &AppHandle, store: &SharedStore) {
    let snapshot = {
        let mut store = store.lock().unwrap();
        store.flush();
        store.snapshot()
    };
    let _ = app.emit(EVENT_DEVICES, snapshot);
}

async fn record(
    adapter: &Adapter,
    id: &btleplug::platform::PeripheralId,
    store: &SharedStore,
    counted: bool,
) {
    let Ok(peripheral) = adapter.peripheral(id).await else {
        return;
    };
    let Ok(Some(props)) = peripheral.properties().await else {
        return;
    };
    let device = Device::from_properties(&id.to_string(), &props, now_ms(), counted);
    store.lock().unwrap().upsert(device);
}

/// Scans on one adapter until its event stream ends or an error occurs.
async fn scan_adapter(
    app: &AppHandle,
    adapter: Adapter,
    store: &SharedStore,
    status: &SharedStatus,
) -> Result<(), btleplug::Error> {
    // Subscribe before scanning so no early events are missed.
    let mut events = adapter.events().await?;

    match adapter.adapter_state().await {
        Ok(CentralState::PoweredOff) => {
            set_status(
                app,
                status,
                ScanStatus::new("bluetooth-off", Some("Bluetooth is turned off".into())),
            );
        }
        _ => {
            adapter.start_scan(ScanFilter::default()).await?;
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
                        record(&adapter, &id, store, true).await;
                    }
                    // Extra detail about an advertisement already counted above.
                    CentralEvent::ManufacturerDataAdvertisement { id, .. }
                    | CentralEvent::RssiUpdate { id, .. } => {
                        record(&adapter, &id, store, false).await;
                    }
                    CentralEvent::StateUpdate(CentralState::PoweredOn) => {
                        adapter.start_scan(ScanFilter::default()).await?;
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

/// Runs forever: finds an adapter, scans, and retries if something goes wrong.
pub async fn run(app: AppHandle, store: SharedStore, status: SharedStatus) {
    // Keep the UI refreshing (and persisting) even while no adapter is available.
    let ticker_app = app.clone();
    let ticker_store = store.clone();
    let ticker_status = status.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        loop {
            tick.tick().await;
            if ticker_status.lock().unwrap().state != "scanning" {
                emit_and_persist(&ticker_app, &ticker_store);
            }
        }
    });

    loop {
        let result: Result<(), String> = async {
            let manager = Manager::new().await.map_err(|e| e.to_string())?;
            let adapters = manager.adapters().await.map_err(|e| e.to_string())?;
            let Some(adapter) = adapters.into_iter().next() else {
                set_status(
                    &app,
                    &status,
                    ScanStatus::new("no-adapter", Some("No Bluetooth adapter found".into())),
                );
                return Ok(());
            };
            scan_adapter(&app, adapter, &store, &status)
                .await
                .map_err(|e| e.to_string())
        }
        .await;

        if let Err(message) = result {
            set_status(&app, &status, ScanStatus::new("error", Some(message)));
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
