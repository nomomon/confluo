//! List signals: commits since my last review and merge conflicts, read in the list query.

use gv_core::Signals;
use serde_json::Value;

/// How many recent commits the list query reads to count commits since my review.
pub(crate) const RECENT: u32 = 20;

/// Extra PR fields for `me` (a login; anything else is dropped so it cannot break the query).
pub(crate) fn fields(me: &str) -> String {
    let login: String = me
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    format!(
        r#"mergeable
  myReviews: reviews(last: 1, author: "{login}", states: [APPROVED, CHANGES_REQUESTED, COMMENTED]) {{ nodes {{ commit {{ oid }} }} }}
  recent: commits(last: {RECENT}) {{ nodes {{ commit {{ oid }} }} }}"#
    )
}

/// Signals from a PR node queried with [`fields`].
pub(crate) fn parse(n: &Value) -> Signals {
    let reviewed = n["myReviews"]["nodes"][0]["commit"]["oid"].as_str();
    let recent: Vec<&str> = n["recent"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| c["commit"]["oid"].as_str())
        .collect();
    let since_my_review = reviewed.map(|oid| match recent.iter().rposition(|c| *c == oid) {
        Some(i) => (recent.len() - 1 - i) as u32,
        // Older than the window, or the branch was rewritten: at least everything we can see.
        None => recent.len().max(1) as u32,
    });
    Signals {
        since_my_review,
        conflicts: n["mergeable"] == "CONFLICTING",
        threads_need_me: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gv_core::{classify, needs_reason, NeedsReason, Section};
    use serde_json::json;

    fn node(reviewed: Option<&str>, commits: &[&str]) -> Value {
        json!({
            "mergeable": "MERGEABLE",
            "myReviews": {"nodes": reviewed.map(|o| vec![json!({"commit": {"oid": o}})]).unwrap_or_default()},
            "recent": {"nodes": commits.iter().map(|o| json!({"commit": {"oid": o}})).collect::<Vec<_>>()},
        })
    }

    #[test]
    fn counts_commits_after_my_review() {
        assert_eq!(parse(&node(None, &["a", "b"])).since_my_review, None);
        assert_eq!(
            parse(&node(Some("b"), &["a", "b"])).since_my_review,
            Some(0)
        );
        assert_eq!(
            parse(&node(Some("a"), &["a", "b", "c", "d"])).since_my_review,
            Some(3)
        );
        assert_eq!(
            parse(&node(Some("gone"), &["x", "y"])).since_my_review,
            Some(2)
        );
    }

    #[test]
    fn conflicts_and_query() {
        assert!(parse(&json!({"mergeable": "CONFLICTING"})).conflicts);
        assert!(!parse(&json!({"mergeable": "UNKNOWN"})).conflicts);
        assert!(fields("me\"){x}").contains("author: \"mex\""));
    }

    #[test]
    fn approved_pr_moves_to_needs_you_after_a_push() {
        let p = crate::GitHubProvider::new("github.com", "x".into());
        let pr = |commits: &[&str]| {
            let mut n = node(Some("h1"), commits);
            let extra = json!({
                "number": 9, "state": "OPEN", "isDraft": false, "repository": {"nameWithOwner": "a/b"},
                "author": {"login": "bob"}, "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-02T00:00:00Z",
                "latestReviews": {"nodes": [{"state": "APPROVED", "author": {"login": "me"}}]},
            });
            n.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            p.parse_pr(&n, "me").unwrap()
        };
        let before = pr(&["h0", "h1"]);
        assert_eq!(classify(&before, Some("me")), Section::Reviewed);
        let after = pr(&["h1", "h2", "h3"]);
        assert_eq!(needs_reason(&after, "me"), Some(NeedsReason::ReReview(2)));
        assert_eq!(classify(&after, Some("me")), Section::NeedsYou);
    }
}
