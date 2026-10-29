//! Parsing pasted project sources: URLs, `provider:host/path` groups and folder paths.

use gv_engine::{ProviderKind, Source};

/// Which kind of source the "Add to project" dialog is building.
#[derive(Clone, Copy, PartialEq)]
pub enum SourceKind {
    Folder,
    Repo,
    Group,
}

/// Parse `provider:host/path` into a group source.
pub fn parse_group(s: &str) -> Option<Source> {
    let (kind, rest) = s.trim().split_once(':')?;
    let provider = ProviderKind::parse(kind)?;
    let (host, path) = rest.trim_start_matches("//").split_once('/')?;
    if host.is_empty() || path.is_empty() {
        return None;
    }
    Some(Source::Group { provider, host: host.to_ascii_lowercase(), path: path.trim_matches('/').to_string() })
}

/// Guess the source kind and a project name from a pasted URL, `provider:host/path` or path.
pub fn infer_source(value: &str) -> Option<(SourceKind, String)> {
    let v = value.trim().trim_end_matches('/');
    if v.is_empty() {
        return None;
    }
    let last = |s: &str| s.trim_end_matches(".git").rsplit(['/', ':']).next().unwrap_or(s).to_string();
    if v.starts_with('/') || v.starts_with('~') || v.starts_with('.') {
        return Some((SourceKind::Folder, last(v)));
    }
    if parse_group(v).is_some() && !v.contains("://") {
        return Some((SourceKind::Group, last(v)));
    }
    let (_, segs) = url_parts(v)?;
    let kind = if segs.first() == Some(&"groups") || segs.first() == Some(&"orgs") || segs.len() == 1 { SourceKind::Group } else { SourceKind::Repo };
    Some((kind, last(segs.last()?)))
}

/// Host and path segments of an http(s) or `git@host:path` URL, without web suffixes.
fn url_parts(v: &str) -> Option<(String, Vec<&str>)> {
    let rest = v.strip_prefix("https://").or_else(|| v.strip_prefix("http://")).or_else(|| v.strip_prefix("git@"))?;
    let (host, path) = rest.split_once(['/', ':'])?;
    let path = path.split("/-/").next().unwrap_or(path).split(['?', '#']).next().unwrap_or(path);
    let segs: Vec<&str> = path.trim_matches('/').trim_end_matches(".git").split('/').filter(|s| !s.is_empty()).collect();
    (!segs.is_empty()).then(|| (host.to_ascii_lowercase(), segs))
}

/// A group source from a web URL such as `https://gitlab.com/groups/acme` or `https://github.com/acme`.
pub fn group_from_url(v: &str) -> Option<Source> {
    let (host, segs) = url_parts(v.trim())?;
    let segs: Vec<&str> = segs.into_iter().skip_while(|s| *s == "groups" || *s == "orgs").collect();
    let provider = if host.contains("github") { ProviderKind::GitHub } else { ProviderKind::GitLab };
    (!segs.is_empty()).then(|| Source::Group { provider, host, path: segs.join("/") })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_sources() {
        let k = |s: &str| infer_source(s).map(|(k, n)| (k == SourceKind::Repo, k == SourceKind::Group, n));
        assert_eq!(k("https://gitlab.com/acme/web/app.git"), Some((true, false, "app".into())));
        assert_eq!(k("https://gitlab.com/groups/acme/"), Some((false, true, "acme".into())));
        assert_eq!(k("https://github.com/acme"), Some((false, true, "acme".into())));
        assert_eq!(k("git@gitlab.com:acme/api.git"), Some((true, false, "api".into())));
        assert_eq!(k("~/code/work"), Some((false, false, "work".into())));
        assert_eq!(group_from_url("https://gitlab.com/groups/a/b"), Some(Source::Group { provider: ProviderKind::GitLab, host: "gitlab.com".into(), path: "a/b".into() }));
    }

    #[test]
    fn parses_group() {
        let g = parse_group("gitlab:gitlab.com/a/b").unwrap();
        assert_eq!(g, Source::Group { provider: ProviderKind::GitLab, host: "gitlab.com".into(), path: "a/b".into() });
        assert!(parse_group("nope").is_none());
    }
}
