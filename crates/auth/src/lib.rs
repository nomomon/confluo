//! Credential lookup per host: env var override, then a token in the system keychain, then
//! the provider CLI (gh / glab / az), then glab's config file as a fallback.

mod account;
mod azure_tenant;
mod credential;
mod store;

pub use account::{parse_provider, provider_name, Account};
pub use credential::{header, Credential, CredentialKind, TokenOrigin};
pub use store::{SecretBackend, TokenStore, SERVICE};

use gv_core::ProviderKind;
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("no credentials for {kind} at {host}; {hint}")]
    Missing {
        kind: ProviderKind,
        host: String,
        hint: String,
    },
}

/// Env vars checked, in order. A host-specific var wins, e.g. `GV_TOKEN_GITLAB_COM`.
pub fn env_vars(kind: ProviderKind, host: &str) -> Vec<String> {
    let host_var: String = host
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    let mut v = vec![format!("GV_TOKEN_{host_var}")];
    match kind {
        ProviderKind::GitHub => v.extend([
            "GV_GITHUB_TOKEN".into(),
            "GH_TOKEN".into(),
            "GITHUB_TOKEN".into(),
        ]),
        ProviderKind::GitLab => v.extend(["GV_GITLAB_TOKEN".into(), "GITLAB_TOKEN".into()]),
        ProviderKind::Azure => v.extend(["GV_AZURE_TOKEN".into(), "AZURE_DEVOPS_EXT_PAT".into()]),
    }
    v
}

/// The env and keychain steps of the lookup (no process spawned). For an Azure org the
/// org's own token wins over a host-wide one.
pub fn lookup_static(
    account: &Account,
    env: impl Fn(&str) -> Option<String>,
    store: &TokenStore,
) -> Option<Credential> {
    let kind = account.kind;
    if let Some(t) = env_vars(kind, &account.host)
        .iter()
        .filter_map(|v| env(v))
        .find(|t| !t.trim().is_empty())
    {
        return Some(Credential::new(kind, TokenOrigin::Env, &t));
    }
    let host_wide = Account::new(kind, &account.host, None);
    [account.clone(), host_wide]
        .iter()
        .find_map(|a| store.get(a))
        .map(|t| Credential::new(kind, TokenOrigin::Keychain, &t))
}

const AZURE_DEVOPS_RESOURCE: &str = "499b84ac-1321-427f-aa17-267ca6975798";

#[derive(Debug, Default, Clone)]
pub struct CredentialSource {
    store: TokenStore,
}

impl CredentialSource {
    /// Backed by the system keychain.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_store(store: TokenStore) -> Self {
        Self { store }
    }

    pub fn store(&self) -> &TokenStore {
        &self.store
    }

    pub async fn get(&self, kind: ProviderKind, host: &str) -> Result<Credential, AuthError> {
        self.get_for(&Account::new(kind, host, None)).await
    }

    /// Lookup for an account (an Azure org included): env, keychain, CLI, glab config.
    pub async fn get_for(&self, account: &Account) -> Result<Credential, AuthError> {
        let (kind, host) = (account.kind, account.host.as_str());
        if let Some(c) = lookup_static(account, |v| std::env::var(v).ok(), &self.store) {
            return Ok(c);
        }
        if let Some(t) = cli_token(kind, host, account.org.as_deref()).await {
            return Ok(Credential::new(kind, TokenOrigin::Cli, &t));
        }
        if kind == ProviderKind::GitLab {
            if let Some(t) = glab_config_token(host) {
                return Ok(Credential::new(kind, TokenOrigin::ConfigFile, &t));
            }
        }
        let hint = match kind {
            ProviderKind::GitHub => {
                format!("run `gh auth login --hostname {host}`, set GH_TOKEN or `gv auth set github:{host}`")
            }
            ProviderKind::GitLab => {
                format!("run `glab auth login --hostname {host}`, set GITLAB_TOKEN or `gv auth set gitlab:{host}`")
            }
            ProviderKind::Azure => {
                "run `az login`, set AZURE_DEVOPS_EXT_PAT or `gv auth set azure:dev.azure.com/<org>`".into()
            }
        };
        Err(AuthError::Missing {
            kind,
            host: host.into(),
            hint,
        })
    }

    /// Per-organization keychain tokens of an Azure DevOps host (env overrides them all).
    pub fn org_tokens(&self, kind: ProviderKind, host: &str) -> Vec<(String, Credential)> {
        if env_vars(kind, host)
            .iter()
            .any(|v| std::env::var(v).is_ok_and(|t| !t.trim().is_empty()))
        {
            return vec![];
        }
        let host = host.to_ascii_lowercase();
        self.store
            .accounts()
            .into_iter()
            .filter(|a| a.kind == kind && a.host == host)
            .filter_map(|a| {
                let org = a.org.clone()?;
                let t = self.store.get(&a)?;
                Some((org, Credential::new(kind, TokenOrigin::Keychain, &t)))
            })
            .collect()
    }

    /// glab refreshes expired OAuth tokens when it makes an API call. Trigger that,
    /// then read the token again. Used by the GitLab provider after a 401.
    pub async fn refresh_gitlab(&self, host: &str) -> Result<Credential, AuthError> {
        let _ = run("glab", &["api", "--hostname", host, "user"]).await;
        self.get(ProviderKind::GitLab, host).await
    }
}

