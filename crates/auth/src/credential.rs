//! A resolved token, where it came from, and how it is sent.

use base64::Engine as _;
use gv_core::ProviderKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenOrigin {
    Env,
    Keychain,
    Cli,
    ConfigFile,
}

impl TokenOrigin {
    pub fn label(self) -> &'static str {
        match self {
            Self::Env => "env",
            Self::Keychain => "keychain",
            Self::Cli => "cli",
            Self::ConfigFile => "config",
        }
    }
}

/// How the token goes on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialKind {
    /// `Authorization: Bearer <token>` (GitHub, OAuth tokens from glab and az).
    Bearer,
    /// `Authorization: Basic base64(":<token>")` (Azure DevOps PATs).
    Basic,
    /// `PRIVATE-TOKEN: <token>` (GitLab PATs).
    PrivateToken,
}

impl CredentialKind {
    /// The scheme for a token of `kind` from `origin`. Azure PATs are Basic, but a JWT
    /// (from az, pasted into an env var) stays Bearer.
    pub fn infer(kind: ProviderKind, origin: TokenOrigin, token: &str) -> Self {
        match (kind, origin) {
            (ProviderKind::Azure, TokenOrigin::Cli) => Self::Bearer,
            (ProviderKind::Azure, _) if token.starts_with("eyJ") => Self::Bearer,
            (ProviderKind::Azure, _) => Self::Basic,
            (ProviderKind::GitLab, TokenOrigin::Keychain) => Self::PrivateToken,
            _ => Self::Bearer,
        }
    }
}

#[derive(Clone)]
pub struct Credential {
    pub token: String,
    pub origin: TokenOrigin,
    pub kind: ProviderKind,
    pub scheme: CredentialKind,
}

impl Credential {
    pub fn new(kind: ProviderKind, origin: TokenOrigin, token: &str) -> Self {
        let token = token.trim().to_string();
        Self {
            scheme: CredentialKind::infer(kind, origin, &token),
            token,
            origin,
            kind,
        }
    }

    /// Header name and value that authenticate a request.
    pub fn header(&self) -> (&'static str, String) {
        header(self.scheme, &self.token)
    }

    /// The CLI can hand out a fresh token after a 401.
    pub fn refreshable(&self) -> bool {
        matches!(self.origin, TokenOrigin::Cli | TokenOrigin::ConfigFile)
    }
}

pub fn header(scheme: CredentialKind, token: &str) -> (&'static str, String) {
    match scheme {
        CredentialKind::Bearer => ("Authorization", format!("Bearer {token}")),
        CredentialKind::Basic => (
            "Authorization",
            format!(
                "Basic {}",
                base64::engine::general_purpose::STANDARD.encode(format!(":{token}"))
            ),
        ),
        CredentialKind::PrivateToken => ("PRIVATE-TOKEN", token.to_string()),
    }
}

impl std::fmt::Debug for Credential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credential")
            .field("origin", &self.origin)
            .field("kind", &self.kind)
            .field("scheme", &self.scheme)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn azure_pat_is_basic_with_empty_user() {
        let c = Credential::new(ProviderKind::Azure, TokenOrigin::Keychain, "abc\n");
        assert_eq!(c.scheme, CredentialKind::Basic);
        // base64(":abc")
        assert_eq!(c.header(), ("Authorization", "Basic OmFiYw==".to_string()));
    }

    #[test]
    fn schemes_per_provider_and_origin() {
        let s = |k, o, t| Credential::new(k, o, t).scheme;
        use ProviderKind::*;
        assert_eq!(s(Azure, TokenOrigin::Cli, "eyJx"), CredentialKind::Bearer);
        assert_eq!(s(Azure, TokenOrigin::Env, "eyJx"), CredentialKind::Bearer);
        assert_eq!(s(Azure, TokenOrigin::Env, "pat"), CredentialKind::Basic);
        assert_eq!(
            s(GitHub, TokenOrigin::Keychain, "ghp"),
            CredentialKind::Bearer
        );
        assert_eq!(
            s(GitLab, TokenOrigin::Keychain, "glpat"),
            CredentialKind::PrivateToken
        );
        assert_eq!(s(GitLab, TokenOrigin::Cli, "oauth"), CredentialKind::Bearer);
        let c = Credential::new(GitLab, TokenOrigin::Keychain, "glpat");
        assert_eq!(c.header(), ("PRIVATE-TOKEN", "glpat".to_string()));
        assert!(!c.refreshable());
        assert!(format!("{c:?}").find("glpat").is_none());
    }
}
