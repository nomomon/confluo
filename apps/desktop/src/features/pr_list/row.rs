//! A single PR row in the list.

use gpui::{div, prelude::*, ClickEvent, Context, SharedString};
use gv_engine::ChangeRequest;

use crate::app::GitViewer;
use crate::features::shared::{checks_summary, provider_badge, relative_time, review_summary, state_pill};
use crate::ui::theme;
use crate::features::shared::layout;

impl GitViewer {
    pub(super) fn render_row(&self, c: &ChangeRequest, is_selected: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let starred = self.starred.contains(&c.id);
        let cr = c.clone();
        let sid = c.id.clone();
        let meta = div()
            .flex()
            .gap(theme::SPACE_2)
            .text_size(theme::TEXT_XS)
            .text_color(theme::TEXT_MUTED)
            .child(div().truncate().child(self.repo_label(&c.id.repo)))
            .child(format!("#{}", c.id.number))
            .child(c.author.username.clone())
            .when(c.needs_you, |d| d.child(div().text_color(theme::ACCENT).child("needs you")));
        let star = div()
            .id(SharedString::from(format!("star-{}", c.id.key())))
            .px(theme::SPACE_1)
            .text_color(if starred { theme::WARNING } else { theme::TEXT_SUBTLE })
            .hover(|s| s.text_color(theme::WARNING))
            .child(if starred { "\u{2605}" } else { "\u{2606}" })
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                this.toggle_star(sid.clone(), cx)
            }));
        div()
            .id(SharedString::from(format!("row-{}", c.id.key())))
            .flex()
            .items_center()
            .gap(theme::SPACE_3)
            .px(theme::SPACE_3)
            .py(theme::SPACE_2)
            .rounded(theme::RADIUS_LG)
            .border_1()
            .border_color(if is_selected { theme::BORDER } else { theme::BORDER_SUBTLE })
            .bg(if is_selected { theme::SURFACE_ACTIVE } else { self.surfaces().card() })
            .hover(|s| s.bg(theme::SURFACE_HOVER))
            .active(|s| s.bg(theme::SURFACE_ACTIVE))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.open_detail(cr.clone(), cx)))
            .child(provider_badge(c.id.repo.kind))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w(theme::SPACE_0)
                    .gap(theme::SPACE_0P5)
                    .child(div().text_size(theme::TEXT_SM).font_weight(theme::WEIGHT_MEDIUM).truncate().child(c.title.clone()))
                    .child(meta),
            )
            .child(div().flex().flex_col().items_end().gap(theme::SPACE_0P5).flex_shrink_0().text_size(theme::TEXT_XS).child(review_summary(c)).child(checks_summary(c)))
            .child(state_pill(c.state))
            .child(div().w(layout::TIME_COL_W).flex_shrink_0().text_size(theme::TEXT_XS).text_color(theme::TEXT_SUBTLE).child(relative_time(c.updated_at)))
            .child(star)
    }
}
