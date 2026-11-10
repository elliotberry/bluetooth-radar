mod config;
mod decode;
mod device;
mod ids;
mod query;
mod scanner;
mod scan_btleplug;
#[cfg(target_os = "macos")]
mod scan_native;
mod store;
mod view;

use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{Emitter, Manager, State};

use config::Config;
use device::GattInfo;
use scanner::{ScanStatus, SharedStatus, SharedStore};
use store::Store;
use view::DeviceView;

#[tauri::command]
fn get_devices(store: State<'_, SharedStore>) -> Vec<DeviceView> {
    store.lock().unwrap().snapshot()
}

#[tauri::command]
fn get_scan_status(status: State<'_, SharedStatus>) -> ScanStatus {
    status.lock().unwrap().clone()
}

#[tauri::command]
fn clear_devices(app: tauri::AppHandle, store: State<'_, SharedStore>) {
    let snapshot = {
        let mut store = store.lock().unwrap();
        store.clear();
        store.snapshot()
    };
    let _ = app.emit(scanner::EVENT_DEVICES, snapshot);
}

type SharedConfig = Mutex<Config>;

#[tauri::command]
fn get_config(config: State<'_, SharedConfig>) -> serde_json::Value {
    config.lock().unwrap().all()
}

#[tauri::command]
fn set_config(
    config: State<'_, SharedConfig>,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    config.lock().unwrap().set(&key, value)
}

#[derive(Serialize)]
struct AppInfo {
    /// "macos", "windows" or "linux"
    platform: &'static str,
}

#[tauri::command]
fn get_app_info() -> AppInfo {
    AppInfo {
        platform: std::env::consts::OS,
    }
}

/// Connects to a device, reads its standard info characteristics and stores
/// them. Only runs when the user asks for it.
#[tauri::command]
async fn query_device(
    app: tauri::AppHandle,
    store: State<'_, SharedStore>,
    id: String,
) -> Result<GattInfo, String> {
    let info = query::query_device(&id).await?;
    let snapshot = {
        let mut store = store.lock().unwrap();
        if !store.set_gatt(&id, info.clone()) {
            return Err("That device is no longer in the list".to_string());
        }
        store.flush();
        store.snapshot()
    };
    let _ = app.emit(scanner::EVENT_DEVICES, snapshot);
    Ok(info)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .setup(|app| {
            // The database lives in the user's data directory under a fixed
            // name, so it survives reinstalling the app.
            let db_dir = store::data_dir(&app.path().data_dir()?);
            // The first version kept a JSON file under the app's identifier;
            // it is imported once if the database is still empty.
            let legacy_json = app.path().app_data_dir()?.join("devices.json");
            let store: SharedStore = Arc::new(Mutex::new(Store::open(
                &db_dir.join("ble-radar.db"),
                Some(&legacy_json),
            )));
            let status: SharedStatus = Arc::new(Mutex::new(scanner::initial_status()));

            app.manage(Mutex::new(Config::open(&db_dir.join("config.json"))));
            app.manage(store.clone());
            app.manage(status.clone());

            tauri::async_runtime::spawn(scanner::run(app.handle().clone(), store, status));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_devices,
            get_scan_status,
            get_app_info,
            get_config,
            set_config,
            clear_devices,
            query_device
        ])
        .build(tauri::generate_context!())
        .expect("error while building BLE Radar");

    app.run(|handle, event| {
        // Save anything still pending (up to a second of sightings) on quit.
        if let tauri::RunEvent::Exit = event {
            if let Some(store) = handle.try_state::<SharedStore>() {
                store.lock().unwrap().flush();
            }
        }
    });
}
