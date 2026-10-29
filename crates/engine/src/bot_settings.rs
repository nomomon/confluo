//! The bot filter as configured in the desktop app's `settings.json`, so the CLI hides the same PRs.

use gv_core::BotFilter;
use serde_json::Value;

/// Read `hide_bots`, `hidden_authors` and `hidden_branch_prefixes` from `<data_dir>/settings.json`.
/// Missing keys fall back to [`BotFilter::default`].
pub fn bot_filter_from_settings() -> BotFilter {
    let v = std::fs::read_to_string(crate::data_dir().join("settings.json")).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok());
    v.map(|v| from_json(&v)).unwrap_or_default()
}

fn from_json(v: &Value) -> BotFilter {
    let d = BotFilter::default();
    let list = |k: &str, dflt: Vec<String>| v[k].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or(dflt);
    BotFilter {
        enabled: v["hide_bots"].as_bool().unwrap_or(d.enabled),
        authors: list("hidden_authors", d.authors),
        branch_prefixes: list("hidden_branch_prefixes", d.branch_prefixes),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_settings_keys() {
        assert_eq!(from_json(&serde_json::json!({})), BotFilter::default());
        let f = from_json(&serde_json::json!({"hide_bots": false, "hidden_authors": ["x"], "hidden_branch_prefixes": []}));
        assert!(!f.enabled);
        assert_eq!(f.authors, vec!["x"]);
        assert!(f.branch_prefixes.is_empty());
    }
}
