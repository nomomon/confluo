//! Which provider hosts are signed in, and readable refresh errors.

use gpui::Context;
use gv_engine::ProviderKind;

use super::bridge;
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

/// Avatar URL for a signed-in GitHub user; other hosts show the initial.
pub fn host_avatar_url(kind: ProviderKind, host: &str, user: &str) -> Option<String> {
    (kind == ProviderKind::GitHub && host == "github.com").then(|| format!("https://github.com/{user}.png?size=64"))
}

impl GitViewer {
    /// Ask every known host who we are, in the background.
    pub(crate) fn check_hosts(&mut self, cx: &mut Context<Self>) {
        let engine = self.engine.clone();
        let rx = bridge::spawn(async move {
            let jobs = engine.known_hosts().into_iter().map(|(kind, host)| {
                let engine = engine.clone();
                async move {
                    let user = engine.whoami(kind, &host).await.ok().map(|u| u.username);
                    HostStatus { kind, host, user }
                }
            });
            futures::future::join_all(jobs).await
        });
        cx.spawn(async move |this, cx| {
            if let Ok(v) = rx.await {
                this.update(cx, |t, cx| {
                    t.hosts = Some(v);
                    cx.notify()
                })
                .ok();
            }
        })
        .detach();
    }

    pub(crate) fn connected_hosts(&self) -> Vec<&HostStatus> {
        self.hosts.iter().flatten().filter(|h| h.user.is_some()).collect()
    }

    fn host_kind(&self, label: &str) -> ProviderKind {
        let host = label.split('/').next().unwrap_or(label);
        self.hosts.iter().flatten().find(|h| h.host == host).map(|h| h.kind).unwrap_or(if host.contains("github") {
            ProviderKind::GitHub
        } else {
            ProviderKind::GitLab
        })
    }

    /// One line for the error banner, e.g. "gitlab.com: 401, run `glab auth login`".
    pub(crate) fn summarize_errors(&self, errors: &[(String, String)]) -> Option<String> {
        let (label, err) = errors.first()?;
        let mut line = format!("{label}: {}", short_error(err));
        if is_auth_error(err) {
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
    }
}
