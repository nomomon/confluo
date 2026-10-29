//! Adding project sources and editing repo aliases.

use gpui::Context;
use gv_engine::{ProviderKind, RepoRef, Source};
use std::path::PathBuf;

use super::err_str;
use crate::app::{GitViewer, Overlay};

/// Which kind of source the "Add to project" dialog is building.
#[derive(Clone, Copy, PartialEq)]
pub enum SourceKind {
    Folder,
    Repo,
    Group,
}

impl GitViewer {
    pub(crate) fn submit_add_project(&mut self, cx: &mut Context<Self>) {
        let name = self.add_name.read(cx).text().trim().to_string();
        let value = self.add_value.read(cx).text().trim().to_string();
        if name.is_empty() || value.is_empty() {
            self.add_error = Some("Project name and value are required".into());
            cx.notify();
            return;
        }
        let source = match self.add_kind {
            SourceKind::Folder => {
                let p = if let Some(rest) = value.strip_prefix("~/") {
                    PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(rest)
                } else {
                    PathBuf::from(&value)
                };
                Source::Folder { path: p, depth: 3 }
            }
            SourceKind::Repo => Source::Repo { url: value },
            SourceKind::Group => match parse_group(&value) {
                Some(s) => s,
                None => {
                    self.add_error = Some("Expected provider:host/path, e.g. gitlab:gitlab.com/my-group".into());
                    cx.notify();
                    return;
                }
            },
        };
        match self.engine.add_source(&name, source) {
            Ok(p) => {
                self.add_error = None;
                self.overlay = Overlay::None;
                self.add_value.update(cx, |i, cx| i.set_text("", cx));
                self.reload_projects();
                self.select_project(Some(p.id), cx);
            }
            Err(e) => {
                self.add_error = Some(err_str(e));
                cx.notify();
            }
        }
    }

    pub(crate) fn open_add_project(&mut self, cx: &mut Context<Self>) {
        self.overlay = Overlay::AddProject;
        self.add_error = None;
        let n = self.selected.map(|_| self.project_name()).unwrap_or_default();
        self.add_name.update(cx, |i, cx| i.set_text(n, cx));
        cx.notify();
    }

    pub(crate) fn open_alias(&mut self, repo: Option<RepoRef>, cx: &mut Context<Self>) {
        self.overlay = Overlay::Alias;
        self.set_alias_repo(repo, cx);
    }

    pub(crate) fn set_alias_repo(&mut self, repo: Option<RepoRef>, cx: &mut Context<Self>) {
        let cur = repo.as_ref().and_then(|r| self.engine.repo_override(r).ok()).and_then(|o| o.alias).unwrap_or_default();
        self.alias_input.update(cx, |i, cx| i.set_text(cur, cx));
        self.alias_repo = repo;
        cx.notify();
    }

    pub(crate) fn alias_candidates(&self) -> Vec<RepoRef> {
        let mut v: Vec<RepoRef> = match self.selected {
            Some(id) => self.engine.store().project_repos(id).unwrap_or_default(),
            None => vec![],
        };
        v.extend(self.items.iter().map(|c| c.id.repo.clone()));
        v.sort();
        v.dedup();
        v
    }

    pub(crate) fn save_alias(&mut self, cx: &mut Context<Self>) {
        let Some(repo) = self.alias_repo.clone() else { return };
        let alias = self.alias_input.read(cx).text().trim().to_string();
        let mut o = self.engine.repo_override(&repo).unwrap_or_default();
        o.alias = (!alias.is_empty()).then_some(alias);
        if let Err(e) = self.engine.set_repo_override(&repo, o) {
            self.error = Some(err_str(e));
        }
        self.reload();
        cx.notify();
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_group() {
        let g = parse_group("gitlab:gitlab.com/a/b").unwrap();
        assert_eq!(g, Source::Group { provider: ProviderKind::GitLab, host: "gitlab.com".into(), path: "a/b".into() });
        assert!(parse_group("nope").is_none());
    }
}
