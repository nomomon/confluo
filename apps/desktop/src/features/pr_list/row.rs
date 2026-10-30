//! A single PR row: state icon and title, a muted meta line, time and diff size.

use gpui::{canvas, div, prelude::*, ClickEvent, Context, MouseButton, MouseDownEvent, SharedString};
use gv_engine::ChangeRequest;

use crate::app::GitViewer;
use crate::state;
use crate::features::shared::ci::{ci_status, ci_words, status_icon};
use crate::features::shared::{layout, number_label, provider_mark, relative_time, row_hint, state_icon};
use crate::ui::atoms::{tooltip, Avatar, Icon, IconName, ListItem, Pill};
use crate::ui::theme;

impl GitViewer {
    /// The row is a `ListItem`: its highlight is an inset rounded rect inside the section card.
    pub(super) fn render_row(&self, c: &ChangeRequest, cx: &mut Context<Self>) -> impl IntoElement {
        let is_selected = self.cursor.as_ref() == Some(&c.id) || self.detail.as_ref().is_some_and(|d| d.summary.id == c.id);
        let starred = self.starred.contains(&c.id);
        let cr = c.clone();
        let menu_id = c.id.clone();
        let menu = self.render_row_menu(c, cx);
        let sid = c.id.clone();
        let group: SharedString = format!("row-{}", c.id.key()).into();
        let repo = c.id.repo.clone();
        let ci = ci_status(&c.checks, None).map(|st| {
            div()
                .id(SharedString::from(format!("ci-{}", c.id.key())))
                .flex()
                .flex_shrink_0()
                .items_center()
                .gap(theme::SPACE_1)
                .tooltip(tooltip(ci_words(&c.checks)))
                .child(status_icon(&format!("row-{}", c.id.key()), st))
                .when(c.checks.failed > 0, |e| e.child(div().whitespace_nowrap().text_color(theme::danger()).child(format!("{} failing", c.checks.failed))))
        });
        let meta = div()
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .min_w(theme::SPACE_0)
            .text_size(theme::text_xs())
            .text_color(theme::text_muted())
            .child(provider_mark(c.id.repo.kind))
            .child(
                div()
                    .id(SharedString::from(format!("repo-{}", c.id.key())))
                    .min_w(theme::SPACE_0)
                    .truncate()
                    .tooltip(tooltip(format!("{}  right-click to rename", c.id.repo.path)))
                    .on_mouse_down(MouseButton::Right, cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                        cx.stop_propagation();
                        this.open_repo_menu(repo.clone(), ev.position, cx)
                    }))
                    .child(self.repo_label(&c.id.repo)),
            )
            .when(!self.compact, |d| d.child(div().flex().items_center().gap(theme::SPACE_1P5).flex_shrink_0().child(Avatar::new(c.author.username.clone()).image(self.avatar(c.author.avatar_url.as_deref()))).child(c.author.username.clone())))
            .child(Pill::new(number_label(c), theme::success()).icon(IconName::GitPullRequest))
            .children(ci)
            .children(row_hint(c).filter(|_| !self.compact).map(|h| div().flex_shrink_0().whitespace_nowrap().text_color(theme::text_muted()).child(h)));
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
            .flex()
            .items_center()
            .gap(theme::SPACE_3)
            .min_h(theme::row_min_h())
            .pl(theme::row_pad_x())
            .pr(theme::SPACE_2)
            .py(theme::row_pad_y())
            .list_item(is_selected)
            .on_mouse_down(MouseButton::Right, cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                this.quiet.row_menu = Some((menu_id.clone(), ev.position));
                cx.notify()
            }))
            .on_click(cx.listener(move |this, ev: &ClickEvent, window, cx| {
                this.set_cursor(cr.id.clone(), cx);
                window.focus(&this.focus);
                if this.settings.open_in == crate::settings::OpenIn::Browser {
                    cx.open_url(&cr.web_url)
                } else {
                    // Click previews (again: hides it); a double click (or Enter) opens the review.
                    this.click_row(cr.clone(), ev.click_count(), cx)
                }
            }))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w(theme::SPACE_0)
                    .gap(theme::row_line_gap())
                    .child(
                        div()
                            .flex()
                            .w_full()
                            .min_w(theme::SPACE_0)
                            .items_center()
                            .gap(theme::SPACE_2P5)
                            .child(state_icon(c.state))
                            .child(div().flex_1().min_w(theme::SPACE_0).text_size(theme::text_md()).font_weight(theme::WEIGHT_MEDIUM).truncate().child(c.title.clone())),
                    )
                    .child(div().w_full().min_w(theme::SPACE_0).overflow_hidden().pl(theme::ICON_LG + theme::SPACE_2P5).child(meta)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_end()
                    .gap(theme::row_line_gap())
                    .w(layout::TIME_COL_W)
                    .flex_shrink_0()
                    .text_size(theme::text_xs())
                    .text_color(theme::text_muted())
                    .child(relative_time(c.updated_at))
                    .children(diff),
            )
            .child(star)
            .children(reveal)
            .children(menu)
    }
}
