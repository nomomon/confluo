//! Center column: PR list header, filters, status banner and rows.

mod filters;
mod picker;
mod row;

use gpui::{div, prelude::*, ClickEvent, Context};

use crate::app::GitViewer;
use crate::ui::atoms::Button;
use crate::ui::molecules::{EmptyState, Notice};
use crate::ui::theme;
use crate::features::shared::layout;

impl GitViewer {
    pub(crate) fn render_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let visible = self.visible_items(cx);

        let header = div()
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .child(div().text_size(theme::TEXT_2XL).font_weight(theme::WEIGHT_SEMIBOLD).child("Pull requests"))
            .child(div().text_size(theme::TEXT_SM).text_color(theme::TEXT_MUTED).child(visible.len().to_string()))
            .child(div().flex_1())
            .child(Button::new("picker", format!("{} \u{25BE}", self.project_name())).on_click(cx.listener(
                |this, _: &ClickEvent, _, cx| {
                    this.picker_open = !this.picker_open;
                    cx.notify()
                },
            )))
            .child(
                Button::new("refresh", if self.refreshing { "Refreshing..." } else { "Refresh" })
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.refresh(cx))),
            );

        let mut rows = div().id("pr-list").flex().flex_col().flex_1().min_h(theme::SPACE_0).overflow_y_scroll().gap(theme::SPACE_1).pb(theme::SPACE_4);
        if visible.is_empty() {
            let hint = if self.refreshing { "Fetching from providers..." } else { "Try another tab or refresh" };
            rows = rows.child(EmptyState::new("No open pull requests", hint));
        }
        let selected_id = self.detail.as_ref().map(|d| d.summary.id.clone());
        for c in visible {
            rows = rows.child(self.render_row(c, selected_id.as_ref() == Some(&c.id), cx));
        }

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(theme::SPACE_0)
            .h_full()
            .pt(layout::TITLEBAR_INSET)
            .px(theme::SPACE_4)
            .gap(theme::SPACE_3)
            .child(header)
            .child(div().flex().items_center().gap(theme::SPACE_3).child(self.render_involvement_tabs(cx)).child(div().flex_1().child(self.search.clone())))
            .child(self.render_filter_chips(cx))
            .when_some(self.status.clone(), |col, s| col.child(Notice::warning(s)))
            .child(rows)
    }
}
