//! Review actions (approve, request changes, ...) and which of them a provider offers on a PR.

use crate::{ChangeRequest, ReviewState, User};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Something the viewer can submit as a review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReviewAction {
    Approve,
    RequestChanges,
    /// A review summary without a verdict.
    Comment,
    /// Revoke an earlier approval (GitLab).
    Unapprove,
    /// Azure DevOps vote 5.
    ApproveWithSuggestions,
    /// Azure DevOps vote -5.
    WaitForAuthor,
    /// Azure DevOps vote 0: take back any vote.
    ResetVote,
}

impl ReviewAction {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().replace('_', "-").as_str() {
            "approve" => Some(Self::Approve),
            "request-changes" | "reject" => Some(Self::RequestChanges),
            "comment" => Some(Self::Comment),
            "unapprove" | "revoke" => Some(Self::Unapprove),
            "approve-with-suggestions" | "suggest" => Some(Self::ApproveWithSuggestions),
            "wait-for-author" | "wait" => Some(Self::WaitForAuthor),
            "reset" | "reset-vote" => Some(Self::ResetVote),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Approve => "Approve",
            Self::RequestChanges => "Request changes",
            Self::Comment => "Comment",
            Self::Unapprove => "Revoke approval",
            Self::ApproveWithSuggestions => "Approve with suggestions",
            Self::WaitForAuthor => "Wait for author",
            Self::ResetVote => "Reset vote",
        }
    }
}

/// Review actions the provider supports on one PR for the signed-in user.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewCapabilities {
    pub approve: bool,
    pub unapprove: bool,
    pub request_changes: bool,
    pub comment_review: bool,
    pub merge: bool,
    /// Why approving is not possible although the provider has it (e.g. own PR).
    pub approve_blocked: Option<String>,
    /// Azure DevOps votes besides approve and reject.
    #[serde(default)]
    pub approve_with_suggestions: bool,
    #[serde(default)]
    pub wait_for_author: bool,
    #[serde(default)]
    pub reset_vote: bool,
    /// Provider wording for "Request changes" (Azure DevOps: "Reject").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_changes_label: Option<String>,
}

impl ReviewCapabilities {
    pub fn allows(&self, a: ReviewAction) -> bool {
        match a {
            ReviewAction::Approve => self.approve,
            ReviewAction::RequestChanges => self.request_changes,
            ReviewAction::Comment => self.comment_review,
            ReviewAction::Unapprove => self.unapprove,
            ReviewAction::ApproveWithSuggestions => self.approve_with_suggestions,
            ReviewAction::WaitForAuthor => self.wait_for_author,
            ReviewAction::ResetVote => self.reset_vote,
        }
    }

    pub fn any(&self) -> bool {
        self.approve
            || self.unapprove
            || self.request_changes
            || self.comment_review
            || self.approve_blocked.is_some()
            || self.approve_with_suggestions
            || self.wait_for_author
            || self.reset_vote
    }

    /// Label of the "Request changes" action on this provider.
    pub fn request_changes_label(&self) -> &str {
        self.request_changes_label
            .as_deref()
            .unwrap_or("Request changes")
    }
}

/// Approvals given and required (GitLab approval rules).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Approvals {
    pub given: u32,
    pub required: u32,
}

/// Review side of a PR detail: what I can do and what I already did.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewInfo {
    pub caps: ReviewCapabilities,
    /// My latest verdict (Approved / ChangesRequested / Commented), if any.
    pub my_state: Option<ReviewState>,
    pub approvals: Option<Approvals>,
}

/// A non-comment item of the PR timeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimelineKind {
    Opened,
    Reviewed(ReviewState),
    Merged,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub kind: TimelineKind,
    pub actor: User,
    pub at: DateTime<Utc>,
}

fn is_me(u: &User, me: &str) -> bool {
    u.username.eq_ignore_ascii_case(me)
}

/// My verdict among the latest reviews of a PR.
pub fn my_review_state(cr: &ChangeRequest, me: &str) -> Option<ReviewState> {
    cr.reviewers
        .iter()
        .find(|r| is_me(&r.user, me))
        .map(|r| r.state)
        .filter(|s| *s != ReviewState::Pending)
}

/// GitHub: approve, request changes and comment on open PRs; authors may only comment.
pub fn github_capabilities(cr: &ChangeRequest, me: &str) -> ReviewCapabilities {
    if !cr.state.is_active() {
        return ReviewCapabilities::default();
    }
    let own = is_me(&cr.author, me);
    ReviewCapabilities {
        approve: !own,
        request_changes: !own,
        comment_review: true,
        approve_blocked: own.then(|| "You can't approve your own PR".into()),
        ..Default::default()
    }
}

