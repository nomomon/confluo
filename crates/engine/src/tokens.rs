//! Personal access tokens in the system keychain: store, remove, test, and which source
//! (env, keychain, CLI) each account uses. Tokens are never logged or returned.

use crate::Engine;
use anyhow::{anyhow, Result};
use gv_auth::{Account, Credential, TokenOrigin};
use gv_core::{Provider, ProviderKind, User};
use std::collections::BTreeSet;

impl Engine {
    /// Save a token for `account` and drop the cached client so the next call uses it.
    pub async fn set_token(&self, account: &Account, token: &str) -> Result<()> {
        self.creds
            .store()
            .set(account, token)
            .map_err(|e| anyhow!("keychain: {e}"))?;
        self.forget_auth(account.kind, &account.host).await;
        Ok(())
    }

    pub async fn remove_token(&self, account: &Account) -> Result<()> {
        self.creds
            .store()
            .remove(account)
            .map_err(|e| anyhow!("keychain: {e}"))?;
        self.forget_auth(account.kind, &account.host).await;
        Ok(())
    }

    /// Accounts with a token in the keychain.
    pub fn stored_accounts(&self) -> Vec<Account> {
        self.creds.store().accounts()
    }

    /// Forget the client, failed sign-in and username of a host (credentials changed).
    pub async fn forget_auth(&self, kind: ProviderKind, host: &str) {
        let key = (kind, host.to_ascii_lowercase());
        self.providers.lock().await.remove(&key);
        self.auth_failed
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&key);
        self.me_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&key);
    }

    /// Who `token` signs in as on `account`, without storing it.
    pub async fn test_token(&self, account: &Account, token: &str) -> Result<User> {
        let (kind, host) = (account.kind, account.host.as_str());
        let cred = Credential::new(kind, TokenOrigin::Keychain, token);
        let p: Box<dyn Provider> = match kind {
            ProviderKind::GitHub => Box::new(gv_provider_github::GitHubProvider::new(
                host,
                cred.token.clone(),
            )),
            ProviderKind::GitLab => Box::new(gv_provider_gitlab::GitLabProvider::new(host, cred)),
            ProviderKind::Azure => {
                let p = gv_provider_azure::AzureProvider::new(host, cred);
                p.set_namespaces(account.org.iter().cloned().collect());
                Box::new(p)
            }
        };
        Ok(p.whoami().await?)
    }

    /// Every known account with the source its token comes from (None: signed out).
    /// Asks the CLIs, so this can take a moment.
    pub async fn auth_sources(&self) -> Vec<(Account, Option<TokenOrigin>)> {
        let mut accounts: BTreeSet<Account> = self
            .known_hosts()
            .into_iter()
            .map(|(k, h)| Account::new(k, &h, None))
            .collect();
        accounts.extend(self.stored_accounts());
        if let Some(azure) = accounts
            .iter()
            .find(|a| a.kind == ProviderKind::Azure)
            .cloned()
        {
            let orgs = self.azure_orgs();
            accounts.extend(
                orgs.iter()
                    .map(|o| Account::new(azure.kind, &azure.host, Some(o))),
            );
        }
        let mut out = vec![];
        for a in accounts {
            let origin = self.creds.get_for(&a).await.ok().map(|c| c.origin);
            out.push((a, origin));
        }
        out
    }
}
