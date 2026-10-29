//! Detail header: icon actions, big title and a muted meta line.

use gpui::{div, prelude::*, ClickEvent, Context, MouseDownEvent};

use crate::app::{GitViewer, Menu};
use crate::features::shared::{number_label, relative_time};
use crate::state::{Copied, DetailState};
use crate::ui::atoms::{tooltip, Avatar, Button, ButtonVariant, IconName, NavItem, Pill};
use crate::ui::molecules::DropdownMenu;
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
        let icon = |id: &'static str, glyph: IconName, tip: &'static str| Button::new(id, "").icon(glyph).variant(ButtonVariant::Ghost).tooltip(tip);
        let actions = div()
            .flex()
            .items_center()
            .gap(theme::SPACE_1)
            .child(icon("close-detail", IconName::X, "Close  esc").on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.close_detail(cx))))
            .child(div().flex_1())
            .child(icon("detail-refresh", IconName::Refresh, "Refresh").on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.load_detail(cx))))
            .child(
                icon("copy-link", if self.copied == Some(Copied::Link) { IconName::Check } else { IconName::Copy }, "Copy link")
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.copy(Copied::Link, link.clone(), cx))),
            )
            .child(icon("open-browser", IconName::ExternalLink, "Open in browser  o").on_click(cx.listener(move |_, _: &ClickEvent, _, cx| cx.open_url(&url))))
            .child(icon("detail-more", IconName::Ellipsis, "More").on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.menu = if this.menu == Some(Menu::DetailMore) { None } else { Some(Menu::DetailMore) };
                cx.notify()
            })));
        let meta = div()
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .text_size(theme::text_xs())
            .text_color(theme::TEXT_MUTED)
            .child(div().flex().items_center().gap(theme::SPACE_1).child(Avatar::new(s.author.username.clone()).image(self.avatar(s.author.avatar_url.as_deref()))).child(s.author.username.clone()))
            .child(div().id("detail-repo").min_w(theme::SPACE_0).truncate().tooltip(tooltip(s.id.repo.path.clone())).child(self.repo_label(&s.id.repo)))
            .child(Pill::new(number_label(s), theme::TEXT_MUTED))
            .child(div().text_color(theme::TEXT_SUBTLE).child(loaded));
        div()
            .relative()
            .flex()
            .flex_col()
            .gap(theme::SPACE_2)
            .child(actions)
            .child(div().text_size(theme::text_xl()).font_weight(theme::WEIGHT_SEMIBOLD).child(s.title.clone()))
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
            .child(NavItem::new("menu-alias", "Set alias").on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.menu = None;
                this.open_alias(Some(repo.clone()), cx)
            })))
    }
}
