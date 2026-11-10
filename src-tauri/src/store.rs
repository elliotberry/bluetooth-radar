use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};
use serde::Deserialize;

use crate::device::{Device, GattInfo, Mfr, Sighting};
use crate::view::{DeviceView, Live};

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Directory that holds the database. It sits directly in the user's data
/// directory (not under the app's bundle identifier), so it survives
/// uninstalling and reinstalling the app.
pub fn data_dir(base: &Path) -> PathBuf {
    base.join("ble-radar")
}

const SCHEMA_VERSION: i64 = 2;

/// Device table kept in memory for fast snapshots and persisted to SQLite.
pub struct Store {
    devices: HashMap<String, Device>,
    live: HashMap<String, Live>,
    conn: Connection,
    /// Ids changed since the last flush.
    dirty: HashSet<String>,
    /// Set when the table was cleared and the delete is not yet flushed.
    cleared: bool,
}

fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;

    if version < 1 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS devices (
                id                     TEXT PRIMARY KEY,
                word                   TEXT NOT NULL,
                name                   TEXT NOT NULL,
                company_id             TEXT,
                company_name           TEXT,
                manufacturer_data_hex  TEXT,
                rssi                   INTEGER,
                tx_power               INTEGER,
                address                TEXT,
                services               TEXT NOT NULL DEFAULT '[]',
                first_seen             INTEGER NOT NULL,
                last_seen              INTEGER NOT NULL,
                seen_count             INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS devices_last_seen ON devices(last_seen);",
        )?;
    }

    if version < 2 {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(
            "ALTER TABLE devices ADD COLUMN local_name TEXT;
             ALTER TABLE devices ADD COLUMN advertisement_name TEXT;
             ALTER TABLE devices ADD COLUMN manufacturers TEXT NOT NULL DEFAULT '[]';
             ALTER TABLE devices ADD COLUMN service_data TEXT NOT NULL DEFAULT '[]';
             ALTER TABLE devices ADD COLUMN appearance INTEGER;
             ALTER TABLE devices ADD COLUMN address_type TEXT;
             ALTER TABLE devices ADD COLUMN connectable INTEGER;
             ALTER TABLE devices ADD COLUMN gatt TEXT;",
        )?;

        // Carry the single company/manufacturer-data pair over to the list form.
        let rows: Vec<(String, Option<String>, Option<String>, String)> = {
            let mut stmt = tx.prepare(
                "SELECT id, company_id, manufacturer_data_hex, name FROM devices",
            )?;
            let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
            rows.collect::<Result<_, _>>()?
        };
        for (id, company_id, data_hex, name) in rows {
            let mfrs = match (company_id.as_deref().and_then(parse_company_id), data_hex) {
                (Some(company_id), Some(data_hex)) => vec![Mfr { company_id, data_hex }],
                _ => Vec::new(),
            };
            // Older versions only had one merged name; treat it as the local name.
            let local_name = (name != "Unknown").then_some(name);
            tx.execute(
                "UPDATE devices SET manufacturers = ?1, local_name = ?2 WHERE id = ?3",
                params![serde_json::to_string(&mfrs).unwrap_or_else(|_| "[]".into()), local_name, id],
            )?;
        }

        tx.execute_batch(
            "ALTER TABLE devices DROP COLUMN company_id;
             ALTER TABLE devices DROP COLUMN company_name;
             ALTER TABLE devices DROP COLUMN manufacturer_data_hex;",
        )?;
        tx.commit()?;
    }

    conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}

fn parse_company_id(s: &str) -> Option<u16> {
    u16::from_str_radix(s.trim_start_matches("0x").trim_start_matches("0X"), 16).ok()
}

fn open_db(path: &Path) -> rusqlite::Result<Connection> {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let conn = Connection::open(path)?;
    // WAL keeps the database consistent if the app is killed mid-write.
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    migrate(&conn)?;
    Ok(conn)
}

