//! Floating Summary / Activity pill and the body of each tab.

use gpui::{div, prelude::*, AnyElement, Context};

use crate::app::GitViewer;
use crate::state::{DetailState, DetailTab};
use crate::ui::molecules::markdown::markdown;
use crate::ui::molecules::{Card, SegmentedTabs};
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_detail_tabs(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let threads = d.detail.as_ref().map(|x| x.threads.len()).unwrap_or(0);
        SegmentedTabs::new("detail-tabs")
            .floating()
            .tab("Summary", d.tab == DetailTab::Summary)
            .tab(format!("Activity {threads}"), d.tab == DetailTab::Activity)
            .on_select(cx.listener(|this, i: &usize, _, cx| {
                if let Some(d) = this.detail.as_mut() {
                    d.tab = if *i == 0 { DetailTab::Summary } else { DetailTab::Activity };
                }
                cx.notify()
            }))
    }

    pub(super) fn render_detail_body(&self, d: &DetailState, cx: &mut Context<Self>) -> AnyElement {
        if d.tab == DetailTab::Activity {
            return match &d.detail {
                Some(det) => self.render_threads(det, d, cx).into_any_element(),
                None => self.placeholder(d).into_any_element(),
            };
        }
        let description = match &d.detail {
            None => self.placeholder(d).into_any_element(),
            Some(det) if det.description.trim().is_empty() => {
                div().text_size(theme::TEXT_SM).text_color(theme::TEXT_SUBTLE).child("No description").into_any_element()
            }
            Some(det) => div().flex().flex_col().gap(theme::SPACE_2).children(markdown("desc", &det.description)).into_any_element(),
        };
        div()
            .flex()
            .flex_col()
            .gap(theme::SPACE_3)
            .child(super::overview::overview_card(&d.summary, self.surfaces()))
            .child(self.render_checks_card(d, cx))
            .child(self.render_checkout_card(d, cx))
            .child(Card::new(self.surfaces()).child(description))
            .into_any_element()
    }

    fn placeholder(&self, d: &DetailState) -> impl IntoElement {
        div().text_size(theme::TEXT_SM).text_color(theme::TEXT_MUTED).child(if d.loading { "Loading..." } else { "No detail available" })
    }
}
