//! Every provider request goes through here: a shared per-host [`Governor`], ETag revalidation
//! for GETs, rate limit handling, and request counting (`GV_DEBUG_HTTP=1`).

mod governor;
mod stats;

pub use governor::{backoff, governor, paused, read_signal, Bucket, Governor, Signal, Verdict};
pub use stats::{endpoint_label, reset_stats, stats, stats_summary, Stat};

use gv_core::{ProviderError, ProviderResult};
use reqwest::header::{HeaderMap, ETAG, IF_NONE_MATCH};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// A fully read response.
#[derive(Debug, Clone)]
pub struct Resp {
    pub status: u16,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

impl Resp {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }
}

/// Bodies larger than this are not kept for ETag revalidation.
const ETAG_MAX_BODY: usize = 2 * 1024 * 1024;
const ETAG_MAX_ENTRIES: usize = 400;

fn etags() -> &'static Mutex<HashMap<String, (String, Resp)>> {
    static M: OnceLock<Mutex<HashMap<String, (String, Resp)>>> = OnceLock::new();
    M.get_or_init(Default::default)
}

/// Send through the host's governor. GETs revalidate with `If-None-Match` when an ETag is known.
/// On 429 or a secondary-limit 403 the host is paused; the request is retried once if the pause
/// is short, otherwise `ProviderError::RateLimited` is returned without touching the network again.
pub async fn send(req: reqwest::RequestBuilder) -> ProviderResult<Resp> {
    let (client, request) = req.build_split();
    let request = request.map_err(|e| ProviderError::Network(e.to_string()))?;
    let url = request.url().clone();
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    let endpoint = endpoint_label(&url);
    let gov = governor(&host);
    let is_get = request.method() == reqwest::Method::GET;
    let key = url.to_string();

    for attempt in 0..2 {
        let permit = match gov.acquire().await {
            Ok(p) => p,
            Err(until) => {
                stats::record(&host, &endpoint, stats::Outcome::Blocked);
                return Err(ProviderError::RateLimited { host, until });
            }
        };
        let mut r = request.try_clone().ok_or_else(|| ProviderError::Network("request body not clonable".into()))?;
        let cached = if is_get { etags().lock().unwrap_or_else(|e| e.into_inner()).get(&key).cloned() } else { None };
        if let Some((tag, _)) = &cached {
            if let Ok(v) = tag.parse() {
                r.headers_mut().insert(IF_NONE_MATCH, v);
            }
        }
        let resp = client.execute(r).await.map_err(|e| ProviderError::Network(e.to_string()));
        let resp = match resp {
            Ok(x) => x,
            Err(e) => {
                stats::record(&host, &endpoint, stats::Outcome::Sent);
                return Err(e);
            }
        };
        let status = resp.status().as_u16();
        let headers = resp.headers().clone();
        let body = resp.bytes().await.map_err(|e| ProviderError::Network(e.to_string()))?.to_vec();
        drop(permit);
        let sig = read_signal(status, |n| headers.get(n).and_then(|v| v.to_str().ok()).map(str::to_string), &body);
        if let Verdict::Limited(wait) = gov.observe(&sig) {
            stats::record(&host, &endpoint, stats::Outcome::Limited);
            if attempt == 0 && wait <= governor::MAX_INLINE_WAIT {
                continue;
            }
            let until = gov.paused_until().unwrap_or_else(chrono::Utc::now);
            return Err(ProviderError::RateLimited { host, until });
        }
        if status == 304 {
            if let Some((_, r)) = cached {
                stats::record(&host, &endpoint, stats::Outcome::NotModified);
                return Ok(r);
            }
        }
        stats::record(&host, &endpoint, stats::Outcome::Sent);
        let out = Resp { status, headers, body };
        if is_get && (200..300).contains(&status) && out.body.len() <= ETAG_MAX_BODY {
            if let Some(tag) = out.header(ETAG.as_str()) {
                let mut m = etags().lock().unwrap_or_else(|e| e.into_inner());
                if m.len() >= ETAG_MAX_ENTRIES {
                    m.clear();
                }
                m.insert(key.clone(), (tag.to_string(), out.clone()));
            }
        }
        return Ok(out);
    }
    let until = gov.paused_until().unwrap_or_else(chrono::Utc::now);
    Err(ProviderError::RateLimited { host, until })
}

/// Whether request debugging is on (`GV_DEBUG_HTTP=1`).
pub fn debug_enabled() -> bool {
    std::env::var("GV_DEBUG_HTTP").is_ok_and(|v| !v.is_empty() && v != "0")
}
