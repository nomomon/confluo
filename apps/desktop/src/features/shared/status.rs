//! Map engine provider, state, review and check data onto `ui` atoms.

use gpui::Hsla;
use gv_engine::{ChangeRequest, ChangeRequestState, ProviderKind, ReviewState};

use crate::ui::atoms::{Icon, IconName};
use crate::ui::theme;

/// Tabler brand icon for a provider: outline, muted, `icon_md()`.
pub fn provider_icon(kind: ProviderKind) -> Icon {
    let name = match kind {
        ProviderKind::GitHub => IconName::BrandGithub,
        ProviderKind::GitLab => IconName::BrandGitlab,
        ProviderKind::Azure => IconName::BrandAzure,
    };
    Icon::new(name)
        .size(theme::icon_md())
        .color(theme::text_muted())
}

/// Small muted provider mark shown on every PR row, the preview and the review bar. Always shown, so the
/// layout never shifts when the visible rows switch between one and several providers.
pub fn provider_mark(kind: ProviderKind) -> Icon {
    provider_icon(kind)
        .size(theme::icon_sm())
        .color(theme::text_subtle())
}

/// Leading pull request icon: green open, grey draft, purple merged, red closed.
/// The state's icon name (open, draft, merged, closed).
pub fn state_icon_name(s: ChangeRequestState) -> IconName {
    match s {
        ChangeRequestState::Open => IconName::GitPullRequest,
        ChangeRequestState::Draft => IconName::GitPullRequestDraft,
        ChangeRequestState::Merged => IconName::GitMerge,
        ChangeRequestState::Closed => IconName::GitPullRequestClosed,
    }
}

pub fn state_icon(s: ChangeRequestState) -> Icon {
    let (name, color) = match s {
        ChangeRequestState::Open => (IconName::GitPullRequest, theme::success()),
        ChangeRequestState::Draft => (IconName::GitPullRequestDraft, theme::text_subtle()),
        ChangeRequestState::Merged => (IconName::GitMerge, theme::merged()),
        ChangeRequestState::Closed => (IconName::GitPullRequestClosed, theme::danger()),
    };
    Icon::new(name).size(theme::icon_lg()).color(color)
}

/// `#12` on GitHub and Azure, `!12` on GitLab.
pub fn number_label(c: &ChangeRequest) -> String {
    let sep = if c.id.repo.kind == ProviderKind::GitLab {
        '!'
    } else {
        '#'
    };
    format!("{sep}{}", c.id.number)
}

/// Checks in words: green when all passed, red when something fails.
pub fn checks_text(c: &ChangeRequest) -> (String, Hsla) {
    let k = c.checks;
    if k.total == 0 {
        (String::new(), theme::text_subtle())
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
    if c.reviewers
        .iter()
        .any(|r| r.state == ReviewState::ChangesRequested)
    {
        Some("Changes requested")
    } else if c.checks.failed == 0 {
        Some("Your review requested")
    } else {
        None
    }
}
