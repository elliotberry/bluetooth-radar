# ble-radar

Scans for nearby Bluetooth Low Energy devices and lists them. Two front ends:

- **Desktop app** (this folder): Tauri 2, Rust scanner, React UI.
- **CLI** (`cli/`): the original Node scanner. See `cli/`; it keeps its own `data.json`.

## Desktop app

Requirements: Node, Rust, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/).

```sh
npm install
npm run tauri dev      # run in development
npm run tauri build    # produce an app bundle
```

On macOS you are asked to allow Bluetooth. In `tauri dev` the prompt belongs to the terminal or IDE that launched it.

### Scanning

- **macOS:** talks to CoreBluetooth directly, so every advertisement is counted exactly and "connectable" is known.
- **Windows / Linux:** uses btleplug (compiles, but is untested). The MAC address and address type are only available here; macOS hides them.

### What it shows

Word + ID, name (advertised and cached), company, decoded manufacturer data (Apple Continuity, Microsoft CDP, Ruuvi, Govee, iBeacon, Eddystone and more), service data and named services, appearance, RSSI with trend and sparkline, a rough distance estimate, advertising interval, times seen, and a "likely rotating address" flag.

The detail pane has an opt-in **Query device** button that connects to a connectable device and reads the standard Device Information and Battery GATT characteristics. It is intrusive, so it only runs when you click it.

### Data storage

Devices are stored in SQLite at `<user data dir>/ble-radar/ble-radar.db` (on macOS, `~/Library/Application Support/ble-radar/`). The location does not depend on the app bundle, so data survives reinstalling. Raw bytes are stored and decoded at display time, so decoder improvements apply to old data. The schema is versioned with `PRAGMA user_version` (currently 2) and old databases are migrated on launch. A legacy `devices.json` is imported on first run.

### Reference data

`npm run gen-data` regenerates the embedded word list, company names and service names in `src-tauri/data/`. It reads the word list and legacy company list from `cli/lib/` and downloads the Bluetooth SIG numbers from the [Nordic bluetooth-numbers-database](https://github.com/NordicSemiconductor/bluetooth-numbers-database), falling back to the local lists if offline.

### CI

`.github/workflows/build-macos.yml` builds unsigned Apple Silicon and Intel `.dmg` files on every push to `main` and uploads them as workflow artifacts. The workflow only runs on GitHub; this repo's `origin` is currently Bitbucket.
