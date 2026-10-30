//! "Use token" dialog over Settings: provider and host (for a new account), the Azure DevOps
//! organization, a masked token field, Test (whoami) and Save to the keychain.

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context};
use gv_engine::ProviderKind;

use super::token_fields::scope_note;
use super::token_state::TokenStatus;
use crate::app::GitViewer;
use crate::ui::atoms::{Button, ButtonVariant, Spinner};
use crate::ui::molecules::{Dialog, Field, ToggleGroup};
use crate::ui::theme;

const KINDS: [(ProviderKind, &str); 3] = [
    (ProviderKind::GitHub, "GitHub"),
    (ProviderKind::GitLab, "GitLab"),
    (ProviderKind::Azure, "Azure DevOps"),
];

impl GitViewer {
    pub(crate) fn render_token_dialog(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let d = self.accounts_ui.dialog.as_ref()?;
        let mut body = div().flex().flex_col().gap(theme::space_3());
        if !d.fixed {
            let kinds = KINDS
                .iter()
                .fold(ToggleGroup::new("token-kind"), |g, (k, label)| {
                    g.item(*label, None, *k == d.kind)
                });
            let view = cx.entity().downgrade();
            body = body
                .child(Field::new(
                    "Provider",
                    kinds.on_change(move |i, _, cx| {
                        view.update(cx, |t, cx| t.set_token_dialog_kind(KINDS[*i].0, cx))
                            .ok();
                    }),
                ))
                .child(Field::new("Host", d.host.clone()));
        }
        if d.kind == ProviderKind::Azure {
            body = body.child(
                Field::new("Organization", d.org.clone())
                    .description("Tokens are per organization, as in dev.azure.com/<org>"),
            );
        }
        body = body.child(Field::new("Token", d.token.clone()).description(scope_note(d.kind)));
        let status = match &d.status {
            TokenStatus::Idle => None,
            TokenStatus::Busy => Some(
                div()
                    .child(Spinner::new("token-spinner"))
                    .into_any_element(),
            ),
            TokenStatus::Ok(u) => Some(
                div()
                    .text_color(theme::success())
                    .child(format!("Signed in as @{u}"))
                    .into_any_element(),
            ),
            TokenStatus::Err(e) => Some(
                div()
                    .text_color(theme::danger())
                    .child(format!("Token rejected: {e}"))
                    .into_any_element(),
            ),
        };
        let busy = d.status == TokenStatus::Busy;
        let footer = div()
            .flex()
            .items_center()
            .gap(theme::space_2())
            .child(div().flex_1().text_size(theme::text_xs()).children(status))
            .child(
                Button::new("token-cancel", "Cancel")
                    .variant(ButtonVariant::Ghost)
                    .on_click(cx.listener(|t, _: &ClickEvent, _, cx| t.close_token_dialog(cx))),
            )
            .child(Button::new("token-test", "Test").on_click(cx.listener(
                move |t, _: &ClickEvent, _, cx| {
                    if !busy {
                        t.submit_token(false, cx)
                    }
                },
            )))
            .child(
                Button::new("token-save", "Save to Keychain")
                    .variant(ButtonVariant::Primary)
                    .on_click(cx.listener(move |t, _: &ClickEvent, _, cx| {
                        if !busy {
                            t.submit_token(true, cx)
                        }
                    })),
            );
        let close = cx.listener(|t, _: &(), _, cx| t.close_token_dialog(cx));
        Some(
            Dialog::new("Use a personal access token")
                .description(
                    "Stored in the system keychain, never in settings. Used before the CLI login.",
                )
                .content(body)
                .footer(footer)
                .on_close(move |w, cx| close(&(), w, cx))
                .into_any_element(),
        )
    }
}
