//! Keychain account names: `<provider>:<host>`, and `azure:dev.azure.com/<org>` for a
//! per-organization Azure DevOps token.

use gv_core::ProviderKind;

/// Where a stored token applies. Host and org are kept lowercase (both are case-insensitive).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Account {
    pub kind: ProviderKind,
    pub host: String,
    /// Azure DevOps organization; None means every org of the host.
    pub org: Option<String>,
}

pub fn provider_name(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::GitHub => "github",
        ProviderKind::GitLab => "gitlab",
        ProviderKind::Azure => "azure",
    }
}

pub fn parse_provider(s: &str) -> Option<ProviderKind> {
    match s.to_ascii_lowercase().as_str() {
        "github" | "gh" => Some(ProviderKind::GitHub),
        "gitlab" | "gl" => Some(ProviderKind::GitLab),
        "azure" | "ado" | "azuredevops" => Some(ProviderKind::Azure),
        _ => None,
    }
}

impl Account {
    pub fn new(kind: ProviderKind, host: &str, org: Option<&str>) -> Self {
        let org = org
            .map(|o| o.trim().trim_matches('/').to_ascii_lowercase())
            .filter(|o| !o.is_empty());
        Self {
            kind,
            host: host.trim().trim_matches('/').to_ascii_lowercase(),
            org,
        }
    }

    /// The keychain account, e.g. `github:github.com` or `azure:dev.azure.com/myorg`.
    pub fn name(&self) -> String {
        let p = provider_name(self.kind);
        match &self.org {
            Some(o) => format!("{p}:{}/{o}", self.host),
            None => format!("{p}:{}", self.host),
        }
    }

    /// Parses `<provider>:<host>[/org]`. An org is only accepted for Azure DevOps.
    pub fn parse(s: &str) -> Option<Self> {
        let (p, rest) = s.trim().split_once(':')?;
        let kind = parse_provider(p)?;
        let rest = rest.trim_start_matches("//").trim_end_matches('/');
        let (host, org) = match rest.split_once('/') {
            Some((h, o)) => (h, Some(o)),
            None => (rest, None),
        };
        if host.is_empty() || host.contains(char::is_whitespace) {
            return None;
        }
        if org.is_some() && (kind != ProviderKind::Azure || org.is_some_and(|o| o.contains('/'))) {
            return None;
        }
        Some(Self::new(kind, host, org))
    }
}

impl std::fmt::Display for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_parses() {
        let a = Account::new(ProviderKind::Azure, "dev.azure.com", Some("MyOrg"));
        assert_eq!(a.name(), "azure:dev.azure.com/myorg");
        assert_eq!(Account::parse("azure:dev.azure.com/MyOrg"), Some(a));
        let g = Account::parse("github:GitHub.com").unwrap();
        assert_eq!(g.name(), "github:github.com");
        assert_eq!(g.org, None);
        assert_eq!(
            Account::parse("gitlab:git.example.org").unwrap().kind,
            ProviderKind::GitLab
        );
    }

    #[test]
    fn rejects_bad_names() {
        assert_eq!(Account::parse("github.com"), None);
        assert_eq!(Account::parse("bitbucket:x.org"), None);
        assert_eq!(Account::parse("github:github.com/org"), None);
        assert_eq!(Account::parse("azure:dev.azure.com/a/b"), None);
        assert_eq!(Account::parse("azure:"), None);
    }
}
