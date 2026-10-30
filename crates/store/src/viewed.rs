//! Files marked viewed locally (GitLab has no API), keyed by PR and path with the diff fingerprint.

use super::{Store, StoreResult};
use rusqlite::params;
use std::collections::HashMap;

impl Store {
    pub fn set_viewed(&self, pr: &str, path: &str, fp: Option<&str>) -> StoreResult<()> {
        match fp {
            Some(fp) => self.c().execute("INSERT OR REPLACE INTO viewed(pr, path, fp) VALUES (?1, ?2, ?3)", params![pr, path, fp])?,
            None => self.c().execute("DELETE FROM viewed WHERE pr=?1 AND path=?2", params![pr, path])?,
        };
        Ok(())
    }

    /// Path to the fingerprint it was viewed at.
    pub fn viewed(&self, pr: &str) -> StoreResult<HashMap<String, String>> {
        let c = self.c();
        let mut st = c.prepare("SELECT path, fp FROM viewed WHERE pr=?1")?;
        let rows = st.query_map([pr], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewed_roundtrip() {
        let s = Store::in_memory().unwrap();
        s.set_viewed("pr", "a.rs", Some("f1")).unwrap();
        s.set_viewed("pr", "b.rs", Some("f2")).unwrap();
        s.set_viewed("pr", "b.rs", None).unwrap();
        let v = s.viewed("pr").unwrap();
        assert_eq!(v.get("a.rs").map(String::as_str), Some("f1"));
        assert!(!v.contains_key("b.rs"));
    }
}
