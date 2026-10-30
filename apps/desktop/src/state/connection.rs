//! Which provider hosts are signed in, and readable refresh errors.

use gpui::Context;
use gv_engine::ProviderKind;

use super::{bridge, timing};
use crate::app::GitViewer;

/// A provider host and the signed-in username, if any.
#[derive(Clone)]
pub struct HostStatus {
    pub kind: ProviderKind,
    pub host: String,
    pub user: Option<String>,
}

/// CLI command that signs in to a provider.
pub fn auth_command(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::GitHub => "gh auth login",
        ProviderKind::GitLab => "glab auth login",
        ProviderKind::Azure => "az login",
    }
}

/// What to tell a signed-out user, e.g. "Run `gh auth login` in a terminal".
pub fn sign_in_hint(kind: ProviderKind) -> String {
    match kind {
        // az sessions expire (password changes revoke them), so this is usually a re-login.
        ProviderKind::Azure => format!("Sign in again with `{}`", auth_command(kind)),
        _ => format!("Run `{}` in a terminal", auth_command(kind)),
    }
}

/// Avatar URL for a signed-in GitHub user; other hosts show the initial.
pub fn host_avatar_url(kind: ProviderKind, host: &str, user: &str) -> Option<String> {
    (kind == ProviderKind::GitHub && host == "github.com").then(|| format!("https://github.com/{user}.png?size=64"))
}

impl GitViewer {
    /// Ask every known host who we are, in the background.
    pub(crate) fn check_hosts(&mut self, cx: &mut Context<Self>) {
        let engine = self.engine.clone();
        // Show what we already know at once; the network check below confirms it.
        let known = engine.known_hosts();
        if self.hosts.is_none() && known.iter().all(|(k, h)| engine.me(*k, h).is_some()) {
            self.hosts = Some(known.iter().map(|(k, h)| HostStatus { kind: *k, host: h.clone(), user: engine.me(*k, h) }).collect());
        }
        let rx = bridge::spawn(async move {
            let jobs = known.into_iter().map(|(kind, host)| {
                let engine = engine.clone();
                async move {
                    // A busy host (a refresh in flight, a rate limit pause) must not leave Accounts loading forever.
                    let user = match tokio::time::timeout(timing::HOST_CHECK_TIMEOUT, engine.whoami(kind, &host)).await {
                        Ok(r) => r.ok().map(|u| u.username),
                        Err(_) => engine.me(kind, &host),
                    };
                    HostStatus { kind, host, user }
                }
            });
            futures::future::join_all(jobs).await
        });
        cx.spawn(async move |this, cx| {
            let v = rx.await.unwrap_or_default();
            this.update(cx, |t, cx| {
                t.hosts = Some(v);
                cx.notify()
            })
            .ok();
        })
        .detach();
    }

    /// Host as shown in Accounts: Azure DevOps lists its organizations (`dev.azure.com/org`).
    pub(crate) fn host_label(&self, h: &HostStatus) -> String {
        if h.kind != ProviderKind::Azure {
            return h.host.clone();
        }
        let orgs = self.engine.azure_orgs();
        if orgs.is_empty() {
            h.host.clone()
        } else {
            orgs.iter().map(|o| format!("{}/{o}", h.host)).collect::<Vec<_>>().join(", ")
        }
    }

    pub(crate) fn connected_hosts(&self) -> Vec<&HostStatus> {
        self.hosts.iter().flatten().filter(|h| h.user.is_some()).collect()
    }

    fn host_kind(&self, label: &str) -> ProviderKind {
        let host = label.split('/').next().unwrap_or(label);
        self.hosts.iter().flatten().find(|h| h.host == host).map(|h| h.kind).unwrap_or(if host.contains("github") {
            ProviderKind::GitHub
        } else if host == "dev.azure.com" || host.ends_with(".visualstudio.com") {
            ProviderKind::Azure
        } else {
            ProviderKind::GitLab
        })
    }

    /// One line for the error banner, e.g. "gitlab.com: 401, run `glab auth login`".
    pub(crate) fn summarize_errors(&self, errors: &[(String, String)]) -> Option<String> {
        let (label, err) = errors.first()?;
        let kind = self.host_kind(label);
        let mut line = format!("{label}: {}", short_error(err));
        if kind == ProviderKind::Azure && is_auth_error(err) {
            // "dev.azure.com: sign in again with `az login`"
            let host = label.split('/').next().unwrap_or(label);
            line = format!("{host}: sign in again with `{}`", auth_command(kind));
        } else if is_auth_error(err) {
            line.push_str(&format!(", run `{}`", auth_command(self.host_kind(label))));
        }
        if errors.len() > 1 {
            line.push_str(&format!(" (+{} more)", errors.len() - 1));
        }
        Some(line)
    }
}

fn is_auth_error(e: &str) -> bool {
    let e = e.to_ascii_lowercase();
    ["401", "403", "unauthorized", "forbidden", "credential", "token", "not logged", "auth"].iter().any(|k| e.contains(k))
}

/// First line of an error, trimmed to a status code when one is present.
pub fn short_error(e: &str) -> String {
    let first = e.lines().next().unwrap_or(e).trim();
    for code in ["401", "403", "404", "429", "500", "502", "503"] {
        if first.contains(code) {
            return code.to_string();
        }
    }
    let mut s: String = first.chars().take(80).collect();
    if first.chars().count() > 80 {
        s.push_str("...");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortens_errors() {
        assert_eq!(short_error("HTTP status client error (401 Unauthorized) for url"), "401");
        assert_eq!(short_error("dns failed\nmore"), "dns failed");
        assert!(is_auth_error("no credentials for gitlab.com"));
        assert_eq!(sign_in_hint(ProviderKind::Azure), "Sign in again with `az login`");
    }
}
