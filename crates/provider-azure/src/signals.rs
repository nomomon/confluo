//! Commits since my vote: my latest vote time (system `VoteUpdate` threads) against the PR iterations
//! (pushes) created after it, with their commits. Read only when the source commit moved.

use chrono::{DateTime, Utc};
use gv_core::*;
use serde_json::Value;

use super::parse::{opt_time, same, vote_events};
use super::{AzureProvider, API};

/// When I last voted, from the PR threads.
pub(crate) fn last_vote_at(threads: &[Value], me: &User) -> Option<DateTime<Utc>> {
    vote_events(threads)
        .into_iter()
        .filter(|e| same(&e.actor, me))
        .map(|e| e.at)
        .max()
}

/// Commits of the iterations pushed after `at` (an iteration without its commits counts as one).
pub(crate) fn commits_after(iterations: &[Value], at: DateTime<Utc>) -> u32 {
    iterations
        .iter()
        .filter(|i| opt_time(&i["createdDate"]).is_some_and(|t| t > at))
        .map(|i| i["commits"].as_array().map_or(1, |c| c.len().max(1) as u32))
        .sum()
}

impl AzureProvider {
    /// Two requests, only for active PRs I voted on and did not write.
    pub(crate) async fn review_signal(&self, cr: &ChangeRequest, me: &User) -> Option<u32> {
        let mine = cr.reviewers.iter().find(|r| same(&r.user, me))?;
        if mine.state == ReviewState::Pending || !cr.state.is_active() || same(&cr.author, me) {
            return None;
        }
        let base = self.pr_url(&cr.id).ok()?;
        let threads = self.get(&format!("{base}/threads?{API}")).await.ok()?;
        let at = last_vote_at(threads["value"].as_array()?, me)?;
        let its = self
            .get(&format!("{base}/iterations?includeCommits=true&{API}"))
            .await
            .ok()?;
        Some(commits_after(its["value"].as_array()?, at))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn vote(id: &str, v: i64, at: &str) -> Value {
        json!({"publishedDate": at, "properties": {
            "CodeReviewThreadType": {"$value": "VoteUpdate"},
            "CodeReviewVoteResult": {"$value": v.to_string()},
            "CodeReviewVotedByIdentity": {"$value": "1"}},
            "identities": {"1": {"id": id, "uniqueName": id, "displayName": id}}})
    }

    #[test]
    fn new_iterations_after_my_vote() {
        let me = User {
            id: Some("me-id".into()),
            ..User::named("me@x")
        };
        let threads = vec![
            vote("me-id", 10, "2026-10-01T10:00:00Z"),
            vote("other", 10, "2026-10-04T10:00:00Z"),
        ];
        let at = last_vote_at(&threads, &me).unwrap();
        let mut its = vec![json!({"createdDate": "2026-10-01T09:00:00Z", "commits": [{}, {}]})];
        assert_eq!(commits_after(&its, at), 0);
        its.push(json!({"createdDate": "2026-10-02T09:00:00Z", "commits": [{}, {}, {}]}));
        its.push(json!({"createdDate": "2026-10-03T09:00:00Z"}));
        assert_eq!(commits_after(&its, at), 4);
    }

    #[test]
    fn approved_pr_moves_to_needs_you_after_a_push_and_on_vote_reset() {
        let me = User {
            id: Some("me-id".into()),
            ..User::named("me@x")
        };
        let pr = |vote: i64| {
            json!({"pullRequestId": 5, "status": "active", "mergeStatus": "succeeded",
                   "repository": {"name": "r", "project": {"name": "p"}},
                   "createdBy": {"id": "dev", "uniqueName": "dev@x"},
                   "creationDate": "2026-10-01T00:00:00Z",
                   "reviewers": [{"id": "me-id", "uniqueName": "me@x", "vote": vote}]})
        };
        let mut cr = crate::parse::parse_pr("dev.azure.com", "o", &pr(10), &me);
        cr.signals.since_my_review = Some(0);
        assert_eq!(classify(&cr, Some("me@x")), Section::Reviewed);
        cr.signals.since_my_review = Some(commits_after(
            &[json!({"createdDate": "2026-10-03T00:00:00Z", "commits": [{}]})],
            "2026-10-02T00:00:00Z".parse().unwrap(),
        ));
        assert_eq!(needs_reason(&cr, "me@x"), Some(NeedsReason::ReReview(1)));
        // A reset vote (policy "reset votes on push") reads as a fresh request.
        let reset = crate::parse::parse_pr("dev.azure.com", "o", &pr(0), &me);
        assert_eq!(
            needs_reason(&reset, "me@x"),
            Some(NeedsReason::ReviewRequested)
        );
        let mut conflicted = pr(10);
        conflicted["mergeStatus"] = json!("conflicts");
        assert!(
            crate::parse::parse_pr("dev.azure.com", "o", &conflicted, &me)
                .signals
                .conflicts
        );
    }
}
