//! Checks card: a plain-language summary, failing and running checks first, passed ones
//! behind "Show N passed", and "Re-run failed" when the provider allows it.

use chrono::Utc;
use gpui::{div, prelude::*, AnyElement, ClickEvent, Context};
use gv_engine::{checks_sentence, CheckStatus, ChecksInfo};

use crate::app::GitViewer;
use crate::features::shared::checks_text;
use crate::state::DetailState;
use crate::ui::atoms::{Button, ButtonVariant, Icon, IconName, ListItem, Spinner};
use crate::ui::theme;

/// Icon for a check status; running ones spin (still under reduced motion).
pub(super) fn status_icon(id: &str, s: CheckStatus) -> AnyElement {
    let icon = |n: IconName, c| Icon::new(n).size(theme::ICON_MD).color(c).into_any_element();
    match s {
        CheckStatus::Failed => icon(IconName::CircleX, theme::danger()),
        CheckStatus::Running => Spinner::new(format!("spin-{id}")).color(theme::warning()).into_any_element(),
        CheckStatus::Pending => icon(IconName::CircleDashed, theme::warning()),
        CheckStatus::Passed => icon(IconName::CircleCheck, theme::success()),
        CheckStatus::Skipped => icon(IconName::CircleMinus, theme::text_subtle()),
    }
}

/// Overall status for the header icon.
fn overall(c: &ChecksInfo) -> Option<CheckStatus> {
    c.runs.iter().filter(|r| r.status != CheckStatus::Skipped).map(|r| r.status).min_by_key(|s| s.rank())
}

impl GitViewer {
    pub(super) fn render_checks_card(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let info = d.detail.as_ref().map(|x| x.checks.clone()).unwrap_or_default();
        let (sentence, color) = if info.runs.is_empty() && d.summary.checks.total > 0 {
            checks_text(&d.summary)
        } else {
            let c = match overall(&info) {
                Some(CheckStatus::Failed) => theme::danger(),
                Some(CheckStatus::Running | CheckStatus::Pending) => theme::warning(),
                _ => theme::text_muted(),
            };
            (checks_sentence(&info, Utc::now()), c)
        };
        let failed = info.count(CheckStatus::Failed) > 0;
        let rerun = (info.can_rerun && failed).then(|| {
            Button::new("checks-rerun", "Re-run failed").icon(IconName::Refresh).variant(ButtonVariant::Ghost).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.confirm_rerun(cx)))
        });
        let header = div()
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .px(theme::row_pad_x())
            .min_h(theme::control_h())
            .children(overall(&info).map(|s| status_icon("checks-head", s)))
            .child(div().text_size(theme::text_sm()).font_weight(theme::WEIGHT_MEDIUM).child("Checks"))
            .child(div().flex_1().min_w(theme::SPACE_0).truncate().text_size(theme::text_sm()).text_color(color).child(sentence))
            .children(rerun);
        let sorted = info.sorted();
        let (top, rest): (Vec<_>, Vec<_>) = sorted.into_iter().partition(|r| matches!(r.status, CheckStatus::Failed | CheckStatus::Running | CheckStatus::Pending));
        let mut rows: Vec<AnyElement> = top.into_iter().map(|r| self.render_check_row(d, r, cx)).collect();
        if !rest.is_empty() {
            let passed = rest.iter().filter(|r| r.status == CheckStatus::Passed).count();
            let skipped = rest.len() - passed;
            let label = match (d.ui.show_passed, passed, skipped) {
                (true, _, _) => "Hide passed".to_string(),
                (false, 0, s) => format!("Show {s} skipped"),
                (false, p, 0) => format!("Show {p} passed"),
                (false, p, s) => format!("Show {p} passed, {s} skipped"),
            };
            let toggle = div()
                .id("checks-show-passed")
                .flex()
                .items_center()
                .gap(theme::SPACE_2)
                .px(theme::row_pad_x())
                .py(theme::item_pad_y())
                .text_size(theme::text_xs())
                .text_color(theme::text_muted())
                .list_item(false)
                .child(Icon::new(if d.ui.show_passed { IconName::ChevronDown } else { IconName::ChevronRight }).size(theme::ICON_SM).color(theme::text_subtle()))
                .child(label)
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.toggle_show_passed(cx)));
            rows.push(toggle.into_any_element());
            if d.ui.show_passed {
                rows.extend(rest.into_iter().map(|r| self.render_check_row(d, r, cx)));
            }
        }
        div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .p(theme::LIST_INSET)
            .rounded(theme::LIST_RADIUS)
            .border_1()
            .border_color(theme::border_subtle())
            .bg(self.surfaces().card())
            .child(header)
            .children(rows)
    }
}
