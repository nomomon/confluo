//! Accounts: signed-in provider hosts with their user, and how to sign in when disconnected.

use gpui::{div, prelude::*, px, AnyElement, ClickEvent, Context};
use gv_engine::{Account, ProviderKind};

use crate::app::GitViewer;
use crate::features::shared::provider_icon;
use crate::state::{host_avatar_url, sign_in_hint};
use crate::ui::atoms::{Avatar, Badge, BadgeVariant, Button, ButtonVariant, IconName, Skeleton};
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
        let mut rows: Vec<AnyElement> = match &self.hosts {
            None => (0..2)
                .map(|_| {
                    div()
                        .flex()
                        .items_center()
                        .gap(theme::space_3())
                        .px(theme::space_4())
                        .py(theme::space_3())
                        .child(Skeleton::new().size(theme::space_4()))
                        .child(Skeleton::new().h(theme::space_3()).w(px(180.)))
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
                            .gap(theme::space_2())
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
                        .gap(theme::space_3())
                        .px(theme::space_4())
                        .py(theme::space_3())
                        .child(provider_icon(h.kind))
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(theme::space_0p5())
                                .child(div().text_size(theme::text_sm()).child(self.host_label(h)))
                                .child(div().text_size(theme::text_xs()).child(who)),
                        )
                        .child(status)
                        .child(self.token_buttons(h.kind, &h.host, None, cx))
                        .into_any_element()
                })
                .collect(),
        };
        // Per-organization Azure DevOps tokens get a row each.
        for a in self.accounts_ui.stored.iter().filter(|a| a.org.is_some()) {
            rows.push(self.org_token_row(a, cx));
        }
        let add = Button::new("accounts-add", "Add account")
            .icon(IconName::Plus)
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.open_token_dialog(ProviderKind::Azure, None, None, cx)
            }));
        div()
            .child(self.settings_group(Some("Connected hosts"), rows))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(theme::space_2())
                    .pt(theme::space_3())
                    .child(add)
                    .child(recheck),
            )
            .child(
                div()
                    .pt(theme::space_3())
                    .text_size(theme::text_xs())
                    .text_color(theme::text_subtle())
                    .child("Confluo uses a token from an env var, then one in the system keychain, then the gh, glab and az CLI logins."),
            )
    }

    /// "Use token..." or "Replace token" and "Remove token" for an account.
    fn token_buttons(
        &self,
        kind: ProviderKind,
        host: &str,
        org: Option<&str>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let account = Account::new(kind, host, org);
        let stored = self.accounts_ui.stored.contains(&account);
        let id = account.name();
        let (h, o) = (host.to_string(), org.map(str::to_string));
        // Azure tokens are per org: the host row opens the dialog for the first org without one.
        let o = o.or_else(|| {
            (kind == ProviderKind::Azure)
                .then(|| self.azure_org_without_token())
                .flatten()
        });
        let label = if stored {
            "Replace token"
        } else {
            "Use token..."
        };
        let mut row = div().flex().gap(theme::space_1()).child(
            Button::new(format!("token-use-{id}"), label)
                .variant(ButtonVariant::Ghost)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.open_token_dialog(kind, Some(h.clone()), o.clone(), cx)
                })),
        );
        if stored {
            row = row.child(
                Button::new(format!("token-rm-{id}"), "Remove token")
                    .variant(ButtonVariant::Ghost)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.remove_stored_token(account.clone(), cx)
                    })),
            );
        }
        row
    }

    fn azure_org_without_token(&self) -> Option<String> {
        let stored = &self.accounts_ui.stored;
        self.engine.azure_orgs().into_iter().find(|o| {
            !stored
                .iter()
                .any(|a| a.org.as_deref() == Some(&o.to_ascii_lowercase()))
        })
    }

    fn org_token_row(&self, a: &Account, cx: &mut Context<Self>) -> AnyElement {
        let label = format!("{}/{}", a.host, a.org.as_deref().unwrap_or_default());
        div()
            .flex()
            .items_center()
            .gap(theme::space_3())
            .px(theme::space_4())
            .py(theme::space_3())
            .child(provider_icon(a.kind))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(theme::space_0p5())
                    .child(div().text_size(theme::text_sm()).child(label))
                    .child(
                        div()
                            .text_size(theme::text_xs())
                            .text_color(theme::text_muted())
                            .child("Token in the keychain"),
                    ),
            )
            .child(self.token_buttons(a.kind, &a.host, a.org.as_deref(), cx))
            .into_any_element()
    }
}
