//! CI as one status icon: passed green check, failed red x, running amber spinner, nothing when
//! there are no checks. Hovering shows the jobs (status, name, duration) when they are known.

use chrono::Utc;
use gpui::{div, prelude::*, AnyElement, AnyView, App, Context, SharedString, Window};
use gv_engine::{format_duration, CheckStatus, CheckSummary, ChecksInfo};

use crate::ui::atoms::{Icon, IconName, Spinner};
use crate::ui::theme;

/// Icon for a check status; running ones spin (still under reduced motion).
pub fn status_icon(id: &str, s: CheckStatus) -> AnyElement {
    let icon = |n: IconName, c| Icon::new(n).size(theme::ICON_MD).color(c).into_any_element();
    match s {
        CheckStatus::Failed => icon(IconName::CircleX, theme::danger()),
        CheckStatus::Running => Spinner::new(format!("spin-{id}")).color(theme::warning()).into_any_element(),
        CheckStatus::Pending => icon(IconName::CircleDashed, theme::warning()),
        CheckStatus::Passed => icon(IconName::CircleCheck, theme::success()),
        CheckStatus::Skipped => icon(IconName::CircleMinus, theme::text_subtle()),
    }
}

/// Worst status among the runs, else from the list summary; None when nothing reported.
pub fn ci_status(sum: &CheckSummary, info: Option<&ChecksInfo>) -> Option<CheckStatus> {
    if let Some(s) = info.and_then(|c| c.runs.iter().filter(|r| r.status != CheckStatus::Skipped).map(|r| r.status).min_by_key(|s| s.rank())) {
        return Some(s);
    }
    if sum.total == 0 {
        None
    } else if sum.failed > 0 {
        Some(CheckStatus::Failed)
    } else if sum.pending > 0 {
        Some(CheckStatus::Running)
    } else {
        Some(CheckStatus::Passed)
    }
}

/// "2 failing, 5 passed" for the tooltip of a list row.
pub fn ci_words(sum: &CheckSummary) -> String {
    let mut parts = vec![];
    if sum.failed > 0 {
        parts.push(format!("{} failing", sum.failed));
    }
    if sum.pending > 0 {
        parts.push(format!("{} running", sum.pending));
    }
    if sum.passed > 0 {
        parts.push(format!("{} passed", sum.passed));
    }
    parts.join(", ")
}

/// Hover card listing the jobs.
pub struct CiCard {
    rows: Vec<(CheckStatus, String, Option<String>)>,
    summary: String,
}

impl CiCard {
    pub fn tooltip(sum: CheckSummary, info: Option<ChecksInfo>) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
        move |_, cx| {
            let now = Utc::now();
            let rows = info.as_ref().map(|c| c.sorted().into_iter().map(|r| (r.status, r.name.clone(), r.duration_secs(now).map(format_duration))).collect()).unwrap_or_default();
            let summary = ci_words(&sum);
            cx.new(|_| CiCard { rows, summary }).into()
        }
    }
}

impl Render for CiCard {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let shown = self.rows.len().min(12);
        let more = self.rows.len() - shown;
        let card = div()
            .flex()
            .flex_col()
            .gap(theme::SPACE_1)
            .p(theme::SPACE_2)
            .max_w(theme::SPACE_16 * 5.)
            .rounded(theme::RADIUS_MD)
            .border_1()
            .border_color(theme::border())
            .bg(theme::overlay())
            .shadow_md()
            .font_family(theme::ui_font())
            .text_size(theme::text_xs())
            .text_color(theme::text());
        if self.rows.is_empty() {
            return card.child(self.summary.clone());
        }
        card.children(self.rows.iter().take(shown).enumerate().map(|(i, (s, name, dur))| {
            div()
                .flex()
                .items_center()
                .gap(theme::SPACE_2)
                .child(status_icon(&format!("ci-card-{i}"), *s))
                .child(div().flex_1().min_w(theme::SPACE_0).truncate().child(name.clone()))
                .children(dur.clone().map(|d| div().flex_shrink_0().text_color(theme::text_subtle()).child(d)))
        }))
        .when(more > 0, |c| c.child(div().text_color(theme::text_subtle()).child(SharedString::from(format!("{more} more")))))
    }
}