async fn cli_token(kind: ProviderKind, host: &str, org: Option<&str>) -> Option<String> {
    match kind {
        ProviderKind::GitHub => run("gh", &["auth", "token", "--hostname", host]).await,
        ProviderKind::GitLab => run("glab", &["config", "get", "token", "--host", host]).await,
        ProviderKind::Azure => {
            // Like the az devops extension: ask for a token in the org's tenant when known.
            let tenant = match org {
                Some(o) => azure_tenant::org_tenant(host, o).await,
                None => None,
            };
            let mut args = vec![
                "account",
                "get-access-token",
                "--resource",
                AZURE_DEVOPS_RESOURCE,
                "--query",
                "accessToken",
                "-o",
                "tsv",
            ];
            if let Some(t) = tenant.as_deref() {
                args.extend(["--tenant", t]);
            }
            run("az", &args).await
        }
    }
}

async fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let bin = resolve_cli(cmd).unwrap_or_else(|| PathBuf::from(cmd));
    let out = Command::new(bin).args(args).output().await.ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (!s.is_empty()).then_some(s)
}

/// Directories searched for provider CLIs: `$PATH`, then the usual install dirs.
/// Apps launched from Finder or the Dock get a minimal PATH without Homebrew, so
/// the common locations are added explicitly.
fn cli_search_dirs(path: Option<&std::ffi::OsStr>, home: Option<&std::path::Path>) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = path
        .map(|p| std::env::split_paths(p).collect())
        .unwrap_or_default();
    let mut extra = vec![
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/home/linuxbrew/.linuxbrew/bin"),
    ];
    if let Some(h) = home {
        extra.push(h.join(".local/bin"));
        extra.push(h.join("bin"));
    }
    for d in extra {
        if !dirs.contains(&d) {
            dirs.push(d);
        }
    }
    dirs
}

fn find_in(cmd: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    dirs.iter().map(|d| d.join(cmd)).find(|p| {
        std::fs::metadata(p)
            .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    })
}

/// Absolute path of a CLI such as `gh`, `glab` or `az`, if installed.
pub fn resolve_cli(cmd: &str) -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    find_in(
        cmd,
        &cli_search_dirs(std::env::var_os("PATH").as_deref(), home.as_deref()),
    )
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
    glab_config_paths()
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .find_map(|s| {
            parse_glab_config(&s)
                .into_iter()
                .find(|(h, _)| h.eq_ignore_ascii_case(host))
                .and_then(|(_, t)| t)
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
        assert_eq!(
            p,
            vec![
                ("gitlab.com".into(), Some("abc".into())),
                ("git.example.org".into(), Some("def".into()))
            ]
        );
    }

    #[test]
    fn lookup_order_env_then_org_then_host() {
        let store = TokenStore::memory();
        let org = Account::new(ProviderKind::Azure, "dev.azure.com", Some("o"));
        let host = Account::new(ProviderKind::Azure, "dev.azure.com", None);
        let no_env = |_: &str| None;
        assert!(lookup_static(&org, no_env, &store).is_none());
        store.set(&host, "hostpat").unwrap();
        assert_eq!(
            lookup_static(&org, no_env, &store).unwrap().token,
            "hostpat"
        );
        store.set(&org, "orgpat").unwrap();
        let c = lookup_static(&org, no_env, &store).unwrap();
        assert_eq!(
            (c.token.as_str(), c.origin),
            ("orgpat", TokenOrigin::Keychain)
        );
        assert_eq!(c.scheme, CredentialKind::Basic);
        let env = |v: &str| (v == "AZURE_DEVOPS_EXT_PAT").then(|| "envpat".to_string());
        let c = lookup_static(&org, env, &store).unwrap();
        assert_eq!((c.token.as_str(), c.origin), ("envpat", TokenOrigin::Env));
        // A blank env var does not hide the keychain.
        let blank = |_: &str| Some(" ".to_string());
        assert_eq!(lookup_static(&org, blank, &store).unwrap().token, "orgpat");
    }

    #[test]
    fn azure_devops_resource_id() {
        // The Azure DevOps application id; a wrong one makes `az` fail or mint unusable tokens.
        assert_eq!(
            AZURE_DEVOPS_RESOURCE,
            "499b84ac-1321-427f-aa17-267ca6975798"
        );
    }

    #[test]
    fn env_var_names() {
        assert_eq!(
            env_vars(ProviderKind::GitLab, "gitlab.com")[0],
            "GV_TOKEN_GITLAB_COM"
        );
    }
}

#[cfg(test)]
mod resolver_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn search_dirs_add_homebrew_without_duplicates() {
        let dirs = cli_search_dirs(
            Some(std::ffi::OsStr::new("/usr/bin:/opt/homebrew/bin")),
            Some(std::path::Path::new("/Users/x")),
        );
        assert_eq!(dirs[0], PathBuf::from("/usr/bin"));
        assert_eq!(
            dirs.iter()
                .filter(|d| d.as_path() == std::path::Path::new("/opt/homebrew/bin"))
                .count(),
            1
        );
        assert!(dirs.contains(&PathBuf::from("/usr/local/bin")));
        assert!(dirs.contains(&PathBuf::from("/Users/x/.local/bin")));
        // Finder-style minimal PATH still finds Homebrew.
        assert!(
            cli_search_dirs(Some(std::ffi::OsStr::new("/usr/bin:/bin")), None)
                .contains(&PathBuf::from("/opt/homebrew/bin"))
        );
    }

    #[test]
    fn finds_only_executable_files() {
        let root = std::env::temp_dir().join(format!("gv-resolve-{}", std::process::id()));
        let (a, b) = (root.join("a"), root.join("b"));
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(a.join("gh"), "").unwrap(); // not executable
        std::fs::write(b.join("gh"), "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(b.join("gh"), std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(find_in("gh", &[a.clone(), b.clone()]), Some(b.join("gh")));
        assert_eq!(find_in("glab", &[a, b]), None);
        let _ = std::fs::remove_dir_all(&root);
    }
}
