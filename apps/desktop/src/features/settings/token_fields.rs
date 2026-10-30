//! Pure helpers of the token dialog: field validation, default hosts, scope notes.

use gv_engine::{Account, ProviderKind};

/// Scopes a token needs, per provider.
pub(crate) fn scope_note(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::GitHub => "Needs the repo scope (classic token) or read access to the repos.",
        ProviderKind::GitLab => "Needs the read_api scope, or api to comment and approve.",
        ProviderKind::Azure => {
            "Needs Code (Read) and Build (Read); Code (Read & write) to comment and vote."
        }
    }
}

/// The account a dialog's fields name, or why they do not name one.
pub(crate) fn dialog_account(
    kind: ProviderKind,
    host: &str,
    org: &str,
) -> Result<Account, &'static str> {
    let host = host.trim();
    if host.is_empty() || host.contains(['/', ' ', ':']) {
        return Err("Enter a host like gitlab.example.com");
    }
    let org = org.trim().trim_matches('/');
    if kind == ProviderKind::Azure && (org.is_empty() || org.contains(['/', ' '])) {
        return Err("Enter the organization, as in dev.azure.com/<org>");
    }
    Ok(Account::new(
        kind,
        host,
        (kind == ProviderKind::Azure).then_some(org),
    ))
}

pub(crate) fn default_host(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::GitHub => "github.com",
        ProviderKind::GitLab => "gitlab.com",
        ProviderKind::Azure => "dev.azure.com",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_dialog_fields() {
        let a = dialog_account(ProviderKind::Azure, "dev.azure.com", " MyOrg/ ").unwrap();
        assert_eq!(a.name(), "azure:dev.azure.com/myorg");
        assert!(dialog_account(ProviderKind::Azure, "dev.azure.com", "").is_err());
        assert!(dialog_account(ProviderKind::GitLab, "https://x", "").is_err());
        let g = dialog_account(ProviderKind::GitLab, "git.example.org", "ignored").unwrap();
        assert_eq!(g.name(), "gitlab:git.example.org");
    }
}
