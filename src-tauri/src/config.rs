//! User interface settings (column order, panel state, ...), kept in a small
//! JSON file next to the database so they survive reinstalling the app.
//!
//! The backend does not interpret the values; the UI owns the keys.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

pub struct Config {
    path: PathBuf,
    values: Map<String, Value>,
}

impl Config {
    /// Loads the file at `path`. A missing or unreadable file gives defaults.
    pub fn open(path: &Path) -> Config {
        let values = std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .and_then(|value| match value {
                Value::Object(map) => Some(map),
                _ => None,
            })
            .unwrap_or_default();
        Config {
            path: path.to_path_buf(),
            values,
        }
    }

    pub fn all(&self) -> Value {
        Value::Object(self.values.clone())
    }

    /// Sets one key and writes the file.
    pub fn set(&mut self, key: &str, value: Value) -> Result<(), String> {
        self.values.insert(key.to_string(), value);
        self.save()
    }

    fn save(&self) -> Result<(), String> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(&self.values).map_err(|e| e.to_string())?;
        // Write to a temporary file first so a crash cannot leave a half-written config.
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &self.path).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ble-radar-config-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("config.json")
    }

    #[test]
    fn missing_file_gives_empty_config() {
        let config = Config::open(&temp_path("missing"));
        assert_eq!(config.all(), json!({}));
    }

    #[test]
    fn values_persist_across_reopen() {
        let path = temp_path("persist");
        let mut config = Config::open(&path);
        config.set("columnOrder", json!(["id", "word"])).unwrap();
        config.set("detailOpen", json!(false)).unwrap();

        let reopened = Config::open(&path);
        assert_eq!(
            reopened.all(),
            json!({ "columnOrder": ["id", "word"], "detailOpen": false })
        );
    }

    #[test]
    fn corrupt_file_falls_back_to_defaults() {
        let path = temp_path("corrupt");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "not json").unwrap();
        assert_eq!(Config::open(&path).all(), json!({}));
    }
}
