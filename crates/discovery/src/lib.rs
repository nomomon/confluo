//! Find git repositories on disk and classify their remotes.

use gv_core::{ProviderKind, RepoRef};
use std::path::{Path, PathBuf};

const SKIP_DIRS: &[&str] = &["node_modules", "target", ".venv", "venv", "vendor", "dist", "build", ".cache", "Library"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalRepo {
    pub path: PathBuf,
    /// (remote name, url)
    pub remotes: Vec<(String, String)>,
}

/// Recursively look for git repos under `root`, up to `max_depth` levels below it.
/// Does not descend into a repo once found.
pub fn scan(root: &Path, max_depth: usize) -> Vec<LocalRepo> {
    let mut out = vec![];
    walk(root, 0, max_depth, &mut out);
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

fn walk(dir: &Path, depth: usize, max_depth: usize, out: &mut Vec<LocalRepo>) {
    let git = dir.join(".git");
    if let Some(cfg) = git_config_path(&git) {
        let remotes = std::fs::read_to_string(cfg).map(|s| parse_remotes(&s)).unwrap_or_default();
        out.push(LocalRepo { path: dir.to_path_buf(), remotes });
        return;
    }
    if depth >= max_depth {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let Ok(ft) = e.file_type() else { continue };
        if !ft.is_dir() {
            continue;
        }
        let name = e.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || SKIP_DIRS.contains(&name.as_ref()) {
            continue;
        }
        walk(&e.path(), depth + 1, max_depth, out);
    }
}

/// `.git` is usually a dir, but for worktrees/submodules it is a file `gitdir: <path>`.
fn git_config_path(git: &Path) -> Option<PathBuf> {
    if git.is_dir() {
        let c = git.join("config");
        return c.is_file().then_some(c);
    }
    let s = std::fs::read_to_string(git).ok()?;
    let gd = s.trim().strip_prefix("gitdir:")?.trim();
    let mut gd = PathBuf::from(gd);
    if gd.is_relative() {
        gd = git.parent()?.join(gd);
    }
    // Worktrees keep config in the common dir.
    let common = std::fs::read_to_string(gd.join("commondir")).ok().map(|c| gd.join(c.trim()));
    let c = common.unwrap_or(gd).join("config");
    c.is_file().then_some(c)
}

/// Parse `[remote "name"] url = ...` entries from a git config file.
pub fn parse_remotes(cfg: &str) -> Vec<(String, String)> {
    let mut out = vec![];
    let mut current: Option<String> = None;
    for line in cfg.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            current = l
                .strip_prefix("[remote")
                .and_then(|r| r.trim().strip_prefix('"'))
                .and_then(|r| r.split('"').next())
                .map(str::to_string);
        } else if let Some(name) = &current {
            if let Some((k, v)) = l.split_once('=') {
                if k.trim() == "url" {
                    out.push((name.clone(), v.trim().to_string()));
                }
            }
        }
    }
    out
}

/// Split a git remote URL into (host, path) without classifying it.
/// Supports `https://host/path(.git)`, `ssh://git@host:port/path`, and scp-style `git@host:path`.
pub fn split_remote(url: &str) -> Option<(String, String)> {
    let url = url.trim();
    let (host, path) = if let Some((scheme, rest)) = url.split_once("://") {
        if !matches!(scheme, "https" | "http" | "ssh" | "git" | "git+ssh") {
            return None;
        }
        let (authority, path) = rest.split_once('/')?;
        let host = authority.rsplit('@').next()?;
        let host = host.split(':').next()?;
        (host.to_string(), path.to_string())
    } else {
        // scp-like: [user@]host:path
        let (left, path) = url.split_once(':')?;
        if left.contains('/') {
            return None;
        }
        let host = left.rsplit('@').next()?;
        (host.to_string(), path.to_string())
    };
    let path = path.trim_matches('/').trim_end_matches(".git").to_string();
    if host.is_empty() || path.is_empty() {
        return None;
    }
    Some((host.to_ascii_lowercase(), path))
}

/// Classify a remote URL. `gitlab_hosts` are extra hosts known to be GitLab (e.g. from glab config).
pub fn classify_remote(url: &str, gitlab_hosts: &[String]) -> Option<RepoRef> {
    let (host, path) = split_remote(url)?;
    if host == "github.com" || host == "ssh.github.com" {
        return Some(RepoRef::new(ProviderKind::GitHub, "github.com", path));
    }
    if host == "dev.azure.com" || host == "ssh.dev.azure.com" || host.ends_with(".visualstudio.com") || host == "vs-ssh.visualstudio.com" {
        return azure_ref(&host, &path);
    }
    if host == "gitlab.com" || host == "altssh.gitlab.com" {
        return Some(RepoRef::new(ProviderKind::GitLab, "gitlab.com", path));
    }
    if gitlab_hosts.iter().any(|h| h.eq_ignore_ascii_case(&host)) || host.starts_with("gitlab.") {
        return Some(RepoRef::new(ProviderKind::GitLab, host, path));
    }
    None
}

