//! "Changes since your last review": my last review time (my notes, approval or requested changes),
//! the MR version (`GET .../versions`) that was current then, and the compare to the head.

use chrono::{DateTime, Utc};
use gv_core::*;
use serde_json::Value;

use super::{changed_file, enc, time, GitLabProvider};

/// When I last reviewed: my latest note, or system note by me (approved, requested changes).
pub(crate) fn last_review_at(discussions: &[Value], me: &str) -> Option<DateTime<Utc>> {
    discussions
        .iter()
        .flat_map(|d| d["notes"].as_array().into_iter().flatten())
        .filter(|n| {
            n["author"]["username"]
                .as_str()
                .is_some_and(|u| u.eq_ignore_ascii_case(me))
        })
        .filter(|n| {
            n["system"] != true || {
                let b = n["body"].as_str().unwrap_or_default();
                b.starts_with("approved") || b.starts_with("requested changes")
            }
        })
        .filter(|n| n["created_at"].is_string())
        .map(|n| time(&n["created_at"]))
        .max()
}

/// Head sha of the newest version created at or before `at` (versions come newest first, but
/// are sorted here anyway).
pub(crate) fn version_head_at(versions: &[Value], at: DateTime<Utc>) -> Option<String> {
    versions
        .iter()
        .filter(|v| v["created_at"].is_string() && time(&v["created_at"]) <= at)
        .max_by_key(|v| time(&v["created_at"]))
        .and_then(|v| v["head_commit_sha"].as_str())
        .map(str::to_string)
}

impl GitLabProvider {
    /// Head I last reviewed; one request, only when I reviewed at all.
    pub(crate) async fn reviewed_head(
        &self,
        base: &str,
        discussions: &[Value],
        me: &str,
    ) -> Option<String> {
        let at = last_review_at(discussions, me)?;
        let v = self.get(&format!("{base}/versions")).await.ok()?;
        version_head_at(v.as_array()?, at)
    }

    /// Files changed between two commits (`repository/compare`, read-only).
    pub(crate) async fn compare_files(
        &self,
        id: &ChangeRequestId,
        from: &str,
        to: &str,
    ) -> ProviderResult<Vec<ChangedFile>> {
        let v = self
            .get(&format!(
                "/projects/{}/repository/compare?from={from}&to={to}",
                enc(&id.repo.path)
            ))
            .await?;
        Ok(v["diffs"]
            .as_array()
            .map(|a| a.iter().map(changed_file).collect())
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn picks_the_version_current_at_my_review() {
        let versions = vec![
            json!({"id": 3, "head_commit_sha": "h3", "created_at": "2026-10-03T10:00:00Z"}),
            json!({"id": 2, "head_commit_sha": "h2", "created_at": "2026-10-02T10:00:00Z"}),
            json!({"id": 1, "head_commit_sha": "h1", "created_at": "2026-10-01T10:00:00Z"}),
        ];
        let at = |s: &str| s.parse::<DateTime<Utc>>().unwrap();
        assert_eq!(
            version_head_at(&versions, at("2026-10-02T12:00:00Z")).as_deref(),
            Some("h2")
        );
        assert_eq!(
            version_head_at(&versions, at("2026-10-02T10:00:00Z")).as_deref(),
            Some("h2")
        );
        assert_eq!(
            version_head_at(&versions, at("2026-10-05T00:00:00Z")).as_deref(),
            Some("h3")
        );
        assert_eq!(version_head_at(&versions, at("2026-09-01T00:00:00Z")), None);
    }

    #[test]
    fn last_review_is_my_latest_note_or_approval() {
        let note = |who: &str, at: &str, system: bool, body: &str| json!({"author": {"username": who}, "created_at": at, "system": system, "body": body});
        let ds = vec![
            json!({"notes": [
                note("me", "2026-10-01T10:00:00Z", false, "looks off"),
                note("other", "2026-10-04T10:00:00Z", false, "fixed"),
            ]}),
            json!({"notes": [
                note("Me", "2026-10-02T10:00:00Z", true, "approved this merge request"),
                note("me", "2026-10-03T10:00:00Z", true, "added 1 commit"),
            ]}),
        ];
        assert_eq!(
            last_review_at(&ds, "me").map(|t| t.to_rfc3339()),
            Some("2026-10-02T10:00:00+00:00".into())
        );
        assert_eq!(last_review_at(&ds, "nobody"), None);
    }
}
