//! Floating Summary / Code / Activity pill and the body of each tab.

use gpui::{div, prelude::*, AnyElement, Context};

use crate::app::GitViewer;
use crate::state::{DetailState, DetailTab};
use crate::ui::molecules::{Tabs, TabsVariant};
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_detail_tabs(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let threads = d.detail.as_ref().map(|x| x.threads.len()).unwrap_or(0);
        let files = d.code.files.as_ref().map(|f| f.len() as u32).or(d.summary.changed_files);
        Tabs::new("detail-tabs")
            .variant(TabsVariant::Floating)
            .tab("Summary", d.tab == DetailTab::Summary)
            .tab(files.map(|n| format!("Code {n}")).unwrap_or_else(|| "Code".into()), d.tab == DetailTab::Code)
            .tab(format!("Activity {threads}"), d.tab == DetailTab::Activity)
            .on_select(cx.listener(|this, i: &usize, _, cx| {
                let tab = [DetailTab::Summary, DetailTab::Code, DetailTab::Activity][*i];
                this.set_detail_tab(tab, cx)
            }))
    }

    pub(super) fn render_detail_body(&self, d: &DetailState, cx: &mut Context<Self>) -> AnyElement {
        if d.tab == DetailTab::Code {
            return self.render_code(d, cx);
        }
        if d.tab == DetailTab::Activity {
            return match &d.detail {
                Some(det) => self.render_threads(det, d, cx).into_any_element(),
                None => self.placeholder(d).into_any_element(),
            };
        }
        div()
            .flex()
            .flex_col()
            .gap(theme::SPACE_3)
            .child(super::overview::overview_card(&d.summary, self.surfaces()))
            .child(self.render_properties(d, cx))
            .child(self.render_checks_card(d, cx))
            .child(self.render_checkout_card(d, cx))
            .child(self.render_description(d, cx))
            .into_any_element()
    }

    fn placeholder(&self, d: &DetailState) -> impl IntoElement {
        div().text_size(theme::text_sm()).text_color(theme::text_muted()).child(if d.loading { "Loading..." } else { "No detail available" })
    }
}
