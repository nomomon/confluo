//! Map engine provider, state, review and check data onto `ui` atoms.

use gpui::{div, prelude::*, Div, Hsla};
use gv_engine::{ChangeRequest, ChangeRequestState, ProviderKind, ReviewState};

use crate::ui::atoms::{Icon, IconName, Pill};
use crate::ui::theme;

/// Tabler brand icon for a provider: outline, muted, `ICON_MD`.
pub fn provider_icon(kind: ProviderKind) -> Icon {
    let name = match kind {
        ProviderKind::GitHub => IconName::BrandGithub,
        ProviderKind::GitLab => IconName::BrandGitlab,
        ProviderKind::Azure => IconName::BrandAzure,
    };
    Icon::new(name).size(theme::ICON_MD).color(theme::text_muted())
}

pub fn state_pill(s: ChangeRequestState) -> Pill {
    match s {
        ChangeRequestState::Open => Pill::new("Open", theme::success()),
        ChangeRequestState::Draft => Pill::new("Draft", theme::text_subtle()),
        ChangeRequestState::Merged => Pill::new("Merged", theme::merged()),
        ChangeRequestState::Closed => Pill::new("Closed", theme::danger()),
    }
}

/// Leading pull request icon: green open, grey draft, purple merged, red closed.
pub fn state_icon(s: ChangeRequestState) -> Icon {
    let (name, color) = match s {
        ChangeRequestState::Open => (IconName::GitPullRequest, theme::success()),
        ChangeRequestState::Draft => (IconName::GitPullRequestDraft, theme::text_subtle()),
        ChangeRequestState::Merged => (IconName::GitMerge, theme::merged()),
        ChangeRequestState::Closed => (IconName::GitPullRequestClosed, theme::danger()),
    };
    Icon::new(name).size(theme::ICON_LG).color(color)
}

/// `#12` on GitHub and Azure, `!12` on GitLab.
pub fn number_label(c: &ChangeRequest) -> String {
    let sep = if c.id.repo.kind == ProviderKind::GitLab { '!' } else { '#' };
    format!("{sep}{}", c.id.number)
}

pub fn review_summary(c: &ChangeRequest) -> Div {
    let n = |s: ReviewState| c.reviewers.iter().filter(|r| r.state == s).count();
    let (a, ch, p) = (n(ReviewState::Approved), n(ReviewState::ChangesRequested), n(ReviewState::Pending));
    let (text, color) = if ch > 0 {
        (format!("{ch} changes requested"), theme::danger())
    } else if a > 0 {
        (format!("{a} approved"), theme::success())
    } else if p > 0 {
        (format!("{p} pending review"), theme::warning())
    } else if c.reviewers.is_empty() {
        ("No reviewers".into(), theme::text_subtle())
    } else {
        ("Commented".into(), theme::text_muted())
    };
    div().text_size(theme::text_xs()).text_color(color).child(text)
}

/// Checks in words: green when all passed, red when something fails.
pub fn checks_text(c: &ChangeRequest) -> (String, Hsla) {
    let k = c.checks;
    if k.total == 0 {
        ("No checks reported".into(), theme::text_subtle())
    } else if k.failed > 0 {
        (format!("{} failing", k.failed), theme::danger())
    } else if k.pending > 0 {
        (format!("{} pending", k.pending), theme::text_muted())
    } else {
        (format!("{} passed", k.passed), theme::success())
    }
}

/// Muted hint after the checks in a list row: draft, review asked, changes asked.
pub fn row_hint(c: &ChangeRequest) -> Option<&'static str> {
    if c.state == ChangeRequestState::Draft {
        return Some("Draft");
    }
    if !c.needs_you {
        return None;
    }
    if c.reviewers.iter().any(|r| r.state == ReviewState::ChangesRequested) {
        Some("Changes requested")
    } else if c.checks.failed == 0 {
        Some("Your review requested")
    } else {
        None
    }
}

pub fn checks_summary(c: &ChangeRequest) -> Div {
    let (text, color) = checks_text(c);
    div().text_size(theme::text_xs()).text_color(color).child(text)
}