fn load_devices(conn: &Connection) -> rusqlite::Result<HashMap<String, Device>> {
    let mut stmt = conn.prepare(
        "SELECT id, word, name, local_name, advertisement_name, manufacturers, service_data,
                services, rssi, tx_power, appearance, address, address_type, connectable,
                gatt, first_seen, last_seen, seen_count
         FROM devices",
    )?;
    let rows = stmt.query_map([], |r| {
        let manufacturers: String = r.get(5)?;
        let service_data: String = r.get(6)?;
        let services: String = r.get(7)?;
        let appearance: Option<i64> = r.get(10)?;
        let connectable: Option<i64> = r.get(13)?;
        let gatt: Option<String> = r.get(14)?;
        Ok(Device {
            id: r.get(0)?,
            word: r.get(1)?,
            name: r.get(2)?,
            local_name: r.get(3)?,
            advertisement_name: r.get(4)?,
            manufacturers: serde_json::from_str(&manufacturers).unwrap_or_default(),
            service_data: serde_json::from_str(&service_data).unwrap_or_default(),
            services: serde_json::from_str(&services).unwrap_or_default(),
            rssi: r.get(8)?,
            tx_power: r.get(9)?,
            appearance: appearance.map(|a| a as u16),
            address: r.get(11)?,
            address_type: r.get(12)?,
            connectable: connectable.map(|c| c != 0),
            gatt: gatt.and_then(|g| serde_json::from_str(&g).ok()),
            first_seen: r.get::<_, i64>(15)? as u64,
            last_seen: r.get::<_, i64>(16)? as u64,
            seen_count: r.get::<_, i64>(17)? as u64,
        })
    })?;
    let mut devices = HashMap::new();
    for row in rows {
        let device = row?;
        devices.insert(device.id.clone(), device);
    }
    Ok(devices)
}

/// The JSON file format of the first version, before SQLite.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyDevice {
    id: String,
    word: String,
    name: String,
    company_id: Option<String>,
    manufacturer_data_hex: Option<String>,
    rssi: Option<i16>,
    tx_power: Option<i16>,
    address: Option<String>,
    #[serde(default)]
    services: Vec<String>,
    first_seen: u64,
    last_seen: u64,
    #[serde(default)]
    seen_count: u64,
}

impl From<LegacyDevice> for Device {
    fn from(l: LegacyDevice) -> Device {
        let manufacturers = match (l.company_id.as_deref().and_then(parse_company_id), l.manufacturer_data_hex) {
            (Some(company_id), Some(data_hex)) => vec![Mfr { company_id, data_hex }],
            _ => Vec::new(),
        };
        let local_name = (l.name != "Unknown").then(|| l.name.clone());
        Device {
            id: l.id,
            word: l.word,
            name: l.name,
            local_name,
            advertisement_name: None,
            manufacturers,
            service_data: Vec::new(),
            services: l.services,
            rssi: l.rssi,
            tx_power: l.tx_power,
            appearance: None,
            address: l.address,
            address_type: None,
            connectable: None,
            gatt: None,
            first_seen: l.first_seen,
            last_seen: l.last_seen,
            seen_count: l.seen_count,
        }
    }
}

impl Store {
    /// Opens (creating if needed) the database at `db_path`. If the database
    /// is empty, imports the JSON file written by the first version, if any.
    pub fn open(db_path: &Path, legacy_json: Option<&Path>) -> Store {
        let conn = open_db(db_path).unwrap_or_else(|err| {
            // Keep the app usable; data just won't persist this session.
            eprintln!("Could not open {}: {err}. Using memory only.", db_path.display());
            let conn = Connection::open_in_memory().expect("in-memory sqlite");
            migrate(&conn).expect("migrate in-memory db");
            conn
        });
        let devices = load_devices(&conn).unwrap_or_else(|err| {
            eprintln!("Could not read devices: {err}");
            HashMap::new()
        });

        let mut store = Store {
            devices,
            live: HashMap::new(),
            conn,
            dirty: HashSet::new(),
            cleared: false,
        };

        if store.devices.is_empty() {
            if let Some(path) = legacy_json {
                store.import_legacy_json(path);
            }
        }
        store
    }

    fn import_legacy_json(&mut self, path: &Path) {
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        let Ok(old) = serde_json::from_str::<HashMap<String, LegacyDevice>>(&text) else {
            return;
        };
        for (id, legacy) in old {
            self.dirty.insert(id.clone());
            self.devices.insert(id, legacy.into());
        }
        self.flush();
    }

