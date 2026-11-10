use std::collections::HashMap;
use std::sync::OnceLock;

const WORDS_JSON: &str = include_str!("../data/words.json");
const COMPANIES_JSON: &str = include_str!("../data/companies.json");
const SERVICES_JSON: &str = include_str!("../data/services.json");

fn words() -> &'static Vec<String> {
    static WORDS: OnceLock<Vec<String>> = OnceLock::new();
    WORDS.get_or_init(|| serde_json::from_str(WORDS_JSON).expect("valid words.json"))
}

fn companies() -> &'static HashMap<String, String> {
    static COMPANIES: OnceLock<HashMap<String, String>> = OnceLock::new();
    COMPANIES.get_or_init(|| serde_json::from_str(COMPANIES_JSON).expect("valid companies.json"))
}

fn services() -> &'static HashMap<String, String> {
    static SERVICES: OnceLock<HashMap<String, String>> = OnceLock::new();
    SERVICES.get_or_init(|| serde_json::from_str(SERVICES_JSON).expect("valid services.json"))
}

/// Port of `cli/lib/deterministic-id.js`: the id is parsed as a hex integer (as a
/// JS float, like `parseInt(id, 16)`) and used modulo the word list length.
/// Dashes are stripped so a CoreBluetooth UUID maps to the same word noble gives.
pub fn deterministic_word(id: &str) -> String {
    let hex: String = id.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    let words = words();
    let value = u128::from_str_radix(&hex, 16).map(|v| v as f64).unwrap_or(0.0);
    let index = (value % words.len() as f64) as usize;
    words[index].clone()
}

/// Port of `cli/lib/truncate-id.js`.
#[allow(dead_code)]
pub fn truncate_id(id: &str) -> String {
    let chars: Vec<char> = id.chars().collect();
    if chars.len() > 12 {
        let head: String = chars[..4].iter().collect();
        let tail: String = chars[chars.len() - 4..].iter().collect();
        format!("{head}...{tail}")
    } else {
        id.to_string()
    }
}

/// Formats a Bluetooth SIG company identifier as `0x004C`.
pub fn company_id_hex(company_id: u16) -> String {
    format!("0x{:04X}", company_id)
}

/// Company name for a company id, falling back to the hex id.
pub fn company_name(company_id: u16) -> String {
    let key = company_id_hex(company_id);
    match companies().get(&key) {
        Some(name) if !name.is_empty() => name.clone(),
        _ => key,
    }
}

const BASE_UUID_SUFFIX: &str = "-0000-1000-8000-00805f9b34fb";

/// Normalizes a UUID string: Bluetooth-base UUIDs become their short form
/// (`180F`, or 8 digits for 32-bit ones); anything else is the full UUID in
/// uppercase. Accepts the short forms CoreBluetooth returns as well.
pub fn normalize_uuid(uuid: &str) -> String {
    let lower = uuid.trim().to_lowercase();
    if let Some(prefix) = lower.strip_suffix(BASE_UUID_SUFFIX) {
        if prefix.len() == 8 {
            let short = prefix.strip_prefix("0000").unwrap_or(prefix);
            return short.to_uppercase();
        }
    }
    lower.to_uppercase()
}

/// Human-readable name of a (normalized) service UUID, if it is a known one.
pub fn service_name(normalized_uuid: &str) -> Option<String> {
    services().get(normalized_uuid).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_ignores_dashes() {
        assert_eq!(
            deterministic_word("19898c2c-2b91-b123-e8eb-5b20a233c69c"),
            deterministic_word("19898c2c2b91b123e8eb5b20a233c69c")
        );
    }

    #[test]
    fn matches_cli_data() {
        // Taken from a CLI data.json entry: id -> word.
        assert_eq!(deterministic_word("19898c2c2b91b123e8eb5b20a233c69c"), "tahr");
        assert_eq!(deterministic_word("13ddb283edadc308fe6b9c50e8c6eb5c"), "crocodile");
    }

    #[test]
    fn company_lookup() {
        assert_eq!(company_name(0x004C), "Apple, Inc.");
        // Newer than the old embedded list.
        assert_ne!(company_name(0x06A8), "0x06A8");
        assert_eq!(company_name(0xFFFE), "0xFFFE");
    }

    #[test]
    fn truncates() {
        assert_eq!(truncate_id("19898c2c2b91b123e8eb5b20a233c69c"), "1989...c69c");
        assert_eq!(truncate_id("short"), "short");
    }

    #[test]
    fn uuid_normalization() {
        assert_eq!(normalize_uuid("0000180f-0000-1000-8000-00805f9b34fb"), "180F");
        assert_eq!(normalize_uuid("180f"), "180F");
        assert_eq!(normalize_uuid("0000fe2c-0000-1000-8000-00805f9b34fb"), "FE2C");
        assert_eq!(
            normalize_uuid("6e400001-b5a3-f393-e0a9-e50e24dcca9e"),
            "6E400001-B5A3-F393-E0A9-E50E24DCCA9E"
        );
        assert_eq!(service_name("180F").as_deref(), Some("Battery Service"));
    }
}
