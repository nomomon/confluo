//! Map engine provider, state, review and check data onto `ui` atoms.

use gpui::{div, prelude::*, Div};
use gv_engine::{ChangeRequest, ChangeRequestState, ProviderKind, ReviewState};

use crate::ui::atoms::{Badge, Pill};
use crate::ui::theme;

pub fn provider_badge(kind: ProviderKind) -> Badge {
    match kind {
        ProviderKind::GitHub => Badge::new("GH", gpui::hsla(0., 0., 0.85, 1.)),
        ProviderKind::GitLab => Badge::new("GL", gpui::hsla(20. / 360., 0.9, 0.6, 1.)),
        ProviderKind::Azure => Badge::new("AZ", gpui::hsla(205. / 360., 0.85, 0.6, 1.)),
    }
}

pub fn state_pill(s: ChangeRequestState) -> Pill {
    match s {
        ChangeRequestState::Open => Pill::new("Open", theme::green()),
        ChangeRequestState::Draft => Pill::new("Draft", theme::muted()),
        ChangeRequestState::Merged => Pill::new("Merged", theme::purple()),
        ChangeRequestState::Closed => Pill::new("Closed", theme::red()),
    }
}

pub fn review_summary(c: &ChangeRequest) -> Div {
    let n = |s: ReviewState| c.reviewers.iter().filter(|r| r.state == s).count();
    let (a, ch, p) = (n(ReviewState::Approved), n(ReviewState::ChangesRequested), n(ReviewState::Pending));
    let (text, color) = if ch > 0 {
        (format!("{ch} changes requested"), theme::red())
    } else if a > 0 {
        (format!("{a} approved"), theme::green())
    } else if p > 0 {
        (format!("{p} pending review"), theme::yellow())
    } else if c.reviewers.is_empty() {
        ("No reviewers".into(), theme::faint())
    } else {
        ("Commented".into(), theme::muted())
    };
    div().text_xs().text_color(color).child(text)
}

pub fn checks_summary(c: &ChangeRequest) -> Div {
    let k = c.checks;
    let (text, color) = if k.total == 0 {
        ("No checks".to_string(), theme::faint())
    } else if k.failed > 0 {
        (format!("{}/{} checks failed", k.failed, k.total), theme::red())
    } else if k.pending > 0 {
        (format!("{}/{} checks pending", k.pending, k.total), theme::yellow())
    } else {
        (format!("{}/{} checks passed", k.passed, k.total), theme::green())
    };
    div().text_xs().text_color(color).child(text)
}
