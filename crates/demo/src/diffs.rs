//! Believable diffs: Rust for API and CLI repos, TypeScript for the web app.

use crate::patch::{added, hunk};
use gv_core::{ChangedFile, FileStatus};

pub const RETRY_RS: &str = include_str!("fixtures/retry.rs.txt");
const DELIVER: &str = include_str!("fixtures/deliver.rs.txt");
const FEED: &str = include_str!("fixtures/feed.tsx.txt");
const ROWS_TS: &str = include_str!("fixtures/rows.ts.txt");

pub const RETRY_PATH: &str = "src/webhooks/retry.rs";
pub const DELIVER_PATH: &str = "src/webhooks/deliver.rs";

const MOD_RS: &str = " pub mod deliver;
+pub mod retry;
 pub mod sign;
 
 pub struct Endpoint {
     pub url: String,
     pub secret: String,
+    /// Overrides the default retry policy.
+    pub retry: Option<retry::RetryPolicy>,
 }";

const TEST_RS: &str = "use acme_api::webhooks::retry::RetryPolicy;
use std::time::Duration;

#[test]
fn delays_stay_under_the_cap() {
    let p = RetryPolicy { max_attempts: 10, base: Duration::from_millis(100), cap: Duration::from_secs(2) };
    for n in 1..10 {
        assert!(p.delay(n).unwrap() <= Duration::from_secs(2));
    }
    assert!(p.delay(10).is_none());
}

#[test]
fn only_transient_statuses_retry() {
    assert!(RetryPolicy::retryable(503));
    assert!(RetryPolicy::retryable(429));
    assert!(!RetryPolicy::retryable(404));
}";

const FEED_ITEM: &str = " import { Avatar } from \"../ui/Avatar\";
 
+/** Every feed row has this height so the list can be virtualized. */
+export const ROW_HEIGHT = 56;
+
 export function FeedItem({ event }: { event: FeedEvent }) {
-  return <li className=\"feed-item\">
+  return <li className=\"feed-item\" style={{ height: ROW_HEIGHT }}>";

fn rust() -> Vec<ChangedFile> {
    vec![
        ChangedFile::from_patch(RETRY_PATH, RETRY_PATH, FileStatus::Added, &added(RETRY_RS)),
        ChangedFile::from_patch(DELIVER_PATH, DELIVER_PATH, FileStatus::Modified, &hunk(1, 1, DELIVER)),
        ChangedFile::from_patch("src/webhooks/mod.rs", "src/webhooks/mod.rs", FileStatus::Modified, &hunk(1, 1, MOD_RS)),
        ChangedFile::from_patch("tests/webhook_retry.rs", "tests/webhook_retry.rs", FileStatus::Added, &added(TEST_RS)),
    ]
}

fn typescript() -> Vec<ChangedFile> {
    let feed = "src/feed/ActivityFeed.tsx";
    let item = "src/feed/FeedItem.tsx";
    let rows = "src/feed/useVirtualRows.ts";
    vec![
        ChangedFile::from_patch(feed, feed, FileStatus::Modified, &hunk(1, 1, FEED)),
        ChangedFile::from_patch(item, item, FileStatus::Modified, &hunk(1, 1, FEED_ITEM)),
        ChangedFile::from_patch(rows, rows, FileStatus::Added, &added(ROWS_TS)),
    ]
}

/// Changed files of a demo PR in `repo_path`.
pub fn files(repo_path: &str) -> Vec<ChangedFile> {
    if repo_path == "acme/web" { typescript() } else { rust() }
}

/// Whole file at head, for added files (the others are shown as diff hunks only).
pub fn content(path: &str) -> Option<String> {
    match path {
        RETRY_PATH => Some(RETRY_RS.into()),
        "src/feed/useVirtualRows.ts" => Some(ROWS_TS.into()),
        "tests/webhook_retry.rs" => Some(TEST_RS.into()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diffs_parse() {
        let r = files("acme/api");
        assert_eq!(r[0].additions as usize, RETRY_RS.lines().count());
        assert!(r[1].additions > 0 && r[1].deletions > 0);
        let t = files("acme/web");
        assert_eq!(t.len(), 3);
        assert!(t.iter().all(|f| !f.hunks.is_empty()));
    }
}