    /// Records one sighting. `counted` marks whether it is a distinct
    /// advertisement that should add to the device's sighting count.
    pub fn record(&mut self, sighting: Sighting, counted: bool) {
        let now = now_ms();
        let id = sighting.id.clone();
        let rssi = sighting.rssi;
        self.live.entry(id.clone()).or_default().record(now, rssi, counted);
        match self.devices.get_mut(&id) {
            Some(existing) => existing.merge(sighting, now, counted),
            None => {
                self.devices
                    .insert(id.clone(), Device::from_sighting(sighting, now, counted));
            }
        }
        self.dirty.insert(id);
    }

    /// Stores the result of a GATT query.
    pub fn set_gatt(&mut self, id: &str, gatt: GattInfo) -> bool {
        match self.devices.get_mut(id) {
            Some(device) => {
                device.gatt = Some(gatt);
                self.dirty.insert(id.to_string());
                true
            }
            None => false,
        }
    }

    pub fn clear(&mut self) {
        self.devices.clear();
        self.live.clear();
        self.dirty.clear();
        self.cleared = true;
    }

    /// All devices, decoded, with live values filled in.
    pub fn snapshot(&self) -> Vec<DeviceView> {
        let now = now_ms();
        self.devices
            .values()
            .map(|d| DeviceView::build(d, self.live.get(&d.id), now))
            .collect()
    }

    /// Writes pending changes to SQLite in a single transaction.
    pub fn flush(&mut self) {
        if !self.cleared && self.dirty.is_empty() {
            return;
        }
        if let Err(err) = self.write_pending() {
            // Leave `dirty` and `cleared` as they are so the next flush retries.
            eprintln!("Error writing to database: {err}");
            return;
        }
        self.dirty.clear();
        self.cleared = false;
    }

    fn write_pending(&mut self) -> rusqlite::Result<()> {
        let tx = self.conn.transaction()?;
        if self.cleared {
            tx.execute("DELETE FROM devices", [])?;
        }
        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO devices (
                    id, word, name, local_name, advertisement_name, manufacturers, service_data,
                    services, rssi, tx_power, appearance, address, address_type, connectable,
                    gatt, first_seen, last_seen, seen_count
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
                 ON CONFLICT(id) DO UPDATE SET
                    word = excluded.word,
                    name = excluded.name,
                    local_name = excluded.local_name,
                    advertisement_name = excluded.advertisement_name,
                    manufacturers = excluded.manufacturers,
                    service_data = excluded.service_data,
                    services = excluded.services,
                    rssi = excluded.rssi,
                    tx_power = excluded.tx_power,
                    appearance = excluded.appearance,
                    address = excluded.address,
                    address_type = excluded.address_type,
                    connectable = excluded.connectable,
                    gatt = excluded.gatt,
                    first_seen = excluded.first_seen,
                    last_seen = excluded.last_seen,
                    seen_count = excluded.seen_count",
            )?;
            for id in &self.dirty {
                let Some(d) = self.devices.get(id) else { continue };
                stmt.execute(params![
                    d.id,
                    d.word,
                    d.name,
                    d.local_name,
                    d.advertisement_name,
                    to_json(&d.manufacturers),
                    to_json(&d.service_data),
                    to_json(&d.services),
                    d.rssi,
                    d.tx_power,
                    d.appearance.map(|a| a as i64),
                    d.address,
                    d.address_type,
                    d.connectable.map(|c| c as i64),
                    d.gatt.as_ref().and_then(|g| serde_json::to_string(g).ok()),
                    d.first_seen as i64,
                    d.last_seen as i64,
                    d.seen_count as i64,
                ])?;
            }
        }
        tx.commit()
    }
}

