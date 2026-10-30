//! The Entra tenant of an Azure DevOps organization, from the `x-vss-resourcetenant` header
//! of an unauthenticated request (what the az devops extension does). Cached per org.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

fn cache() -> &'static Mutex<HashMap<String, Option<String>>> {
    static C: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();
    C.get_or_init(Default::default)
}

/// A tenant id from the header value; the all-zero id means "no tenant" (MSA-backed org).
pub(crate) fn parse_tenant(v: &str) -> Option<String> {
    let v = v.trim();
    let ok = v.len() == 36 && v.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    (ok && v.chars().any(|c| c != '0' && c != '-')).then(|| v.to_ascii_lowercase())
}

pub(crate) async fn org_tenant(host: &str, org: &str) -> Option<String> {
    let key = format!("{host}/{}", org.to_ascii_lowercase());
    if let Some(t) = cache().lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
        return t.clone();
    }
    let url = format!("https://{host}/{org}/_apis/projects");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?;
    // A network error is not cached, so the next lookup tries again.
    let resp = client.get(&url).send().await.ok()?;
    let t = resp
        .headers()
        .get("x-vss-resourcetenant")
        .and_then(|v| v.to_str().ok())
        .and_then(parse_tenant);
    cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(key, t.clone());
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tenant_header() {
        assert_eq!(
            parse_tenant(" 0A1B2C3D-0000-4000-8000-00000000ABCD ").as_deref(),
            Some("0a1b2c3d-0000-4000-8000-00000000abcd")
        );
        assert_eq!(parse_tenant("00000000-0000-0000-0000-000000000000"), None);
        assert_eq!(parse_tenant("nope"), None);
    }
}
