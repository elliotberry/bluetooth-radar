use std::collections::HashMap;
use std::sync::OnceLock;

const WORDS_JSON: &str = include_str!("../data/words.json");
const COMPANIES_JSON: &str = include_str!("../data/companies.json");

fn words() -> &'static Vec<String> {
    static WORDS: OnceLock<Vec<String>> = OnceLock::new();
    WORDS.get_or_init(|| serde_json::from_str(WORDS_JSON).expect("valid words.json"))
}

fn companies() -> &'static HashMap<String, String> {
    static COMPANIES: OnceLock<HashMap<String, String>> = OnceLock::new();
    COMPANIES.get_or_init(|| serde_json::from_str(COMPANIES_JSON).expect("valid companies.json"))
}

/// Port of `lib/deterministic-id.js`: the id is parsed as a hex integer (as a
/// JS float, like `parseInt(id, 16)`) and used modulo the word list length.
/// Dashes are stripped so a CoreBluetooth UUID maps to the same word noble gives.
pub fn deterministic_word(id: &str) -> String {
    let hex: String = id.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    let words = words();
    let value = u128::from_str_radix(&hex, 16).map(|v| v as f64).unwrap_or(0.0);
    let index = (value % words.len() as f64) as usize;
    words[index].clone()
}

/// Port of `lib/truncate-id.js`.
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

/// Company name for a company id, falling back to the hex id like the CLI does.
pub fn company_name(company_id: u16) -> String {
    let key = company_id_hex(company_id);
    match companies().get(&key) {
        Some(name) if !name.is_empty() => name.clone(),
        _ => key,
    }
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
        assert_eq!(company_name(0xABCD), "0xABCD");
    }

    #[test]
    fn truncates() {
        assert_eq!(truncate_id("19898c2c2b91b123e8eb5b20a233c69c"), "1989...c69c");
        assert_eq!(truncate_id("short"), "short");
    }
}
