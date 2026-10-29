//! Credential lookup: env var override, then the provider CLI (gh / glab / az),
//! then glab's config file as a fallback.

use gv_core::ProviderKind;
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("no credentials for {kind} at {host}; {hint}")]
    Missing { kind: ProviderKind, host: String, hint: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenOrigin {
    Env,
    Cli,
    ConfigFile,
}

#[derive(Clone)]
pub struct Credential {
    pub token: String,
    pub origin: TokenOrigin,
    /// GitLab OAuth tokens (from glab) use `Authorization: Bearer`, PATs use `PRIVATE-TOKEN`.
    /// Bearer works for both on GitLab, so providers can always send Bearer.
    pub kind: ProviderKind,
}

impl std::fmt::Debug for Credential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credential").field("origin", &self.origin).field("kind", &self.kind).finish()
    }
}

/// Env vars checked, in order. A host-specific var wins, e.g. `GV_TOKEN_GITLAB_COM`.
pub fn env_vars(kind: ProviderKind, host: &str) -> Vec<String> {
    let host_var: String = host.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' }).collect();
    let mut v = vec![format!("GV_TOKEN_{host_var}")];
    match kind {
        ProviderKind::GitHub => v.extend(["GV_GITHUB_TOKEN".into(), "GH_TOKEN".into(), "GITHUB_TOKEN".into()]),
        ProviderKind::GitLab => v.extend(["GV_GITLAB_TOKEN".into(), "GITLAB_TOKEN".into()]),
        ProviderKind::Azure => v.extend(["GV_AZURE_TOKEN".into(), "AZURE_DEVOPS_EXT_PAT".into()]),
    }
    v
}

const AZURE_DEVOPS_RESOURCE: &str = "499b84ac-1321-427f-aa17-267ca6b1a4fa";

#[derive(Debug, Default, Clone)]
pub struct CredentialSource;

impl CredentialSource {
    pub fn new() -> Self {
        Self
    }

    pub async fn get(&self, kind: ProviderKind, host: &str) -> Result<Credential, AuthError> {
        for var in env_vars(kind, host) {
            if let Ok(t) = std::env::var(&var) {
                if !t.trim().is_empty() {
                    return Ok(Credential { token: t.trim().to_string(), origin: TokenOrigin::Env, kind });
                }
            }
        }
        let cli = match kind {
            ProviderKind::GitHub => run("gh", &["auth", "token", "--hostname", host]).await,
            ProviderKind::GitLab => run("glab", &["config", "get", "token", "--host", host]).await,
            ProviderKind::Azure => {
                run("az", &["account", "get-access-token", "--resource", AZURE_DEVOPS_RESOURCE, "--query", "accessToken", "-o", "tsv"]).await
            }
        };
        if let Some(t) = cli {
            return Ok(Credential { token: t, origin: TokenOrigin::Cli, kind });
        }
        if kind == ProviderKind::GitLab {
            if let Some(t) = glab_config_token(host) {
                return Ok(Credential { token: t, origin: TokenOrigin::ConfigFile, kind });
            }
        }
        let hint = match kind {
            ProviderKind::GitHub => format!("run `gh auth login --hostname {host}` or set GH_TOKEN"),
            ProviderKind::GitLab => format!("run `glab auth login --hostname {host}` or set GITLAB_TOKEN"),
            ProviderKind::Azure => "run `az login` or set AZURE_DEVOPS_EXT_PAT".into(),
        };
        Err(AuthError::Missing { kind, host: host.into(), hint })
    }

    /// glab refreshes expired OAuth tokens when it makes an API call. Trigger that,
    /// then read the token again. Used by the GitLab provider after a 401.
    pub async fn refresh_gitlab(&self, host: &str) -> Result<Credential, AuthError> {
        let _ = run("glab", &["api", "--hostname", host, "user"]).await;
        self.get(ProviderKind::GitLab, host).await
    }
}

async fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().await.ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (!s.is_empty()).then_some(s)
}

fn glab_config_paths() -> Vec<PathBuf> {
    let mut v = vec![];
    if let Ok(d) = std::env::var("GLAB_CONFIG_DIR") {
        v.push(PathBuf::from(d).join("config.yml"));
    }
    if let Ok(h) = std::env::var("HOME") {
        let h = PathBuf::from(h);
        v.push(h.join("Library/Application Support/glab-cli/config.yml"));
        v.push(h.join(".config/glab-cli/config.yml"));
    }
    v
}

/// Hosts configured in glab (used by discovery to classify self-hosted GitLab).
pub fn glab_hosts() -> Vec<String> {
    glab_config_paths()
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .flat_map(|s| parse_glab_config(&s).into_iter().map(|(h, _)| h))
        .collect()
}

fn glab_config_token(host: &str) -> Option<String> {
    glab_config_paths().iter().filter_map(|p| std::fs::read_to_string(p).ok()).find_map(|s| {
        parse_glab_config(&s).into_iter().find(|(h, _)| h.eq_ignore_ascii_case(host)).and_then(|(_, t)| t)
    })
}

/// Minimal parser for the `hosts:` section of glab's config.yml.
/// Returns (host, token) pairs. Avoids pulling in a YAML dependency.
fn parse_glab_config(s: &str) -> Vec<(String, Option<String>)> {
    let mut out: Vec<(String, Option<String>)> = vec![];
    let mut in_hosts = false;
    let mut host_indent = None;
    for line in s.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = line.len() - trimmed.len();
        if indent == 0 {
            in_hosts = trimmed.starts_with("hosts:");
            host_indent = None;
            continue;
        }
        if !in_hosts {
            continue;
        }
        let hi = *host_indent.get_or_insert(indent);
        if indent == hi {
            if let Some(h) = trimmed.strip_suffix(':') {
                out.push((h.trim_matches('"').to_string(), None));
            }
        } else if indent > hi {
            if let Some(v) = trimmed.strip_prefix("token:") {
                let v = v.trim().trim_matches('"').trim_matches('\'');
                if let Some(last) = out.last_mut() {
                    if !v.is_empty() {
                        last.1 = Some(v.to_string());
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_glab_hosts() {
        let cfg = "host: gitlab.com\nhosts:\n    gitlab.com:\n        # c\n        token: abc\n        user: me\n    git.example.org:\n        api_host: x\n        token: \"def\"\nother: 1\n";
        let p = parse_glab_config(cfg);
        assert_eq!(p, vec![("gitlab.com".into(), Some("abc".into())), ("git.example.org".into(), Some("def".into()))]);
    }

    #[test]
    fn env_var_names() {
        assert_eq!(env_vars(ProviderKind::GitLab, "gitlab.com")[0], "GV_TOKEN_GITLAB_COM");
    }
}
