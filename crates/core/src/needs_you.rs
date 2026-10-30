use crate::{ChangeRequest, ChangeRequestState, ReviewState};

/// Why a PR needs me, most urgent first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeedsReason {
    /// I reviewed, then this many commits were pushed.
    ReReview(u32),
    /// My review is requested and I have not reviewed yet.
    ReviewRequested,
    /// Unresolved threads waiting on me.
    Threads(u32),
    /// My PR: a reviewer requested changes.
    ChangesRequested,
    /// My PR: checks fail.
    ChecksFailing,
    /// My PR: merge conflicts.
    Conflicts,
}

impl NeedsReason {
    /// Short hint for list rows.
    pub fn label(self) -> String {
        match self {
            Self::ReReview(1) => "Re-review: 1 new commit".into(),
            Self::ReReview(n) => format!("Re-review: {n} new commits"),
            Self::ReviewRequested => "Review requested".into(),
            Self::Threads(1) => "1 thread waiting on you".into(),
            Self::Threads(n) => format!("{n} threads waiting on you"),
            Self::ChangesRequested => "Changes requested".into(),
            Self::ChecksFailing => "CI failing".into(),
            Self::Conflicts => "Merge conflicts".into(),
        }
    }
}

/// Why `me` should act on this PR, if at all.
///
/// - Reviewer: requested and not reviewed, or reviewed and new commits came after.
/// - Anyone involved: unresolved threads waiting on me.
/// - Author: changes requested, failing checks or conflicts (not on drafts, which are work in progress).
///
/// Merged and closed PRs never need me.
pub fn needs_reason(cr: &ChangeRequest, me: &str) -> Option<NeedsReason> {
    if !cr.state.is_active() || me.is_empty() {
        return None;
    }
    let is_me = |u: &str| u.eq_ignore_ascii_case(me);
    let sig = cr.signals;
    if is_me(&cr.author.username) {
        if sig.threads_need_me > 0 {
            return Some(NeedsReason::Threads(sig.threads_need_me));
        }
        if cr.state == ChangeRequestState::Draft {
            return None;
        }
        let changes = cr
            .reviewers
            .iter()
            .any(|r| r.state == ReviewState::ChangesRequested);
        return if changes {
            Some(NeedsReason::ChangesRequested)
        } else if cr.checks.failed > 0 {
            Some(NeedsReason::ChecksFailing)
        } else if sig.conflicts {
            Some(NeedsReason::Conflicts)
        } else {
            None
        };
    }
    let mine = cr.reviewers.iter().find(|r| is_me(&r.user.username));
    match (mine.map(|r| r.state), sig.since_my_review) {
        // A pending request outranks an older review: the provider says I owe one.
        (Some(ReviewState::Pending), Some(n)) if n > 0 => return Some(NeedsReason::ReReview(n)),
        (Some(ReviewState::Pending), _) => return Some(NeedsReason::ReviewRequested),
        (Some(_), Some(n)) if n > 0 => return Some(NeedsReason::ReReview(n)),
        _ => {}
    }
    (sig.threads_need_me > 0).then_some(NeedsReason::Threads(sig.threads_need_me))
}

/// Whether `me` should act on this PR. See [`needs_reason`].
pub fn compute_needs_you(cr: &ChangeRequest, me: &str) -> bool {
    needs_reason(cr, me).is_some()
}

#[cfg(test)]
#[path = "needs_you_tests.rs"]
mod tests;
