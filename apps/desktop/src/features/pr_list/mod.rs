//! Center column: header with freshness, filters, error banner and rows.

mod empty;
mod filters;
mod row;
mod sections;

use gpui::{div, prelude::*, ClickEvent, Context};

use crate::app::GitViewer;
use crate::features::shared::{layout, relative_time};
use crate::ui::atoms::{Button, ButtonVariant};
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
            .gap(theme::SPACE_2)
            .child(div().text_size(theme::TEXT_2XL).font_weight(theme::WEIGHT_SEMIBOLD).child(self.project_name()))
            .child(div().text_size(theme::TEXT_SM).text_color(theme::TEXT_MUTED).child(count.to_string()))
            .child(div().text_size(theme::TEXT_XS).text_color(theme::TEXT_SUBTLE).child(freshness))
            .child(div().flex_1())
            .child(
                Button::new("refresh", "")
                    .icon("\u{21BB}")
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
        let mut rows = div().id("pr-list").track_scroll(&self.list_scroll).flex().flex_col().flex_1().min_h(theme::SPACE_0).overflow_y_scroll().gap(theme::SPACE_3).pb(theme::SPACE_4);
        if visible.is_empty() {
            rows = rows.child(self.render_empty(cx));
        }
        rows = rows.children(self.render_sections(&visible, cx));

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(theme::SPACE_0)
            .h_full()
            .pt(layout::TITLEBAR_INSET)
            .px(theme::SPACE_4)
            .gap(theme::SPACE_3)
            .child(self.render_list_header(visible.len(), cx))
            .child(div().flex().items_center().gap(theme::SPACE_3).child(self.render_involvement_tabs(cx)).child(self.render_search(cx)))
            .children(banner)
            .child(rows)
    }
}
