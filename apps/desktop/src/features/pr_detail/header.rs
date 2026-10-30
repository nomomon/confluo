//! Detail header: icon actions, big title and a muted meta line.

use gpui::{div, prelude::*, ClickEvent, Context, MouseDownEvent};

use crate::app::{GitViewer, Menu};
use crate::features::keymap::{self, CloseDetail, CopyLink, OpenInBrowser};
use crate::features::shared::{number_label, relative_time};
use crate::state::{Copied, DetailState};
use crate::ui::atoms::{tooltip, Avatar, Button, ButtonVariant, IconName, Pill};
use crate::ui::molecules::{DropdownMenu, MenuItem, MenuSeparator};
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
        let link = s.web_url.clone();
        let icon = |id: &'static str, glyph: IconName, tip: gpui::SharedString| Button::new(id, "").icon(glyph).variant(ButtonVariant::Ghost).tooltip(tip);
        let actions = div()
            .flex()
            .items_center()
            .gap(theme::SPACE_1)
            .child(icon("close-detail", IconName::X, keymap::tip::<CloseDetail>("Close")).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.close_detail(cx))))
            .child(div().flex_1())
            .child(icon("detail-refresh", IconName::Refresh, "Refresh detail".into()).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.load_detail(cx))))
            .child(
                icon("copy-link", if self.copied == Some(Copied::Link) { IconName::Check } else { IconName::Copy }, keymap::tip::<CopyLink>("Copy link"))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.copy(Copied::Link, link.clone(), cx))),
            )
            .child(icon("open-browser", IconName::ExternalLink, keymap::tip::<OpenInBrowser>("Open in browser")).on_click(cx.listener(move |_, _: &ClickEvent, _, cx| cx.open_url(&url))))
            .children(self.render_merge_button(d, cx).map(|b| div().pl(theme::SPACE_2).child(b)))
            .children(self.render_review_button(d, cx).map(|b| div().pl(theme::SPACE_1).child(b)))
            .child(icon("detail-more", IconName::Ellipsis, "More".into()).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.menu = if this.menu == Some(Menu::DetailMore) { None } else { Some(Menu::DetailMore) };
                cx.notify()
            })));
        let meta = div()
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .text_size(theme::text_xs())
            .text_color(theme::text_muted())
            .child(div().flex().items_center().gap(theme::SPACE_1).child(Avatar::new(s.author.username.clone()).image(self.avatar(s.author.avatar_url.as_deref()))).child(s.author.username.clone()))
            .child(div().id("detail-repo").min_w(theme::SPACE_0).truncate().tooltip(tooltip(s.id.repo.path.clone())).child(self.repo_label(&s.id.repo)))
            .child(Pill::new(number_label(s), theme::success()).icon(IconName::GitPullRequest))
            .children(self.render_review_state(d))
            .child(div().text_color(theme::text_subtle()).child(loaded));
        div()
            .relative()
            .flex()
            .flex_col()
            .gap(theme::SPACE_2)
            .child(actions)
            .child(self.render_title(d, cx))
            .child(meta)
            .when(self.menu == Some(Menu::DetailMore), |el| el.child(self.render_detail_menu(d, cx)))
    }

    fn render_detail_menu(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let repo = d.summary.id.repo.clone();
        DropdownMenu::new("detail-more-menu")
            .top(theme::SPACE_8)
            .right(theme::SPACE_0)
            .on_dismiss(cx.listener(|this, _: &MouseDownEvent, _, cx| {
                this.menu = None;
                cx.notify()
            }))
            .child(MenuItem::new("menu-open", "Open in browser").icon(IconName::ExternalLink).shortcut(keymap::kbd::<OpenInBrowser>()).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.menu = None;
                if let Some(d) = &this.detail {
                    cx.open_url(&d.summary.web_url)
                }
            })))
            .children(self.edit_menu_items(d, cx))
            .child(MenuSeparator)
            .children(self.snooze_items(&d.summary, cx))
            .child(MenuSeparator)
            .child(MenuItem::new("menu-alias", "Set alias").icon(IconName::Pencil).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.menu = None;
                this.open_alias(Some(repo.clone()), cx)
            })))
    }
}