/// Normalise Azure paths to `org/project/repo` on host `dev.azure.com`.
fn azure_ref(host: &str, path: &str) -> Option<RepoRef> {
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    let (org, project, repo) = if host == "ssh.dev.azure.com" || host == "vs-ssh.visualstudio.com" {
        // v3/org/project/repo
        let p = if parts.first() == Some(&"v3") { &parts[1..] } else { &parts[..] };
        (p.first()?.to_string(), p.get(1)?.to_string(), p.get(2)?.to_string())
    } else if host == "dev.azure.com" {
        // org/project/_git/repo
        let gi = parts.iter().position(|p| *p == "_git")?;
        (parts.first()?.to_string(), parts.get(gi - 1)?.to_string(), parts.get(gi + 1)?.to_string())
    } else {
        // org.visualstudio.com/[DefaultCollection/]project/_git/repo
        let org = host.strip_suffix(".visualstudio.com")?.to_string();
        let gi = parts.iter().position(|p| *p == "_git")?;
        (org, parts.get(gi.checked_sub(1)?)?.to_string(), parts.get(gi + 1)?.to_string())
    };
    Some(RepoRef::new(ProviderKind::Azure, "dev.azure.com", format!("{org}/{project}/{repo}")))
}

/// Scan and classify, preferring the `origin` remote, deduplicated.
pub fn discover_repos(root: &Path, max_depth: usize, gitlab_hosts: &[String]) -> Vec<(PathBuf, RepoRef)> {
    let mut out: Vec<(PathBuf, RepoRef)> = vec![];
    for r in scan(root, max_depth) {
        let mut remotes = r.remotes.clone();
        remotes.sort_by_key(|(n, _)| (n != "origin", n != "upstream"));
        if let Some(rr) = remotes.iter().find_map(|(_, u)| classify_remote(u, gitlab_hosts)) {
            if !out.iter().any(|(_, x)| *x == rr) {
                out.push((r.path, rr));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(u: &str) -> Option<RepoRef> {
        classify_remote(u, &["git.corp.example".into()])
    }

    #[test]
    fn github_urls() {
        let want = RepoRef::new(ProviderKind::GitHub, "github.com", "nomomon/zeron");
        assert_eq!(c("git@github.com:nomomon/zeron.git"), Some(want.clone()));
        assert_eq!(c("https://github.com/nomomon/zeron"), Some(want.clone()));
        assert_eq!(c("https://user:tok@github.com/nomomon/zeron.git/"), Some(want.clone()));
        assert_eq!(c("ssh://git@ssh.github.com:443/nomomon/zeron.git"), Some(want));
    }

    #[test]
    fn gitlab_urls() {
        let want = RepoRef::new(ProviderKind::GitLab, "gitlab.com", "grp/sub/proj");
        assert_eq!(c("git@gitlab.com:grp/sub/proj.git"), Some(want.clone()));
        assert_eq!(c("https://gitlab.com/grp/sub/proj.git"), Some(want));
        assert_eq!(c("ssh://git@git.corp.example:2222/a/b.git"), Some(RepoRef::new(ProviderKind::GitLab, "git.corp.example", "a/b")));
        assert_eq!(c("git@unknown.example:a/b.git"), None);
    }

    #[test]
    fn azure_urls() {
        let want = RepoRef::new(ProviderKind::Azure, "dev.azure.com", "org/proj/repo");
        assert_eq!(c("https://org@dev.azure.com/org/proj/_git/repo"), Some(want.clone()));
        assert_eq!(c("git@ssh.dev.azure.com:v3/org/proj/repo"), Some(want.clone()));
        assert_eq!(c("https://org.visualstudio.com/DefaultCollection/proj/_git/repo"), Some(want.clone()));
        assert_eq!(c("https://org.visualstudio.com/proj/_git/repo"), Some(want));
    }

    #[test]
    fn remotes_from_config() {
        let cfg = "[core]\n\tbare = false\n[remote \"origin\"]\n\turl = git@github.com:a/b.git\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n[branch \"main\"]\n\tremote = origin\n[remote \"up\"]\n\turl = https://gitlab.com/c/d\n";
        assert_eq!(parse_remotes(cfg), vec![("origin".into(), "git@github.com:a/b.git".into()), ("up".into(), "https://gitlab.com/c/d".into())]);
    }

    #[test]
    fn scan_tempdir() {
        let root = std::env::temp_dir().join(format!("gv-disc-{}", std::process::id()));
        let repo = root.join("a/b");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::create_dir_all(root.join("node_modules/x/.git")).unwrap();
        std::fs::write(repo.join(".git/config"), "[remote \"origin\"]\n url = git@github.com:x/y.git\n").unwrap();
        std::fs::write(root.join("node_modules/x/.git/config"), "[remote \"origin\"]\n url = git@github.com:z/z.git\n").unwrap();
        let found = discover_repos(&root, 3, &[]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1.path, "x/y");
        assert!(discover_repos(&root, 1, &[]).is_empty());
        std::fs::remove_dir_all(&root).unwrap();
    }
}
