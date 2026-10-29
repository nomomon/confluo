//! Center column: header with freshness, filters, error banner and rows.

mod empty;
mod filters;
mod row;
mod sections;

use gpui::{div, prelude::*, ClickEvent, Context};

use crate::app::GitViewer;
use crate::features::shared::{layout, relative_time};
use crate::ui::atoms::{Button, ButtonVariant, IconName};
use crate::ui::molecules::Banner;
use crate::ui::theme;

impl GitViewer {
    fn render_list_header(&self, count: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let freshness = if self.show_loading {
            "Refreshing...".to_string()
        } else {
            self.refreshed_at.map(|t| format!("Updated {}", relative_time(t))).unwrap_or_else(|| "Not updated yet".into())
        };
        div()
            .flex()
            .items_center()
            .gap(theme::SPACE_3)
            .child(div().text_size(theme::text_2xl()).font_weight(theme::WEIGHT_SEMIBOLD).child(self.project_name()))
            .child(div().text_size(theme::text_lg()).text_color(theme::text_muted()).child(count.to_string()))
            .child(div().pl(theme::SPACE_2).text_size(theme::text_sm()).text_color(theme::text_subtle()).child(freshness))
            .child(div().flex_1())
            .child(
                Button::new("refresh", "")
                    .icon(IconName::Refresh)
                    .variant(ButtonVariant::Ghost)
                    .tooltip("Refresh  \u{2318}R")
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.refresh(cx))),
            )
    }

    pub(crate) fn render_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let visible = self.visible_items(cx);
        let banner = self.error.clone().map(|e| {
            Banner::warning("refresh-error", e)
                .action("Retry", cx.listener(|this, _: &ClickEvent, _, cx| this.retry(cx)))
                .on_dismiss(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.error = None;
                    cx.notify()
                }))
        });
        let mut rows = div().id("pr-list").track_scroll(&self.list_scroll).flex().flex_col().flex_1().min_h(theme::SPACE_0).overflow_y_scroll().pb(theme::SPACE_12);
        if visible.is_empty() {
            rows = rows.child(self.render_empty(cx));
        }
        rows = rows.children(self.render_sections(&visible, cx));

        let content = div()
            .flex()
            .flex_col()
            .w_full()
            .max_w(layout::CONTENT_MAX_W)
            .h_full()
            .pt(layout::TITLEBAR_INSET)
            .px(theme::SPACE_12)
            .child(self.render_list_header(visible.len(), cx))
            .child(div().flex().items_center().gap(theme::SPACE_4).mt(theme::SPACE_6).child(self.render_involvement_tabs(cx)).child(self.render_search(cx)))
            .children(banner.map(|b| div().mt(theme::SPACE_4).child(b)))
            .child(rows);
        // The column breathes: content is centered and capped, like a document.
        div().flex().justify_center().flex_1().min_w(theme::SPACE_0).h_full().child(content)
    }
}
