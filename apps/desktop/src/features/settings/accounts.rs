//! Accounts: signed-in provider hosts with their user, and how to sign in when disconnected.

use gpui::{div, prelude::*, px, ClickEvent, Context};

use crate::app::GitViewer;
use crate::features::shared::provider_icon;
use crate::state::{host_avatar_url, sign_in_hint};
use crate::ui::atoms::{Avatar, Badge, BadgeVariant, Button, Skeleton};
use crate::ui::molecules::Field;
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_accounts(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let recheck = Button::new("accounts-recheck", "Re-check").on_click(cx.listener(
            |this, _: &ClickEvent, _, cx| {
                this.hosts = None;
                this.check_hosts(cx)
            },
        ));
        let rows = match &self.hosts {
            None => (0..2)
                .map(|_| {
                    div()
                        .flex()
                        .items_center()
                        .gap(theme::SPACE_3)
                        .px(theme::SPACE_4)
                        .py(theme::SPACE_3)
                        .child(Skeleton::new().size(theme::SPACE_4))
                        .child(Skeleton::new().h(theme::SPACE_3).w(px(180.)))
                        .into_any_element()
                })
                .collect(),
            Some(hosts) if hosts.is_empty() => vec![Field::new("No hosts found", div())
                .description("Add a project to discover its hosts")
                .horizontal()
                .into_any_element()],
            Some(hosts) => hosts
                .iter()
                .map(|h| {
                    let connected = h.user.is_some();
                    let url = h
                        .user
                        .as_deref()
                        .and_then(|u| host_avatar_url(h.kind, &h.host, u));
                    let who = match &h.user {
                        Some(u) => div()
                            .flex()
                            .items_center()
                            .gap(theme::SPACE_2)
                            .child(Avatar::new(u.clone()).image(self.avatar(url.as_deref())))
                            .child(format!("@{u}")),
                        None => div()
                            .text_color(theme::text_muted())
                            .child(sign_in_hint(h.kind)),
                    };
                    let status = if connected {
                        Badge::new("Connected").tint(theme::success())
                    } else {
                        Badge::new("Not signed in").variant(BadgeVariant::Destructive)
                    };
                    div()
                        .flex()
                        .items_center()
                        .gap(theme::SPACE_3)
                        .px(theme::SPACE_4)
                        .py(theme::SPACE_3)
                        .child(provider_icon(h.kind))
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(theme::SPACE_0P5)
                                .child(div().text_size(theme::text_sm()).child(self.host_label(h)))
                                .child(div().text_size(theme::text_xs()).child(who)),
                        )
                        .child(status)
                        .into_any_element()
                })
                .collect(),
        };
        div()
            .child(self.settings_group(Some("Connected hosts"), rows))
            .child(div().flex().justify_end().pt(theme::SPACE_3).child(recheck))
            .child(
                div()
                    .pt(theme::SPACE_3)
                    .text_size(theme::text_xs())
                    .text_color(theme::text_subtle())
                    .child("Confluo uses the gh, glab and az CLI logins; it stores no tokens."),
            )
    }
}
