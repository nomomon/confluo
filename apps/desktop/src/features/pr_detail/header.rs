//! Detail header: provider, repo, actions, title and author line.

use gpui::{div, prelude::*, ClickEvent, Context, FontWeight};

use crate::app::GitViewer;
use crate::features::shared::{provider_badge, relative_time};
use crate::state::DetailState;
use crate::ui::atoms::{Button, ButtonVariant};
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_detail_header(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let s = &d.summary;
        let loaded = match (d.loading, d.loaded_at) {
            (true, _) => "Loading...".to_string(),
            (false, Some(t)) => format!("Loaded {}", relative_time(t)),
            (false, None) => "Cached".into(),
        };
        let url = s.web_url.clone();
        let repo = s.id.repo.clone();
        let actions = div()
            .flex()
            .items_center()
            .gap_2()
            .child(provider_badge(s.id.repo.kind))
            .child(div().text_xs().text_color(theme::muted()).truncate().child(format!("{} #{}", self.repo_label(&s.id.repo), s.id.number)))
            .child(div().flex_1())
            .child(Button::new("open-browser", "Open in browser").on_click(cx.listener(move |_, _: &ClickEvent, _, cx| cx.open_url(&url))))
            .child(
                Button::new("alias-btn", "Alias")
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.open_alias(Some(repo.clone()), cx))),
            )
            .child(Button::new("close-detail", "").icon("\u{2715}").variant(ButtonVariant::Ghost).on_click(cx.listener(
                |this, _: &ClickEvent, _, cx| {
                    this.detail = None;
                    cx.notify()
                },
            )));
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(actions)
            .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child(s.title.clone()))
            .child(div().flex().gap_2().text_xs().text_color(theme::muted()).child(format!("by {}", s.author.username)).child("\u{00B7}").child(loaded))
    }
}
