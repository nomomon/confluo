//! Request counters per host and endpoint, for `GV_DEBUG_HTTP=1` summaries.

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Copy)]
pub(crate) enum Outcome {
    /// Went over the network with a full response.
    Sent,
    /// 304 answered from the ETag cache.
    NotModified,
    /// Answered 429 or a secondary limit.
    Limited,
    /// Not sent: the host was paused.
    Blocked,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stat {
    pub sent: u32,
    pub not_modified: u32,
    pub limited: u32,
    pub blocked: u32,
}

fn table() -> &'static Mutex<BTreeMap<(String, String), Stat>> {
    static T: OnceLock<Mutex<BTreeMap<(String, String), Stat>>> = OnceLock::new();
    T.get_or_init(Default::default)
}

pub(crate) fn record(host: &str, endpoint: &str, o: Outcome) {
    let mut t = table().lock().unwrap_or_else(|e| e.into_inner());
    let s = t
        .entry((host.to_string(), endpoint.to_string()))
        .or_default();
    match o {
        Outcome::Sent => s.sent += 1,
        Outcome::NotModified => s.not_modified += 1,
        Outcome::Limited => s.limited += 1,
        Outcome::Blocked => s.blocked += 1,
    }
}

/// Snapshot of all counters.
pub fn stats() -> BTreeMap<(String, String), Stat> {
    table().lock().unwrap_or_else(|e| e.into_inner()).clone()
}

pub fn reset_stats() {
    table().lock().unwrap_or_else(|e| e.into_inner()).clear();
}

/// Human summary: per host total, then endpoints sorted by count.
pub fn stats_summary() -> String {
    let t = stats();
    let mut hosts: BTreeMap<&str, Vec<(&str, Stat)>> = BTreeMap::new();
    for ((h, e), s) in &t {
        hosts.entry(h).or_default().push((e, *s));
    }
    let mut out = String::from("HTTP requests\n");
    let total: u32 = t
        .values()
        .map(|s| s.sent + s.not_modified + s.limited)
        .sum();
    for (h, mut eps) in hosts {
        eps.sort_by_key(|(e, s)| {
            (
                std::cmp::Reverse(s.sent + s.not_modified + s.limited),
                e.to_string(),
            )
        });
        let n: u32 = eps
            .iter()
            .map(|(_, s)| s.sent + s.not_modified + s.limited)
            .sum();
        out.push_str(&format!("  {h}: {n}\n"));
        for (e, s) in eps {
            let mut extra = vec![];
            if s.not_modified > 0 {
                extra.push(format!("{} not modified", s.not_modified));
            }
            if s.limited > 0 {
                extra.push(format!("{} limited", s.limited));
            }
            if s.blocked > 0 {
                extra.push(format!("{} held back", s.blocked));
            }
            let extra = if extra.is_empty() {
                String::new()
            } else {
                format!(" ({})", extra.join(", "))
            };
            out.push_str(&format!(
                "    {:>4}  {e}{extra}\n",
                s.sent + s.not_modified + s.limited
            ));
        }
    }
    out.push_str(&format!("  total: {total}\n"));
    out
}

/// Path with ids collapsed: numbers become `:n`, encoded project paths `:project`,
/// owner/repo after `repos` become `:owner/:repo`, long hashes `:hash`.
pub fn endpoint_label(url: &reqwest::Url) -> String {
    let segs: Vec<&str> = url.path().split('/').filter(|s| !s.is_empty()).collect();
    let mut out: Vec<String> = Vec::with_capacity(segs.len());
    let mut i = 0;
    while i < segs.len() {
        let s = segs[i];
        let prev = i.checked_sub(1).map(|j| segs[j]);
        let label = if matches!(prev, Some("repos")) && i + 1 < segs.len() {
            i += 1;
            ":owner/:repo".to_string()
        } else if matches!(prev, Some("users" | "groups")) {
            ":name".to_string()
        } else if s.chars().all(|c| c.is_ascii_digit()) {
            ":n".to_string()
        } else if s.contains('%') {
            ":project".to_string()
        } else if s.len() >= 24 {
            ":hash".to_string()
        } else {
            s.to_string()
        };
        out.push(label);
        i += 1;
    }
    format!("/{}", out.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l(u: &str) -> String {
        endpoint_label(&reqwest::Url::parse(u).unwrap())
    }

    #[test]
    fn labels_collapse_ids() {
        assert_eq!(
            l("https://gitlab.com/api/v4/projects/g%2Fp/merge_requests/12/reviewers"),
            "/api/v4/projects/:project/merge_requests/:n/reviewers"
        );
        assert_eq!(
            l("https://api.github.com/repos/a/b/pulls/3/files?page=1"),
            "/repos/:owner/:repo/pulls/:n/files"
        );
        assert_eq!(
            l("https://gitlab.com/api/v4/merge_requests?scope=all"),
            "/api/v4/merge_requests"
        );
        assert_eq!(
            l("https://avatars.githubusercontent.com/u/123?v=4"),
            "/u/:n"
        );
    }
}
