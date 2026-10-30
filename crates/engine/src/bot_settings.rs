//! The bot filter as configured in the desktop app's `settings.json`, so the CLI hides the same PRs.

use gv_core::BotFilter;
use serde_json::Value;

/// Read `hide_bots`, `hidden_authors` and `hidden_branch_prefixes` from `<data_dir>/settings.json`.
/// Missing keys fall back to [`BotFilter::default`].
pub fn bot_filter_from_settings() -> BotFilter {
    let v = std::fs::read_to_string(crate::data_dir().join("settings.json")).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok());
    v.map(|v| from_json(&v)).unwrap_or_default()
}

/// `stale_days` from `settings.json`: a number, `null` for never, missing for the default.
pub fn stale_days_from_settings() -> Option<u32> {
    let v = std::fs::read_to_string(crate::data_dir().join("settings.json")).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok());
    stale_days(v.as_ref())
}

fn stale_days(v: Option<&Value>) -> Option<u32> {
    match v.and_then(|v| v.get("stale_days")) {
        None => Some(gv_core::DEFAULT_STALE_DAYS),
        Some(x) => x.as_u64().map(|d| d as u32),
    }
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

    #[test]
    fn reads_stale_days() {
        assert_eq!(stale_days(None), Some(60));
        assert_eq!(stale_days(Some(&serde_json::json!({"stale_days": null}))), None);
        assert_eq!(stale_days(Some(&serde_json::json!({"stale_days": 90}))), Some(90));
    }
}
