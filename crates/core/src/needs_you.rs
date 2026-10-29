use crate::{ChangeRequest, ChangeRequestState, ReviewState};

/// Decide whether `me` should act on this change request.
///
/// - As a reviewer: requested and not yet approved / requested changes.
/// - As the author: changes were requested or checks failed.
///
/// Drafts and closed/merged items never need you.
pub fn compute_needs_you(cr: &ChangeRequest, me: &str) -> bool {
    if cr.state != ChangeRequestState::Open {
        return false;
    }
    let is_me = |u: &str| u.eq_ignore_ascii_case(me);
    if is_me(&cr.author.username) {
        let changes_requested = cr.reviewers.iter().any(|r| r.state == ReviewState::ChangesRequested);
        return changes_requested || cr.checks.failed > 0;
    }
    cr.reviewers
        .iter()
        .any(|r| is_me(&r.user.username) && matches!(r.state, ReviewState::Pending | ReviewState::Commented))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use chrono::Utc;

    fn cr(author: &str, reviewers: Vec<(&str, ReviewState)>, failed: u32) -> ChangeRequest {
        ChangeRequest {
            id: ChangeRequestId { repo: RepoRef::new(ProviderKind::GitHub, "github.com", "a/b"), number: 1 },
            title: "t".into(),
            author: User::named(author),
            state: ChangeRequestState::Open,
            source_branch: "f".into(),
            target_branch: "main".into(),
            reviewers: reviewers.into_iter().map(|(u, s)| Review { user: User::named(u), state: s }).collect(),
            checks: CheckSummary { total: failed, passed: 0, failed, pending: 0 },
            labels: vec![],
            comments_count: 0,
            additions: None,
            deletions: None,
            changed_files: None,
            updated_at: Utc::now(),
            created_at: Utc::now(),
            web_url: String::new(),
            needs_you: false,
        }
    }

    #[test]
    fn reviewer_pending() {
        assert!(compute_needs_you(&cr("x", vec![("me", ReviewState::Pending)], 0), "me"));
        assert!(!compute_needs_you(&cr("x", vec![("me", ReviewState::Approved)], 0), "me"));
        assert!(!compute_needs_you(&cr("x", vec![], 0), "me"));
    }

    #[test]
    fn author_rules() {
        assert!(compute_needs_you(&cr("me", vec![("y", ReviewState::ChangesRequested)], 0), "me"));
        assert!(compute_needs_you(&cr("me", vec![], 1), "me"));
        assert!(!compute_needs_you(&cr("me", vec![("y", ReviewState::Approved)], 0), "me"));
    }

    #[test]
    fn drafts_never() {
        let mut c = cr("x", vec![("me", ReviewState::Pending)], 0);
        c.state = ChangeRequestState::Draft;
        assert!(!compute_needs_you(&c, "me"));
    }
}
