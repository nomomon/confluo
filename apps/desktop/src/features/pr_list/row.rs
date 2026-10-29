//! A single PR row: state icon and title, a muted meta line, time and diff size.

use gpui::{canvas, div, prelude::*, ClickEvent, Context, SharedString};
use gv_engine::ChangeRequest;

use crate::app::GitViewer;
use crate::state;
use crate::features::shared::{checks_text, layout, number_label, provider_badge, relative_time, row_hint, state_icon};
use crate::ui::atoms::{tooltip, Avatar, Icon, IconName, Pill};
use crate::ui::theme;

impl GitViewer {
    /// `edges` (first, last) round the row so its hover fill stays inside the card corners.
    pub(super) fn render_row(&self, c: &ChangeRequest, show_provider: bool, edges: (bool, bool), cx: &mut Context<Self>) -> impl IntoElement {
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
            .text_size(theme::text_sm())
            .text_color(theme::text_muted())
            .when(show_provider, |d| d.child(provider_badge(c.id.repo.kind)))
            .child(
                div()
                    .id(SharedString::from(format!("repo-{}", c.id.key())))
                    .min_w(theme::SPACE_0)
                    .truncate()
                    .tooltip(tooltip(c.id.repo.path.clone()))
                    .child(self.repo_label(&c.id.repo)),
            )
            .child(div().flex().items_center().gap(theme::SPACE_1P5).flex_shrink_0().child(Avatar::new(c.author.username.clone()).image(self.avatar(c.author.avatar_url.as_deref()))).child(c.author.username.clone()))
            .child(Pill::new(number_label(c), theme::success()).icon(IconName::GitPullRequest))
            .child(div().flex_shrink_0().text_color(checks_color).child(checks))
            .children(row_hint(c).map(|h| div().flex_shrink_0().text_color(theme::text_muted()).child(h)));
        let diff = match (c.additions, c.deletions) {
            (Some(a), Some(d)) => Some(
                div()
                    .flex()
                    .gap(theme::SPACE_2)
                    .font_family(theme::mono_font())
                    .child(div().text_color(theme::success()).child(format!("+{a}")))
                    .child(div().text_color(theme::danger()).child(format!("-{d}"))),
            ),
            _ => None,
        };
        let is_cursor = self.cursor.as_ref() == Some(&c.id);
        let reveal = is_cursor.then(|| {
            let (flag, handle) = (self.reveal_cursor.clone(), self.list_scroll.clone());
            canvas(
                move |bounds, window, _| {
                    if flag.replace(false) && state::reveal(&handle, bounds, window) {
                        window.refresh();
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full()
        });
        let star = div()
            .id(SharedString::from(format!("star-{}", c.id.key())))
            .px(theme::SPACE_2)
            .cursor_pointer()
            .child(Icon::new(if starred { IconName::StarFilled } else { IconName::Star }).color(if starred { theme::warning() } else { theme::text_subtle() }))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                this.toggle_star(sid.clone(), cx)
            }));
        div()
            .id(group.clone())
            .group(group)
            .relative()
            .when(edges.0, |d| d.rounded_t(theme::RADIUS_XL))
            .when(edges.1, |d| d.rounded_b(theme::RADIUS_XL))
            .flex()
            .items_center()
            .gap(theme::SPACE_4)
            .min_h(theme::ROW_MIN_H)
            .pl(theme::SPACE_5)
            .pr(theme::SPACE_4)
            .py(theme::SPACE_4)
            .when(is_selected, |d| d.bg(theme::surface_active()))
            .hover(|s| s.bg(theme::surface_hover()))
            .active(|s| s.bg(theme::surface_active()))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.set_cursor(cr.id.clone(), cx);
                window.focus(&this.focus);
                this.open_detail(cr.clone(), cx)
            }))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w(theme::SPACE_0)
                    .gap(theme::SPACE_2)
                    .child(
                        div()
                            .flex()
                            .w_full()
                            .min_w(theme::SPACE_0)
                            .items_center()
                            .gap(theme::SPACE_3)
                            .child(state_icon(c.state))
                            .child(div().flex_1().min_w(theme::SPACE_0).text_size(theme::text_md()).font_weight(theme::WEIGHT_MEDIUM).truncate().child(c.title.clone())),
                    )
                    .child(div().w_full().min_w(theme::SPACE_0).overflow_hidden().pl(theme::SPACE_7).child(meta)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_end()
                    .gap(theme::SPACE_2)
                    .w(layout::TIME_COL_W)
                    .flex_shrink_0()
                    .text_size(theme::text_sm())
                    .text_color(theme::text_muted())
                    .child(relative_time(c.updated_at))
                    .children(diff),
            )
            .child(star)
            .children(reveal)
    }
}
