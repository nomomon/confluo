//! Collapsible checks card.

use gpui::{div, prelude::*, ClickEvent, Context};

use crate::app::GitViewer;
use crate::features::shared::checks_summary;
use crate::state::DetailState;
use crate::ui::atoms::{Icon, IconName};
use crate::ui::molecules::{Card, KeyValueRow};
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_checks_card(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let ch = d.summary.checks;
        let toggle = div()
            .id("checks-toggle")
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .cursor_pointer()
            .child(div().text_size(theme::text_sm()).font_weight(theme::WEIGHT_MEDIUM).child("Checks"))
            .child(checks_summary(&d.summary))
            .child(div().flex_1())
            .child(Icon::new(if d.checks_open { IconName::ChevronDown } else { IconName::ChevronRight }).size(theme::ICON_SM).color(theme::TEXT_SUBTLE))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                if let Some(d) = this.detail.as_mut() {
                    d.checks_open = !d.checks_open;
                }
                cx.notify()
            }));
        Card::new(self.surfaces()).child(toggle).when(d.checks_open, |card| {
            card.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(theme::SPACE_1)
                    .text_size(theme::text_xs())
                    .child(KeyValueRow::new("Passed", div().text_color(theme::SUCCESS).child(ch.passed.to_string())))
                    .child(KeyValueRow::new("Failed", div().text_color(theme::DANGER).child(ch.failed.to_string())))
                    .child(KeyValueRow::new("Pending", div().text_color(theme::WARNING).child(ch.pending.to_string())))
                    .child(KeyValueRow::new("Total", div().child(ch.total.to_string()))),
            )
        })
    }
}
