//! Which list section a PR belongs to (Inbox, Starred and projects group the same way).

use serde::{Deserialize, Serialize};

use crate::{needs_reason, ChangeRequest, ChangeRequestState, ReviewState};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Section {
    NeedsYou,
    Yours,
    Waiting,
    /// I reviewed and nothing new came since. Older settings call it `AwaitingReview`.
    #[serde(alias = "AwaitingReview")]
    Reviewed,
    Drafts,
}

impl Section {
    /// Display order.
    pub const ALL: [Section; 5] = [
        Section::NeedsYou,
        Section::Yours,
        Section::Waiting,
        Section::Reviewed,
        Section::Drafts,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Section::NeedsYou => "Needs you",
            Section::Yours => "Yours",
            Section::Waiting => "Waiting on others",
            Section::Reviewed => "Reviewed",
            Section::Drafts => "Drafts",
        }
    }

    /// Sections collapsed until the user opens them.
    pub fn collapsed_by_default(self) -> bool {
        matches!(self, Section::Drafts | Section::Reviewed)
    }
}

/// Place a PR in exactly one section; first match wins. `me` is the signed-in username on its host.
pub fn classify(c: &ChangeRequest, me: Option<&str>) -> Section {
    let me = me.unwrap_or_default();
    let is = |u: &str| !me.is_empty() && me.eq_ignore_ascii_case(u);
    if needs_reason(c, me).is_some() {
        Section::NeedsYou
    } else if c.state == ChangeRequestState::Draft {
        Section::Drafts
    } else if is(&c.author.username) {
        Section::Yours
    } else if c
        .reviewers
        .iter()
        .any(|r| is(&r.user.username) && r.state != ReviewState::Pending)
    {
        Section::Reviewed
    } else {
        Section::Waiting
    }
}
