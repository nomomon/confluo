//! State of Settings > Accounts tokens: the keychain accounts and the "Use token" dialog
//! (test with whoami, save, remove). The token only lives in the masked input until saved.

use gpui::{AppContext, Context, Entity};
use gv_engine::{Account, ProviderKind};

use super::token_fields::{default_host, dialog_account};
use crate::app::GitViewer;
use crate::state::bridge;
use crate::ui::molecules::text_input::TextInput;

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum TokenStatus {
    Idle,
    Busy,
    /// Signed in as this user.
    Ok(String),
    Err(String),
}

pub(crate) struct TokenDialog {
    pub kind: ProviderKind,
    /// Provider and host come from an existing row (only the org is editable).
    pub fixed: bool,
    pub host: Entity<TextInput>,
    pub org: Entity<TextInput>,
    pub token: Entity<TextInput>,
    pub status: TokenStatus,
}

#[derive(Default)]
pub(crate) struct AccountsUi {
    pub dialog: Option<TokenDialog>,
    /// Accounts with a token in the keychain (read when Accounts is checked).
    pub stored: Vec<Account>,
}

impl GitViewer {
    pub(crate) fn reload_stored_tokens(&mut self) {
        self.accounts_ui.stored = self.engine.stored_accounts();
    }

    /// Open the token dialog for a known host (`fixed`) or for a new account.
    pub(crate) fn open_token_dialog(
        &mut self,
        kind: ProviderKind,
        host: Option<String>,
        org: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let fixed = host.is_some();
        let host_text = host.unwrap_or_else(|| default_host(kind).into());
        let field = |cx: &mut Context<Self>, ph: &'static str, v: String| {
            cx.new(|cx| {
                let mut i = TextInput::new(ph, cx);
                i.set_text(v, cx);
                i
            })
        };
        let host = field(cx, "Host", host_text);
        let org = field(cx, "Organization", org.unwrap_or_default());
        let token = cx.new(|cx| TextInput::new("Paste the token", cx).masked());
        self.accounts_ui.dialog = Some(TokenDialog {
            kind,
            fixed,
            host,
            org,
            token,
            status: TokenStatus::Idle,
        });
        cx.notify();
    }

    /// Dev only: `CONFLUO_TOKEN_DIALOG=add` (or an account like `azure:dev.azure.com/org`) opens
    /// the token dialog over Settings > Accounts with a fake token typed in.
    pub(crate) fn token_dialog_from_env(&mut self, cx: &mut Context<Self>) {
        let Ok(v) = std::env::var("CONFLUO_TOKEN_DIALOG") else {
            return;
        };
        self.open_settings(Some(crate::state::SettingsPage::Accounts), cx);
        match Account::parse(&v) {
            Some(a) => self.open_token_dialog(a.kind, Some(a.host), a.org, cx),
            None => self.open_token_dialog(ProviderKind::Azure, None, None, cx),
        }
        if let Some(d) = &self.accounts_ui.dialog {
            d.token
                .update(cx, |i, cx| i.set_text("not-a-real-token", cx));
        }
    }

    /// Switch the provider of an "Add account" dialog (resets the host to its default).
    pub(crate) fn set_token_dialog_kind(&mut self, kind: ProviderKind, cx: &mut Context<Self>) {
        if let Some(d) = self.accounts_ui.dialog.as_mut() {
            d.kind = kind;
            d.status = TokenStatus::Idle;
            d.host
                .update(cx, |i, cx| i.set_text(default_host(kind), cx));
        }
        cx.notify();
    }

    pub(crate) fn close_token_dialog(&mut self, cx: &mut Context<Self>) {
        self.accounts_ui.dialog = None;
        cx.notify();
    }

    fn dialog_input(&self, cx: &Context<Self>) -> Result<(Account, String), String> {
        let d = self.accounts_ui.dialog.as_ref().ok_or("no dialog")?;
        let account = dialog_account(d.kind, &d.host.read(cx).text(), &d.org.read(cx).text())?;
        let token = d.token.read(cx).text();
        if token.trim().is_empty() {
            return Err("Paste a token first".into());
        }
        Ok((account, token))
    }

    /// Whoami with the typed token; with `save`, store it in the keychain when it works.
    pub(crate) fn submit_token(&mut self, save: bool, cx: &mut Context<Self>) {
        let (account, token) = match self.dialog_input(cx) {
            Ok(v) => v,
            Err(e) => return self.set_token_status(TokenStatus::Err(e), cx),
        };
        self.set_token_status(TokenStatus::Busy, cx);
        let engine = self.engine.clone();
        let rx = bridge::spawn(async move {
            let user = engine.test_token(&account, &token).await?;
            if save {
                engine.set_token(&account, &token).await?;
            }
            anyhow::Ok(user.username)
        });
        cx.spawn(async move |this, cx| {
            let r = rx
                .await
                .unwrap_or_else(|_| Err(anyhow::anyhow!("cancelled")));
            this.update(cx, |t, cx| match r {
                Ok(_) if save => {
                    t.accounts_ui.dialog = None;
                    t.reload_stored_tokens();
                    t.hosts = None;
                    t.check_hosts(cx);
                    cx.notify()
                }
                Ok(u) => t.set_token_status(TokenStatus::Ok(u), cx),
                Err(e) => {
                    let msg = e
                        .to_string()
                        .lines()
                        .next()
                        .unwrap_or_default()
                        .chars()
                        .take(90)
                        .collect();
                    t.set_token_status(TokenStatus::Err(msg), cx)
                }
            })
            .ok();
        })
        .detach();
    }

    fn set_token_status(&mut self, s: TokenStatus, cx: &mut Context<Self>) {
        if let Some(d) = self.accounts_ui.dialog.as_mut() {
            d.status = s;
        }
        cx.notify();
    }

    pub(crate) fn remove_stored_token(&mut self, account: Account, cx: &mut Context<Self>) {
        let engine = self.engine.clone();
        let rx = bridge::spawn(async move { engine.remove_token(&account).await });
        cx.spawn(async move |this, cx| {
            let r = rx.await;
            this.update(cx, |t, cx| {
                if let Ok(Err(e)) = r {
                    t.error = Some(e.to_string());
                }
                t.reload_stored_tokens();
                t.hosts = None;
                t.check_hosts(cx);
            })
            .ok();
        })
        .detach();
    }
}
