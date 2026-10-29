//! Inbox grouping: which section each PR belongs to, and section collapse state.

use gpui::Context;
use gv_engine::{ChangeRequest, ChangeRequestState};

use crate::app::GitViewer;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Section {
    NeedsYou,
    AwaitingReview,
    Yours,
    Drafts,
    Waiting,
}

impl Section {
    pub const ALL: [Section; 5] = [Section::NeedsYou, Section::AwaitingReview, Section::Yours, Section::Drafts, Section::Waiting];

    pub fn title(self) -> &'static str {
        match self {
            Section::NeedsYou => "Needs you",
            Section::AwaitingReview => "Awaiting review",
            Section::Yours => "Yours",
            Section::Drafts => "Drafts",
            Section::Waiting => "Waiting on others",
        }
    }

    /// Sections collapsed until the user opens them.
    pub fn collapsed_by_default(self) -> bool {
        self == Section::Drafts
    }
}

/// Place a PR in exactly one section. `me` is the signed-in username on its host.
pub fn classify(c: &ChangeRequest, me: Option<&str>) -> Section {
    let is = |u: &str| me.is_some_and(|m| m.eq_ignore_ascii_case(u));
    if c.state == ChangeRequestState::Draft {
        Section::Drafts
    } else if c.needs_you {
        Section::NeedsYou
    } else if is(&c.author.username) {
        Section::Yours
    } else if c.reviewers.iter().any(|r| is(&r.user.username)) {
        Section::AwaitingReview
    } else {
        Section::Waiting
    }
}

impl GitViewer {
    pub(crate) fn section_of(&self, c: &ChangeRequest) -> Section {
        classify(c, self.engine.me(c.id.repo.kind, &c.id.repo.host).as_deref())
    }

    pub(crate) fn toggle_section(&mut self, s: Section, cx: &mut Context<Self>) {
        if !self.collapsed.remove(&s) {
            self.collapsed.insert(s);
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use gv_engine::{ChangeRequestId, CheckSummary, ProviderKind, RepoRef, Review, ReviewState, User};

    fn cr(author: &str, state: ChangeRequestState, reviewer: Option<&str>, needs_you: bool) -> ChangeRequest {
        ChangeRequest {
            id: ChangeRequestId { repo: RepoRef::new(ProviderKind::GitLab, "gitlab.com", "g/p"), number: 1 },
            title: "t".into(),
            author: User::named(author),
            state,
            source_branch: "a".into(),
            target_branch: "b".into(),
            reviewers: reviewer.map(|r| Review { user: User::named(r), state: ReviewState::Pending }).into_iter().collect(),
            checks: CheckSummary::default(),
            labels: vec![],
            comments_count: 0,
            additions: None,
            deletions: None,
            changed_files: None,
            updated_at: Utc::now(),
            created_at: Utc::now(),
            web_url: String::new(),
            needs_you,
        }
    }

    #[test]
    fn classifies() {
        use ChangeRequestState::*;
        let me = Some("me");
        assert_eq!(classify(&cr("me", Draft, None, true), me), Section::Drafts);
        assert_eq!(classify(&cr("x", Open, Some("me"), true), me), Section::NeedsYou);
        assert_eq!(classify(&cr("x", Open, Some("me"), false), me), Section::AwaitingReview);
        assert_eq!(classify(&cr("me", Open, None, false), me), Section::Yours);
        assert_eq!(classify(&cr("x", Open, Some("y"), false), me), Section::Waiting);
    }
}
