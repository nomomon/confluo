use super::*;
use crate::*;
use chrono::Utc;

fn cr(author: &str, reviewers: Vec<(&str, ReviewState)>, failed: u32) -> ChangeRequest {
    ChangeRequest {
        id: ChangeRequestId {
            repo: RepoRef::new(ProviderKind::GitHub, "github.com", "a/b"),
            number: 1,
        },
        title: "t".into(),
        author: User::named(author),
        state: ChangeRequestState::Open,
        source_branch: "f".into(),
        target_branch: "main".into(),
        reviewers: reviewers
            .into_iter()
            .map(|(u, s)| Review {
                user: User::named(u),
                state: s,
            })
            .collect(),
        checks: CheckSummary {
            total: failed,
            passed: 0,
            failed,
            pending: 0,
        },
        labels: vec![],
        comments_count: 0,
        additions: None,
        deletions: None,
        changed_files: None,
        updated_at: Utc::now(),
        created_at: Utc::now(),
        web_url: String::new(),
        needs_you: false,
        signals: Default::default(),
    }
}

fn since(mut c: ChangeRequest, n: u32) -> ChangeRequest {
    c.signals.since_my_review = Some(n);
    c
}

use ReviewState::*;

#[test]
fn reviewer_requested() {
    let c = cr("x", vec![("me", Pending)], 0);
    assert_eq!(needs_reason(&c, "me"), Some(NeedsReason::ReviewRequested));
    assert_eq!(classify(&c, Some("me")), Section::NeedsYou);
    assert_eq!(needs_reason(&cr("x", vec![], 0), "me"), None);
}

#[test]
fn approved_without_new_commits_is_reviewed() {
    for st in [Approved, Commented, ChangesRequested] {
        let c = since(cr("x", vec![("Me", st)], 0), 0);
        assert_eq!(needs_reason(&c, "me"), None);
        assert_eq!(classify(&c, Some("me")), Section::Reviewed);
        // Unknown push state still counts as reviewed.
        assert_eq!(
            classify(&cr("x", vec![("me", st)], 0), Some("me")),
            Section::Reviewed
        );
    }
}

#[test]
fn rereview_after_push() {
    let before = since(cr("x", vec![("me", Approved)], 0), 0);
    assert_eq!(classify(&before, Some("me")), Section::Reviewed);
    let after = since(before, 3);
    assert_eq!(needs_reason(&after, "me"), Some(NeedsReason::ReReview(3)));
    assert_eq!(classify(&after, Some("me")), Section::NeedsYou);
    assert_eq!(NeedsReason::ReReview(3).label(), "Re-review: 3 new commits");
    // A re-request after the push also reads as a re-review.
    let rerequested = since(cr("x", vec![("me", Pending)], 0), 2);
    assert_eq!(
        needs_reason(&rerequested, "me"),
        Some(NeedsReason::ReReview(2))
    );
}

#[test]
fn threads_waiting_on_me() {
    let mut c = cr("x", vec![("me", Approved)], 0);
    c.signals.threads_need_me = 2;
    assert_eq!(needs_reason(&c, "me"), Some(NeedsReason::Threads(2)));
    let mut w = cr("x", vec![], 0);
    w.signals.threads_need_me = 1;
    assert_eq!(classify(&w, Some("me")), Section::NeedsYou);
}

#[test]
fn author_rules() {
    let changes = cr("me", vec![("y", ChangesRequested)], 0);
    assert_eq!(
        needs_reason(&changes, "me"),
        Some(NeedsReason::ChangesRequested)
    );
    assert_eq!(
        needs_reason(&cr("me", vec![], 1), "me"),
        Some(NeedsReason::ChecksFailing)
    );
    let mut conflict = cr("me", vec![], 0);
    conflict.signals.conflicts = true;
    assert_eq!(needs_reason(&conflict, "me"), Some(NeedsReason::Conflicts));
    let fine = cr("me", vec![("y", Approved)], 0);
    assert_eq!(needs_reason(&fine, "me"), None);
    assert_eq!(classify(&fine, Some("me")), Section::Yours);
    // Others' conflicts are not mine to fix.
    let mut theirs = cr("x", vec![("y", Approved)], 1);
    theirs.signals.conflicts = true;
    assert_eq!(classify(&theirs, Some("me")), Section::Waiting);
}

#[test]
fn drafts() {
    let mut mine = cr("me", vec![], 1);
    mine.state = ChangeRequestState::Draft;
    assert_eq!(classify(&mine, Some("me")), Section::Drafts);
    let mut asked = cr("x", vec![("me", Pending)], 0);
    asked.state = ChangeRequestState::Draft;
    assert_eq!(classify(&asked, Some("me")), Section::NeedsYou);
    let mut other = cr("x", vec![("y", Pending)], 0);
    other.state = ChangeRequestState::Draft;
    assert_eq!(classify(&other, Some("me")), Section::Drafts);
}

#[test]
fn closed_never_and_unknown_me() {
    let mut c = cr("x", vec![("me", Pending)], 0);
    c.state = ChangeRequestState::Merged;
    assert_eq!(needs_reason(&c, "me"), None);
    assert_eq!(
        classify(&cr("x", vec![("me", Pending)], 0), None),
        Section::Waiting
    );
}

#[test]
fn old_section_name_reads_as_reviewed() {
    let s: Section = serde_json::from_str("\"AwaitingReview\"").unwrap();
    assert_eq!(s, Section::Reviewed);
    assert_eq!(
        Section::ALL.map(Section::title),
        [
            "Needs you",
            "Yours",
            "Waiting on others",
            "Reviewed",
            "Drafts"
        ]
    );
}
