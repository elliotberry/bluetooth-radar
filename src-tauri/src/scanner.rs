//! Scanning orchestration shared by the platform backends: status reporting,
//! persisting, and pushing snapshots to the UI.
//!
//! macOS uses a direct CoreBluetooth scanner (`scan_native`); other
//! platforms use btleplug (`scan_btleplug`).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::device::Sighting;
use crate::store::Store;

pub const EVENT_DEVICES: &str = "devices-updated";
pub const EVENT_STATUS: &str = "scan-status";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStatus {
    /// "starting", "scanning", "bluetooth-off", "unauthorized", "no-adapter" or "error"
    pub state: String,
    pub message: Option<String>,
}

impl ScanStatus {
    pub fn new(state: &str, message: Option<String>) -> Self {
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

pub fn set_status(app: &AppHandle, status: &SharedStatus, next: ScanStatus) {
    let mut current = status.lock().unwrap();
    if current.state != next.state || current.message != next.message {
        *current = next.clone();
        let _ = app.emit(EVENT_STATUS, next);
    }
}

pub fn emit_and_persist(app: &AppHandle, store: &SharedStore) {
    let snapshot = {
        let mut store = store.lock().unwrap();
        store.flush();
        store.snapshot()
    };
    let _ = app.emit(EVENT_DEVICES, snapshot);
}

pub fn record(store: &SharedStore, sighting: Sighting, counted: bool) {
    store.lock().unwrap().record(sighting, counted);
}

/// One scan session on the current platform's backend.
#[cfg(target_os = "macos")]
async fn scan_once(
    app: &AppHandle,
    store: &SharedStore,
    status: &SharedStatus,
) -> Result<(), String> {
    use crate::scan_native::{NativeEvent, NativeScanner, NativeState};

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    // Dropping this at the end of the session stops the scan.
    let _scanner = NativeScanner::start(tx).map_err(|e| {
        set_status(app, status, ScanStatus::new("unauthorized", Some(e.clone())));
        e
    })?;

    let mut tick = tokio::time::interval(Duration::from_secs(1));
    loop {
        tokio::select! {
            event = rx.recv() => match event {
                // Every callback is one advertisement.
                Some(NativeEvent::Advert(sighting)) => record(store, sighting, true),
                Some(NativeEvent::State(state)) => {
                    let next = match state {
                        NativeState::PoweredOn => ScanStatus::new("scanning", None),
                        NativeState::PoweredOff => {
                            ScanStatus::new("bluetooth-off", Some("Bluetooth is turned off".into()))
                        }
                        NativeState::Unauthorized => ScanStatus::new(
                            "unauthorized",
                            Some("Bluetooth permission denied".into()),
                        ),
                        NativeState::Unsupported => ScanStatus::new(
                            "no-adapter",
                            Some("Bluetooth LE is not supported on this Mac".into()),
                        ),
                        NativeState::Other => ScanStatus::new("starting", None),
                    };
                    set_status(app, status, next);
                }
                None => return Ok(()),
            },
            _ = tick.tick() => emit_and_persist(app, store),
        }
    }
}

#[cfg(not(target_os = "macos"))]
async fn scan_once(
    app: &AppHandle,
    store: &SharedStore,
    status: &SharedStatus,
) -> Result<(), String> {
    crate::scan_btleplug::scan_once(app, store, status).await
}

/// Runs forever: scans, and retries if something goes wrong.
pub async fn run(app: AppHandle, store: SharedStore, status: SharedStatus) {
    // Keep the UI refreshing (and persisting) even while not scanning.
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
        if let Err(message) = scan_once(&app, &store, &status).await {
            let current = status.lock().unwrap().state.clone();
            if current != "unauthorized" {
                set_status(&app, &status, ScanStatus::new("error", Some(message)));
            }
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
