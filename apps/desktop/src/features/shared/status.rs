//! Map engine provider, state, review and check data onto `ui` atoms.

use gpui::{div, prelude::*, Div, Hsla};
use gv_engine::{ChangeRequest, ChangeRequestState, ProviderKind, ReviewState};

use crate::ui::atoms::{Badge, Icon, Pill};
use crate::ui::theme;

pub fn provider_badge(kind: ProviderKind) -> Badge {
    match kind {
        ProviderKind::GitHub => Badge::new("GH", theme::TEXT_MUTED),
        ProviderKind::GitLab => Badge::new("GL", theme::TEXT_MUTED),
        ProviderKind::Azure => Badge::new("AZ", theme::TEXT_MUTED),
    }
}

pub fn state_pill(s: ChangeRequestState) -> Pill {
    match s {
        ChangeRequestState::Open => Pill::new("Open", theme::SUCCESS),
        ChangeRequestState::Draft => Pill::new("Draft", theme::TEXT_SUBTLE),
        ChangeRequestState::Merged => Pill::new("Merged", theme::MERGED),
        ChangeRequestState::Closed => Pill::new("Closed", theme::DANGER),
    }
}

/// Leading state glyph: green open, grey draft, purple merged, red closed.
pub fn state_icon(s: ChangeRequestState) -> Icon {
    match s {
        ChangeRequestState::Open => Icon::new("\u{25CF}").color(theme::SUCCESS),
        ChangeRequestState::Draft => Icon::new("\u{25CB}").color(theme::TEXT_SUBTLE),
        ChangeRequestState::Merged => Icon::new("\u{25CF}").color(theme::MERGED),
        ChangeRequestState::Closed => Icon::new("\u{25CF}").color(theme::DANGER),
    }
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
        (format!("{ch} changes requested"), theme::DANGER)
    } else if a > 0 {
        (format!("{a} approved"), theme::SUCCESS)
    } else if p > 0 {
        (format!("{p} pending review"), theme::WARNING)
    } else if c.reviewers.is_empty() {
        ("No reviewers".into(), theme::TEXT_SUBTLE)
    } else {
        ("Commented".into(), theme::TEXT_MUTED)
    };
    div().text_size(theme::TEXT_XS).text_color(color).child(text)
}

/// Checks in words; red only when something fails.
pub fn checks_text(c: &ChangeRequest) -> (String, Hsla) {
    let k = c.checks;
    if k.total == 0 {
        ("No checks reported".into(), theme::TEXT_SUBTLE)
    } else if k.failed > 0 {
        (format!("{} failing", k.failed), theme::DANGER)
    } else if k.pending > 0 {
        (format!("{} pending", k.pending), theme::TEXT_MUTED)
    } else {
        (format!("{} passed", k.passed), theme::TEXT_MUTED)
    }
}

pub fn checks_summary(c: &ChangeRequest) -> Div {
    let (text, color) = checks_text(c);
    div().text_size(theme::TEXT_XS).text_color(color).child(text)
}
