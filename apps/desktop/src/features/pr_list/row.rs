//! A single PR row: state icon and title, a muted meta line, time and diff size.

use gpui::{div, prelude::*, ClickEvent, Context, SharedString};
use gv_engine::ChangeRequest;

use crate::app::GitViewer;
use crate::features::shared::{checks_text, layout, number_label, provider_badge, relative_time, state_icon};
use crate::ui::atoms::{Avatar, Pill};
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_row(&self, c: &ChangeRequest, show_provider: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let is_selected = self.cursor.as_ref() == Some(&c.id) || self.detail.as_ref().is_some_and(|d| d.summary.id == c.id);
        let starred = self.starred.contains(&c.id);
        let cr = c.clone();
        let sid = c.id.clone();
        let group: SharedString = format!("row-{}", c.id.key()).into();
        let (checks, checks_color) = checks_text(c);
        let meta = div()
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .min_w(theme::SPACE_0)
            .text_size(theme::TEXT_XS)
            .text_color(theme::TEXT_MUTED)
            .when(show_provider, |d| d.child(provider_badge(c.id.repo.kind)))
            .child(div().flex().items_center().gap(theme::SPACE_1).flex_shrink_0().child(Avatar::new(c.author.username.clone())).child(c.author.username.clone()))
            .child(div().truncate().child(self.repo_label(&c.id.repo)))
            .child(Pill::new(number_label(c), theme::TEXT_MUTED))
            .child(div().flex_shrink_0().text_color(checks_color).child(checks));
        let diff = match (c.additions, c.deletions) {
            (Some(a), Some(d)) => Some(
                div()
                    .flex()
                    .gap(theme::SPACE_1)
                    .child(div().text_color(theme::SUCCESS).child(format!("+{a}")))
                    .child(div().text_color(theme::DANGER).child(format!("-{d}"))),
            ),
            _ => None,
        };
        let star = div()
            .id(SharedString::from(format!("star-{}", c.id.key())))
            .px(theme::SPACE_1)
            .text_color(if starred { theme::WARNING } else { theme::TRANSPARENT })
            .group_hover(group.clone(), |s| s.text_color(if starred { theme::WARNING } else { theme::TEXT_SUBTLE }))
            .hover(|s| s.text_color(theme::WARNING))
            .child(if starred { "\u{2605}" } else { "\u{2606}" })
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                this.toggle_star(sid.clone(), cx)
            }));
        div()
            .id(group.clone())
            .group(group)
            .flex()
            .items_center()
            .gap(theme::SPACE_3)
            .px(theme::SPACE_2P5)
            .py(theme::SPACE_2)
            .rounded(theme::RADIUS_MD)
            .when(is_selected, |d| d.bg(theme::SURFACE_ACTIVE))
            .hover(|s| s.bg(theme::SURFACE_HOVER))
            .active(|s| s.bg(theme::SURFACE_ACTIVE))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.open_detail(cr.clone(), cx)))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w(theme::SPACE_0)
                    .gap(theme::SPACE_1)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(theme::SPACE_2)
                            .child(state_icon(c.state))
                            .child(div().flex_1().min_w(theme::SPACE_0).text_size(theme::TEXT_SM).font_weight(theme::WEIGHT_MEDIUM).truncate().child(c.title.clone())),
                    )
                    .child(div().pl(theme::SPACE_6).child(meta)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_end()
                    .gap(theme::SPACE_1)
                    .w(layout::TIME_COL_W)
                    .flex_shrink_0()
                    .text_size(theme::TEXT_XS)
                    .text_color(theme::TEXT_SUBTLE)
                    .child(relative_time(c.updated_at))
                    .children(diff),
            )
            .child(star)
    }
}
