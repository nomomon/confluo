//! A single PR row in the list.

use gpui::{div, prelude::*, px, ClickEvent, Context, FontWeight, SharedString};
use gv_engine::ChangeRequest;

use crate::app::GitViewer;
use crate::features::shared::{checks_summary, provider_badge, relative_time, review_summary, state_pill};
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_row(&self, c: &ChangeRequest, is_selected: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let starred = self.starred.contains(&c.id);
        let cr = c.clone();
        let sid = c.id.clone();
        let meta = div()
            .flex()
            .gap_2()
            .text_xs()
            .text_color(theme::muted())
            .child(div().truncate().child(self.repo_label(&c.id.repo)))
            .child(format!("#{}", c.id.number))
            .child(c.author.username.clone())
            .when(c.needs_you, |d| d.child(div().text_color(theme::accent()).child("needs you")));
        let star = div()
            .id(SharedString::from(format!("star-{}", c.id.key())))
            .px_1()
            .text_color(if starred { theme::yellow() } else { theme::faint() })
            .hover(|s| s.text_color(theme::yellow()))
            .child(if starred { "\u{2605}" } else { "\u{2606}" })
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                this.toggle_star(sid.clone(), cx)
            }));
        div()
            .id(SharedString::from(format!("row-{}", c.id.key())))
            .flex()
            .items_center()
            .gap_3()
            .px_3()
            .py_2()
            .rounded_lg()
            .border_1()
            .border_color(if is_selected { theme::accent().opacity(0.5) } else { theme::border() })
            .bg(if is_selected { theme::selected() } else { theme::card(self.settings.panel_opacity * 0.6) })
            .hover(|s| s.bg(theme::hover()))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.open_detail(cr.clone(), cx)))
            .child(provider_badge(c.id.repo.kind))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w(px(0.))
                    .gap_0p5()
                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).truncate().child(c.title.clone()))
                    .child(meta),
            )
            .child(div().flex().flex_col().items_end().gap_0p5().flex_shrink_0().text_xs().child(review_summary(c)).child(checks_summary(c)))
            .child(state_pill(c.state))
            .child(div().w(px(56.)).flex_shrink_0().text_xs().text_color(theme::faint()).child(relative_time(c.updated_at)))
            .child(star)
    }
}