/// Serializes to a JSON string, falling back to an empty array.
fn to_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "[]".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::{Mfr, SvcData};

    fn sighting(id: &str) -> Sighting {
        Sighting {
            id: id.into(),
            local_name: Some("Thing".into()),
            advertisement_name: Some("Cached".into()),
            manufacturers: vec![Mfr { company_id: 0x4c, data_hex: "100605".into() }],
            service_data: vec![SvcData { uuid: "180F".into(), data_hex: "64".into() }],
            services: vec!["180A".into()],
            rssi: Some(-60),
            appearance: Some(0x00c0),
            address_type: Some("random".into()),
            connectable: Some(true),
            ..Default::default()
        }
    }

    fn temp_db(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ble-radar-test-{name}-{}", now_ms()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("test.db")
    }

    #[test]
    fn persists_across_reopen() {
        let path = temp_db("reopen");
        {
            let mut store = Store::open(&path, None);
            store.record(sighting("a"), true);
            store.record(sighting("a"), true);
            store.record(sighting("a"), false);
            store.set_gatt("a", GattInfo { model_number: Some("M1".into()), ..Default::default() });
            store.flush();
        }
        let store = Store::open(&path, None);
        let devices = store.snapshot();
        assert_eq!(devices.len(), 1);
        let d = &devices[0];
        assert_eq!(d.seen_count, 2);
        assert_eq!(d.local_name.as_deref(), Some("Thing"));
        assert_eq!(d.advertisement_name.as_deref(), Some("Cached"));
        assert_eq!(d.manufacturers[0].company_name, "Apple, Inc.");
        assert_eq!(d.service_data[0].decoded[0].value, "100%");
        assert_eq!(d.appearance_label.as_deref(), Some("Watch"));
        assert_eq!(d.connectable, Some(true));
        assert_eq!(d.address_type.as_deref(), Some("random"));
        assert!(d.likely_random);
        assert_eq!(d.gatt.as_ref().unwrap().model_number.as_deref(), Some("M1"));
        assert_eq!(d.services[0].name.as_deref(), Some("Device Information"));
    }

    #[test]
    fn clear_is_persisted() {
        let path = temp_db("clear");
        let mut store = Store::open(&path, None);
        store.record(sighting("a"), true);
        store.flush();
        store.clear();
        store.flush();
        drop(store);
        assert!(Store::open(&path, None).snapshot().is_empty());
    }

    #[test]
    fn migrates_version_1_database() {
        let path = temp_db("v1");
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE devices (
                    id TEXT PRIMARY KEY, word TEXT NOT NULL, name TEXT NOT NULL,
                    company_id TEXT, company_name TEXT, manufacturer_data_hex TEXT,
                    rssi INTEGER, tx_power INTEGER, address TEXT,
                    services TEXT NOT NULL DEFAULT '[]',
                    first_seen INTEGER NOT NULL, last_seen INTEGER NOT NULL,
                    seen_count INTEGER NOT NULL DEFAULT 0);
                 INSERT INTO devices VALUES
                    ('a','tahr','Old name','0x004C','Apple, Inc.','100605',-70,NULL,NULL,'[]',1,2,7),
                    ('b','crow','Unknown',NULL,NULL,NULL,-80,NULL,NULL,'[]',3,4,1);
                 PRAGMA user_version = 1;",
            )
            .unwrap();
        }
        let store = Store::open(&path, None);
        let mut devices = store.snapshot();
        devices.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(devices[0].seen_count, 7);
        assert_eq!(devices[0].local_name.as_deref(), Some("Old name"));
        assert_eq!(devices[0].manufacturers[0].data_hex, "100605");
        assert_eq!(devices[1].local_name, None);
        assert!(devices[1].manufacturers.is_empty());
    }

    #[test]
    fn imports_legacy_json_once() {
        let path = temp_db("legacy");
        let json_path = path.with_file_name("devices.json");
        std::fs::write(
            &json_path,
            r#"{"a":{"id":"a","word":"tahr","name":"Thing","companyId":"0x004C","companyName":"Apple, Inc.",
               "manufacturerDataHex":"0a0b","rssi":-60,"txPower":null,"address":null,"services":[],
               "firstSeen":1,"lastSeen":2,"seenCount":5,"active":true}}"#,
        )
        .unwrap();

        let store = Store::open(&path, Some(&json_path));
        assert_eq!(store.snapshot().len(), 1);
        drop(store);

        // A second open reads from SQLite and does not double-import.
        let store = Store::open(&path, Some(&json_path));
        let devices = store.snapshot();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].seen_count, 5);
        assert_eq!(devices[0].manufacturers[0].company_id, "0x004C");
    }
}
