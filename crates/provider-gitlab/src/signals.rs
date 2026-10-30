//! Commits since my last review, from the newest MR notes: my last review (note, approval or requested
//! changes, see [`last_review_at`]) against the "added N commits" system notes after it.

use gv_core::*;
use serde_json::{json, Value};

use super::versions::last_review_at;
use super::{time, GitLabProvider};

/// Commits in an "added N commit(s)" push note.
fn pushed_commits(note: &Value) -> Option<u32> {
    if note["system"] != true {
        return None;
    }
    let rest = note["body"].as_str()?.strip_prefix("added ")?;
    let (n, tail) = rest.split_once(' ')?;
    tail.starts_with("commit").then(|| n.parse().ok())?
}

/// Commits pushed after my last review, or None when I have not reviewed (in these notes).
pub(crate) fn since_my_review(notes: &[Value], me: &str) -> Option<u32> {
    let at = last_review_at(&[json!({ "notes": notes })], me)?;
    Some(
        notes
            .iter()
            .filter(|n| n["created_at"].is_string() && time(&n["created_at"]) > at)
            .filter_map(pushed_commits)
            .sum(),
    )
}

impl GitLabProvider {
    /// One request, only for open MRs I reviewed and did not write.
    pub(crate) async fn review_signal(
        &self,
        base: &str,
        cr: &ChangeRequest,
        me: &str,
    ) -> Option<u32> {
        let reviewed = my_review_state(cr, me).is_some();
        if !reviewed || !cr.state.is_active() || cr.author.username.eq_ignore_ascii_case(me) {
            return None;
        }
        let v = self
            .get(&format!(
                "{base}/notes?sort=desc&order_by=created_at&per_page=100"
            ))
            .await
            .ok()?;
        since_my_review(v.as_array()?, me)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(who: &str, at: &str, system: bool, body: &str) -> Value {
        json!({"author": {"username": who}, "created_at": at, "system": system, "body": body})
    }

    #[test]
    fn counts_pushes_after_my_approval() {
        let mut notes = vec![
            note(
                "dev",
                "2026-10-01T09:00:00Z",
                true,
                "added 2 commits\n\n<ul><li>a</li></ul>",
            ),
            note(
                "me",
                "2026-10-01T10:00:00Z",
                true,
                "approved this merge request",
            ),
        ];
        assert_eq!(since_my_review(&notes, "me"), Some(0));
        notes.push(note(
            "dev",
            "2026-10-02T10:00:00Z",
            true,
            "added 1 commit\n\n<ul></ul>",
        ));
        notes.push(note("dev", "2026-10-03T10:00:00Z", true, "added 2 commits"));
        notes.push(note(
            "dev",
            "2026-10-03T11:00:00Z",
            false,
            "added 9 commits, please look",
        ));
        assert_eq!(since_my_review(&notes, "me"), Some(3));
        assert_eq!(since_my_review(&notes, "other"), None);
    }

    #[test]
    fn approved_mr_moves_to_needs_you_after_a_push() {
        let mut cr = crate::parse_mr(
            "gitlab.com",
            &json!({"iid": 3, "state": "opened", "web_url": "https://gitlab.com/g/p/-/merge_requests/3",
                    "author": {"username": "dev"}, "has_conflicts": false}),
        );
        cr.reviewers = vec![Review {
            user: User::named("me"),
            state: ReviewState::Approved,
        }];
        let mut notes = vec![note(
            "me",
            "2026-10-01T10:00:00Z",
            true,
            "approved this merge request",
        )];
        cr.signals.since_my_review = since_my_review(&notes, "me");
        assert_eq!(classify(&cr, Some("me")), Section::Reviewed);
        notes.push(note("dev", "2026-10-02T10:00:00Z", true, "added 2 commits"));
        cr.signals.since_my_review = since_my_review(&notes, "me");
        assert_eq!(needs_reason(&cr, "me"), Some(NeedsReason::ReReview(2)));
    }
}
