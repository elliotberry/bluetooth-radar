mod device;
mod ids;
mod scanner;
mod store;

use std::sync::{Arc, Mutex};

use tauri::{Emitter, Manager, State};

use device::Device;
use scanner::{ScanStatus, SharedStatus, SharedStore};
use store::Store;

#[tauri::command]
fn get_devices(store: State<'_, SharedStore>) -> Vec<Device> {
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .setup(|app| {
            // The database lives in the user's data directory under a fixed
            // name, so it survives reinstalling the app.
            let db_dir = store::data_dir(&app.path().data_dir()?);
            // Earlier versions kept a JSON file under the app's identifier;
            // it is imported once if the database is still empty.
            let legacy_json = app.path().app_data_dir()?.join("devices.json");
            let store: SharedStore = Arc::new(Mutex::new(Store::open(
                &db_dir.join("ble-radar.db"),
                Some(&legacy_json),
            )));
            let status: SharedStatus = Arc::new(Mutex::new(scanner::initial_status()));

            app.manage(store.clone());
            app.manage(status.clone());

            tauri::async_runtime::spawn(scanner::run(app.handle().clone(), store, status));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_devices,
            get_scan_status,
            clear_devices
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
