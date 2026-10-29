//! Map engine provider, state, review and check data onto `ui` atoms.

use gpui::{div, prelude::*, Div};
use gv_engine::{ChangeRequest, ChangeRequestState, ProviderKind, ReviewState};

use crate::ui::atoms::{Badge, Pill};
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

pub fn checks_summary(c: &ChangeRequest) -> Div {
    let k = c.checks;
    let (text, color) = if k.total == 0 {
        ("No checks".to_string(), theme::TEXT_SUBTLE)
    } else if k.failed > 0 {
        (format!("{}/{} checks failed", k.failed, k.total), theme::DANGER)
    } else if k.pending > 0 {
        (format!("{}/{} checks pending", k.pending, k.total), theme::WARNING)
    } else {
        (format!("{}/{} checks passed", k.passed, k.total), theme::SUCCESS)
    };
    div().text_size(theme::TEXT_XS).text_color(color).child(text)
}
