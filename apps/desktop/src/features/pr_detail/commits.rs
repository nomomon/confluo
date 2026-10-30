//! Overview commits card: a click filters the diff to the commit, the hover button opens it in the
//! browser, and right-click opens the shared `ContextMenu` (open, copy SHA, copy link).

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context, MouseButton, MouseDownEvent};
use gv_engine::CommitInfo;

use crate::app::{GitViewer, Layer};
use crate::features::keymap::{self, CopyLink, OpenInBrowser};
use crate::features::shared::relative_time;
use crate::state::Copied;
use crate::ui::atoms::{tooltip, Button, ButtonVariant, IconName, ListItem, Size};
use crate::ui::molecules::{ContextMenu, MenuItem};
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_commits(
        &self,
        commits: &[CommitInfo],
        reviewed: Option<&str>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let seen = reviewed.and_then(|r| commits.iter().position(|c| c.sha == r));
        let rows: Vec<AnyElement> = commits
            .iter()
            .enumerate()
            .rev()
            .map(|(i, c)| self.commit_row(i, c, seen.is_some_and(|s| i > s), cx))
            .collect();
        div()
            .flex()
            .flex_col()
            .gap(theme::space_1())
            .child(
                div()
                    .text_size(theme::text_sm())
                    .font_weight(theme::WEIGHT_MEDIUM)
                    .child(format!("Commits {}", commits.len())),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .p(theme::LIST_INSET)
                    .rounded(theme::LIST_RADIUS)
                    .border_1()
                    .border_color(theme::card_border())
                    .bg(self.surfaces().card())
                    .children(rows),
            )
            .children(self.commit_menu(commits, cx))
    }

    fn commit_row(
        &self,
        i: usize,
        c: &CommitInfo,
        new: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (sha, menu_sha) = (c.sha.clone(), c.sha.clone());
        let group = gpui::SharedString::from(format!("commit-g-{i}"));
        let open = c.url.clone().map(|url| {
            div()
                .flex_shrink_0()
                .opacity(0.)
                .group_hover(group.clone(), |s| s.opacity(1.))
                .child(
                    Button::new(format!("commit-open-{i}"), "")
                        .icon(IconName::ExternalLink)
                        .variant(ButtonVariant::Ghost)
                        .size(Size::Sm)
                        .tooltip("Open in browser")
                        .on_click(move |_, _, cx| {
                            cx.stop_propagation();
                            cx.open_url(&url)
                        }),
                )
        });
        div()
            .id(("commit", i))
            .group(group)
            .flex()
            .items_center()
            .gap(theme::space_2())
            .px(theme::space_2())
            .py(theme::item_pad_y())
            .list_item_radius(false, theme::RADIUS_SM)
            .text_size(theme::text_xs())
            .cursor_pointer()
            .tooltip(tooltip("Show this commit's changes"))
            .on_click(
                cx.listener(move |this, _: &ClickEvent, _, cx| this.show_commit(sha.clone(), cx)),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                    this.open_layer(Layer::CommitAt(menu_sha.clone(), ev.position), cx)
                }),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .font_family(theme::mono_font())
                    .text_color(if new {
                        theme::accent()
                    } else {
                        theme::text_subtle()
                    })
                    .child(c.short().to_string()),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(theme::space_0())
                    .truncate()
                    .child(c.message.clone()),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .text_color(theme::text_subtle())
                    .child(match c.at {
                        Some(t) => format!("{} {}", c.author, relative_time(t)),
                        None => c.author.clone(),
                    }),
            )
            .children(open)
            .into_any_element()
    }

    fn commit_menu(&self, commits: &[CommitInfo], cx: &mut Context<Self>) -> Option<AnyElement> {
        let Some(Layer::CommitAt(sha, at)) = self.layer.clone() else {
            return None;
        };
        let c = commits.iter().find(|c| c.sha == sha)?;
        let close = |this: &mut Self| this.layer = None;
        let mut menu = ContextMenu::new("commit-ctx", at)
            .on_dismiss(cx.listener(|this, _: &MouseDownEvent, _, cx| this.dismiss_layer(cx)));
        if let Some(url) = c.url.clone() {
            let link = url.clone();
            menu = menu.child(
                MenuItem::new("commit-open", "Open in browser")
                    .icon(IconName::ExternalLink)
                    .shortcut(keymap::kbd::<OpenInBrowser>())
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        close(this);
                        cx.open_url(&url)
                    })),
            );
            menu = menu.child(
                MenuItem::new("commit-link", "Copy commit link")
                    .icon(IconName::Link)
                    .shortcut(keymap::kbd::<CopyLink>())
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        close(this);
                        this.copy(Copied::Link, link.clone(), cx)
                    })),
            );
        }
        let full = c.sha.clone();
        menu = menu.child(
            MenuItem::new("commit-sha", "Copy SHA")
                .icon(IconName::Copy)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    close(this);
                    this.copy(Copied::Sha, full.clone(), cx)
                })),
        );
        Some(menu.into_any_element())
    }
}
