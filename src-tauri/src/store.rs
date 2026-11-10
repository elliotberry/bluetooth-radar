use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};

use crate::device::{Device, ACTIVE_WINDOW_MS};

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

const SCHEMA_VERSION: i64 = 1;

/// Device table kept in memory for fast snapshots and persisted to SQLite.
pub struct Store {
    devices: HashMap<String, Device>,
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
    conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
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
        "SELECT id, word, name, company_id, company_name, manufacturer_data_hex,
                rssi, tx_power, address, services, first_seen, last_seen, seen_count
         FROM devices",
    )?;
    let rows = stmt.query_map([], |r| {
        let services: String = r.get(9)?;
        Ok(Device {
            id: r.get(0)?,
            word: r.get(1)?,
            name: r.get(2)?,
            company_id: r.get(3)?,
            company_name: r.get(4)?,
            manufacturer_data_hex: r.get(5)?,
            rssi: r.get(6)?,
            tx_power: r.get(7)?,
            address: r.get(8)?,
            services: serde_json::from_str(&services).unwrap_or_default(),
            first_seen: r.get::<_, i64>(10)? as u64,
            last_seen: r.get::<_, i64>(11)? as u64,
            seen_count: r.get::<_, i64>(12)? as u64,
            active: false,
        })
    })?;
    let mut devices = HashMap::new();
    for row in rows {
        let device = row?;
        devices.insert(device.id.clone(), device);
    }
    Ok(devices)
}

impl Store {
    /// Opens (creating if needed) the database at `db_path`. If the database
    /// is empty, imports the JSON file written by earlier versions, if any.
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
        let Ok(old) = serde_json::from_str::<HashMap<String, Device>>(&text) else {
            return;
        };
        for (id, mut device) in old {
            device.active = false;
            self.dirty.insert(id.clone());
            self.devices.insert(id, device);
        }
        self.flush();
    }

    pub fn upsert(&mut self, device: Device) {
        let id = device.id.clone();
        match self.devices.get_mut(&id) {
            Some(existing) => existing.merge(device),
            None => {
                self.devices.insert(id.clone(), device);
            }
        }
        self.dirty.insert(id);
    }

    pub fn clear(&mut self) {
        self.devices.clear();
        self.dirty.clear();
        self.cleared = true;
    }

    /// All devices with `active` recomputed from `last_seen`.
    pub fn snapshot(&self) -> Vec<Device> {
        let now = now_ms();
        self.devices
            .values()
            .map(|d| {
                let mut d = d.clone();
                d.active = now.saturating_sub(d.last_seen) < ACTIVE_WINDOW_MS;
                d
            })
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
                    id, word, name, company_id, company_name, manufacturer_data_hex,
                    rssi, tx_power, address, services, first_seen, last_seen, seen_count
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                 ON CONFLICT(id) DO UPDATE SET
                    word = excluded.word,
                    name = excluded.name,
                    company_id = excluded.company_id,
                    company_name = excluded.company_name,
                    manufacturer_data_hex = excluded.manufacturer_data_hex,
                    rssi = excluded.rssi,
                    tx_power = excluded.tx_power,
                    address = excluded.address,
                    services = excluded.services,
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
                    d.company_id,
                    d.company_name,
                    d.manufacturer_data_hex,
                    d.rssi,
                    d.tx_power,
                    d.address,
                    serde_json::to_string(&d.services).unwrap_or_else(|_| "[]".into()),
                    d.first_seen as i64,
                    d.last_seen as i64,
                    d.seen_count as i64,
                ])?;
            }
        }
        tx.commit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::Device;

    fn device(id: &str, count: u64) -> Device {
        Device {
            id: id.into(),
            word: "tahr".into(),
            name: "Thing".into(),
            company_id: Some("0x004C".into()),
            company_name: Some("Apple, Inc.".into()),
            manufacturer_data_hex: Some("0a0b".into()),
            rssi: Some(-60),
            tx_power: None,
            address: None,
            services: vec!["180f".into()],
            first_seen: 1000,
            last_seen: 2000,
            seen_count: count,
            active: true,
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
            store.upsert(device("a", 1));
            store.upsert(device("a", 2)); // merged: count becomes 3
            store.flush();
        }
        let store = Store::open(&path, None);
        let devices = store.snapshot();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].seen_count, 3);
        assert_eq!(devices[0].services, vec!["180f".to_string()]);
        assert_eq!(devices[0].first_seen, 1000);
    }

    #[test]
    fn clear_is_persisted() {
        let path = temp_db("clear");
        let mut store = Store::open(&path, None);
        store.upsert(device("a", 1));
        store.flush();
        store.clear();
        store.flush();
        drop(store);
        assert!(Store::open(&path, None).snapshot().is_empty());
    }

    #[test]
    fn imports_legacy_json_once() {
        let path = temp_db("legacy");
        let json_path = path.with_file_name("devices.json");
        let mut old = HashMap::new();
        old.insert("a".to_string(), device("a", 5));
        std::fs::write(&json_path, serde_json::to_string(&old).unwrap()).unwrap();

        let store = Store::open(&path, Some(&json_path));
        assert_eq!(store.snapshot().len(), 1);
        drop(store);

        // A second open reads from SQLite and does not double-import.
        let store = Store::open(&path, Some(&json_path));
        let devices = store.snapshot();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].seen_count, 5);
    }
}
