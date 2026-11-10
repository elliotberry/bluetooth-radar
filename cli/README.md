# ble-radar

Scans for nearby Bluetooth Low Energy devices and lists them. There are two front ends that do the same thing:

## CLI (Node)

```sh
npm install
node index.js
```

Live, colored list in the terminal. State is saved to `./data.json`.

## Desktop (Tauri)

A desktop app in `desktop/` with a Rust (btleplug) scanner and a React UI: sortable table of active devices, a collapsible inactive section, search, and a detail pane. State is saved to the app data directory (`devices.json`), separate from the CLI's `data.json`.

Requirements: Node, Rust, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/).

```sh
cd desktop
npm install
npm run tauri dev      # run in development
npm run tauri build    # produce an app bundle
```

On macOS you will be asked to allow Bluetooth access. In `tauri dev` the prompt belongs to the terminal or IDE that launched it; the built app gets its own prompt.

The company and word lists are shared with the CLI (`lib/companies.js`, `lib/deterministic-id.js`). After changing them, regenerate the data embedded in the Rust app with `npm run gen-data` inside `desktop/`.
