//! Summary / Activity tab switcher and the summary tab body.

use gpui::{div, prelude::*, AnyElement, Context};

use crate::app::GitViewer;
use crate::state::{DetailState, DetailTab};
use crate::ui::molecules::markdown::markdown;
use crate::ui::molecules::SegmentedTabs;
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_detail_tabs(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let threads = d.detail.as_ref().map(|x| x.threads.len()).unwrap_or(0);
        SegmentedTabs::new("detail-tabs")
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
        match (&d.detail, d.tab) {
            (None, _) => div()
                .text_size(theme::TEXT_SM)
                .text_color(theme::TEXT_MUTED)
                .child(if d.loading { "Loading..." } else { "No detail available" })
                .into_any_element(),
            (Some(det), DetailTab::Summary) if det.description.trim().is_empty() => {
                div().text_size(theme::TEXT_SM).text_color(theme::TEXT_SUBTLE).child("No description").into_any_element()
            }
            (Some(det), DetailTab::Summary) => div().flex().flex_col().gap(theme::SPACE_1).children(markdown(&det.description)).into_any_element(),
            (Some(det), DetailTab::Activity) => self.render_threads(det, d, cx).into_any_element(),
        }
    }
}