/// GitLab: approve or revoke depending on whether I approved; no "request changes".
/// `can_approve` is `user_can_approve` from the approvals endpoint.
pub fn gitlab_capabilities(
    cr: &ChangeRequest,
    me: &str,
    approved_by_me: bool,
    can_approve: bool,
) -> ReviewCapabilities {
    if !cr.state.is_active() {
        return ReviewCapabilities::default();
    }
    let blocked = (!approved_by_me && !can_approve).then(|| {
        if is_me(&cr.author, me) {
            "You can't approve your own MR".to_string()
        } else {
            "You can't approve this MR".to_string()
        }
    });
    ReviewCapabilities {
        approve: !approved_by_me && can_approve,
        unapprove: approved_by_me,
        comment_review: true,
        approve_blocked: blocked,
        ..Default::default()
    }
}

/// The state shown after `action` succeeds, for optimistic updates.
pub fn state_after(action: ReviewAction, before: Option<ReviewState>) -> Option<ReviewState> {
    match action {
        ReviewAction::Approve => Some(ReviewState::Approved),
        ReviewAction::RequestChanges => Some(ReviewState::ChangesRequested),
        ReviewAction::Comment => before.or(Some(ReviewState::Commented)),
        ReviewAction::Unapprove | ReviewAction::ResetVote => None,
        ReviewAction::ApproveWithSuggestions => Some(ReviewState::Approved),
        ReviewAction::WaitForAuthor => Some(ReviewState::ChangesRequested),
    }
}

/// Azure DevOps: every vote on active PRs, including on your own (Azure allows it unless a policy forbids).
/// `my_vote` is my current vote (0 when none).
pub fn azure_capabilities(cr: &ChangeRequest, my_vote: i64) -> ReviewCapabilities {
    if !cr.state.is_active() {
        return ReviewCapabilities::default();
    }
    ReviewCapabilities {
        approve: my_vote != 10,
        approve_with_suggestions: my_vote != 5,
        wait_for_author: my_vote != -5,
        request_changes: my_vote != -10,
        reset_vote: my_vote != 0,
        request_changes_label: Some("Reject".into()),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    fn cr(author: &str, state: ChangeRequestState) -> ChangeRequest {
        ChangeRequest {
            id: ChangeRequestId {
                repo: RepoRef::new(ProviderKind::GitHub, "github.com", "a/b"),
                number: 1,
            },
            title: "t".into(),
            author: User::named(author),
            state,
            source_branch: "f".into(),
            target_branch: "main".into(),
            reviewers: vec![Review {
                user: User::named("Me"),
                state: ReviewState::Approved,
            }],
            checks: CheckSummary::default(),
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
    fn github_own_pr_cannot_approve() {
        let c = github_capabilities(&cr("me", ChangeRequestState::Open), "me");
        assert!(!c.approve && !c.request_changes && c.comment_review);
        assert_eq!(
            c.approve_blocked.as_deref(),
            Some("You can't approve your own PR")
        );
        let c = github_capabilities(&cr("ada", ChangeRequestState::Open), "me");
        assert!(c.approve && c.request_changes && !c.unapprove && c.approve_blocked.is_none());
    }

    #[test]
    fn closed_prs_offer_nothing() {
        assert!(!github_capabilities(&cr("ada", ChangeRequestState::Merged), "me").any());
        assert!(
            !gitlab_capabilities(&cr("ada", ChangeRequestState::Closed), "me", false, true).any()
        );
    }

    #[test]
    fn gitlab_toggles_approve_and_revoke() {
        let c = gitlab_capabilities(&cr("ada", ChangeRequestState::Open), "me", false, true);
        assert!(c.approve && !c.unapprove && !c.request_changes);
        let c = gitlab_capabilities(&cr("ada", ChangeRequestState::Open), "me", true, false);
        assert!(!c.approve && c.unapprove && c.approve_blocked.is_none());
        let c = gitlab_capabilities(&cr("me", ChangeRequestState::Open), "me", false, false);
        assert_eq!(
            c.approve_blocked.as_deref(),
            Some("You can't approve your own MR")
        );
    }

    #[test]
    fn my_state_and_optimistic() {
        assert_eq!(
            my_review_state(&cr("ada", ChangeRequestState::Open), "me"),
            Some(ReviewState::Approved)
        );
        assert_eq!(
            state_after(ReviewAction::Unapprove, Some(ReviewState::Approved)),
            None
        );
        assert_eq!(
            state_after(ReviewAction::Comment, None),
            Some(ReviewState::Commented)
        );
        assert_eq!(
            ReviewAction::parse("request-changes"),
            Some(ReviewAction::RequestChanges)
        );
        assert_eq!(
            ReviewAction::parse("wait"),
            Some(ReviewAction::WaitForAuthor)
        );
        assert_eq!(
            state_after(ReviewAction::ResetVote, Some(ReviewState::Approved)),
            None
        );
    }

    #[test]
    fn azure_votes() {
        let c = azure_capabilities(&cr("me", ChangeRequestState::Open), 10);
        assert!(
            !c.approve
                && c.approve_with_suggestions
                && c.wait_for_author
                && c.request_changes
                && c.reset_vote
        );
        assert_eq!(c.request_changes_label(), "Reject");
        assert!(!azure_capabilities(&cr("ada", ChangeRequestState::Open), 0).reset_vote);
        assert!(!azure_capabilities(&cr("ada", ChangeRequestState::Merged), 0).any());
    }
}
