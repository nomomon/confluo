//! Which credential a request uses: an organization's own PAT for URLs under that org,
//! else the host-wide credential.

use super::AzureProvider;
use gv_auth::{Account, Credential};
use gv_core::ProviderKind;
use std::collections::HashMap;

/// Organization of `https://<host>/<org>/...` (lowercase), None for other URLs.
pub(crate) fn org_of_url(host: &str, url: &str) -> Option<String> {
    let rest = url
        .strip_prefix("https://")?
        .strip_prefix(host)?
        .strip_prefix('/')?;
    let org = rest.split(['/', '?']).next()?;
    let org = percent_encoding::percent_decode_str(org)
        .decode_utf8()
        .ok()?;
    (!org.is_empty()).then(|| org.to_ascii_lowercase())
}

impl AzureProvider {
    /// Use these per-organization credentials (org names are matched case-insensitively).
    pub fn with_org_creds(self, creds: Vec<(String, Credential)>) -> Self {
        *self
            .org_creds
            .try_write()
            .expect("new provider is not shared") = creds
            .into_iter()
            .map(|(o, c)| (o.to_ascii_lowercase(), c))
            .collect::<HashMap<_, _>>();
        self
    }

    /// The credential for `url`, and its org when it is an org-scoped one.
    pub(crate) async fn cred_for(&self, url: &str) -> (Credential, Option<String>) {
        if let Some(org) = org_of_url(&self.host, url) {
            if let Some(c) = self.org_creds.read().await.get(&org) {
                return (c.clone(), Some(org));
            }
        }
        (self.cred.read().await.clone(), None)
    }

    /// Ask az for a fresh token (in the org's tenant for an org credential). True when renewed.
    pub(crate) async fn refresh(&self, org: Option<&str>) -> bool {
        let account = Account::new(ProviderKind::Azure, &self.host, org);
        let Ok(c) = self.creds.get_for(&account).await else {
            return false;
        };
        match org {
            Some(o) => {
                self.org_creds.write().await.insert(o.to_string(), c);
            }
            None => *self.cred.write().await = c,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gv_auth::{CredentialKind, TokenOrigin};
    use gv_core::ProviderKind;

    #[test]
    fn org_from_url() {
        let h = "dev.azure.com";
        assert_eq!(
            org_of_url(h, "https://dev.azure.com/MyOrg/p/_apis/git?x").as_deref(),
            Some("myorg")
        );
        assert_eq!(
            org_of_url(h, "https://dev.azure.com/My%20Org/_apis").as_deref(),
            Some("my org")
        );
        assert_eq!(
            org_of_url(h, "https://app.vssps.visualstudio.com/_apis/profile"),
            None
        );
        assert_eq!(org_of_url(h, "https://dev.azure.com/"), None);
    }

    #[tokio::test]
    async fn org_token_wins_for_its_org() {
        let host = Credential::new(ProviderKind::Azure, TokenOrigin::Cli, "eyJhost");
        let org = Credential::new(ProviderKind::Azure, TokenOrigin::Keychain, "pat");
        let p =
            AzureProvider::new("dev.azure.com", host).with_org_creds(vec![("OrgA".into(), org)]);
        let (c, scoped) = p
            .cred_for("https://dev.azure.com/orga/_apis/connectionData")
            .await;
        assert_eq!(scoped.as_deref(), Some("orga"));
        assert_eq!(c.scheme, CredentialKind::Basic);
        let (c, scoped) = p.cred_for("https://dev.azure.com/other/_apis/x").await;
        assert_eq!(scoped, None);
        assert_eq!(c.scheme, CredentialKind::Bearer);
    }
}
