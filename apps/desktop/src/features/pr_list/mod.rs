//! Center column: header with freshness, filters, error banner and rows.

mod empty;
mod filters;
mod footer;
mod row;
mod row_menu;
mod sections;

use gpui::{div, prelude::*, ClickEvent, Context};

use crate::app::GitViewer;
use crate::features::keymap::{NextPr, PrevPr, Refresh};
use crate::features::shared::{layout, relative_time};
use crate::ui::atoms::{Button, ButtonVariant, HintAnchor, IconName, ShortcutHint};
use crate::ui::molecules::Banner;
use crate::ui::theme;

impl GitViewer {
    fn render_list_header(&self, count: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let paused = self.paused_until.filter(|t| *t > chrono::Utc::now());
        let freshness = if let Some(t) = paused {
            // Quiet: cached data stays, no banner.
            format!(
                "Paused until {} (rate limit)",
                t.with_timezone(&chrono::Local).format("%H:%M")
            )
        } else if self.show_loading {
            "Refreshing...".to_string()
        } else {
            self.refreshed_at
                .map(|t| format!("Updated {}", relative_time(t)))
                .unwrap_or_else(|| "Not updated yet".into())
        };
        div()
            .flex()
            .items_center()
            .gap(theme::space_2p5())
            .child(ShortcutHint::wrap(
                div()
                    .text_size(theme::text_2xl())
                    .font_weight(theme::WEIGHT_SEMIBOLD)
                    .child(self.project_name()),
                self.hint_group(
                    "hint-pr-nav",
                    vec![self.hint_keys::<PrevPr>(), self.hint_keys::<NextPr>()],
                    HintAnchor::Below,
                ),
            ))
            .child(
                div()
                    .text_size(theme::text_base())
                    .text_color(theme::text_subtle())
                    .child(count.to_string()),
            )
            .child(
                div()
                    .pl(theme::space_2())
                    .text_size(theme::text_sm())
                    .text_color(theme::text_subtle())
                    .child(freshness),
            )
            .child(div().flex_1())
            .child(
                self.hinted::<Refresh>(
                    Button::new("refresh", "")
                        .icon(IconName::Refresh)
                        .variant(ButtonVariant::Ghost)
                        .tooltip(crate::features::keymap::tip::<Refresh>("Refresh"))
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.refresh(cx))),
                    HintAnchor::Corner,
                ),
            )
    }

    pub(crate) fn render_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let visible = self.visible_items(cx);
        let banner = self.error.clone().map(|e| {
            Banner::warning("refresh-error", e)
                .action(
                    "Retry",
                    cx.listener(|this, _: &ClickEvent, _, cx| this.retry(cx)),
                )
                .on_dismiss(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.error = None;
                    cx.notify()
                }))
        });
        let mut body = crate::ui::elastic::content("pr-list").flex().flex_col();
        if visible.is_empty() {
            body = body.child(self.render_empty(cx));
        }
        body = body.children(self.render_sections(&visible, cx));
        body = body.children(self.render_stale_section(cx));
        body = body.children(self.render_quiet_footer(cx));
        let rows = div()
            .id("pr-list")
            .track_scroll(&self.list_scroll)
            .flex()
            .flex_col()
            .flex_1()
            .min_h(theme::space_0())
            .map(crate::ui::scroll::y_only)
            .pb(layout::END_PAD)
            .child(body);
        let rows = crate::ui::elastic::y_scroller(rows, "pr-list", self.list_scroll.clone());

        let content = div()
            .flex()
            .flex_col()
            .w_full()
            .max_w(layout::CONTENT_MAX_W)
            .h_full()
            .pt(layout::TITLEBAR_INSET)
            .px(if self.detail.is_some() {
                theme::page_pad_x_narrow()
            } else {
                theme::page_pad_x()
            })
            .child(self.render_list_header(visible.len(), cx))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(theme::space_3())
                    .mt(theme::card_gap())
                    .child(self.render_involvement_tabs(cx))
                    .child(self.render_search(cx)),
            )
            .children(banner.map(|b| div().mt(theme::card_gap()).child(b)))
            .child(rows);
        // The column breathes: content is centered and capped, like a document.
        div()
            .flex()
            .justify_center()
            .flex_1()
            .min_w(theme::space_0())
            .h_full()
            .bg(self.surfaces().content())
            .child(content)
    }
}
