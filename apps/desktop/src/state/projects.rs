//! Adding project sources and editing repo aliases.

use gpui::Context;
use gv_engine::{RepoRef, Source};
use std::path::PathBuf;

use super::sources::{group_from_url, infer_source, parse_group, SourceKind};
use super::{bridge, err_str};
use crate::app::{GitViewer, Overlay};


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
            SourceKind::Group => match parse_group(&value).or_else(|| group_from_url(&value)) {
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
                self.auto_name = None;
                self.reload_projects();
                self.select_project(Some(p.id), cx);
                self.announce_repo_count(p.id, cx);
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

    /// Resolve the project's repos in the background and toast how many were found.
    fn announce_repo_count(&mut self, project_id: i64, cx: &mut Context<Self>) {
        let engine = self.engine.clone();
        let rx = bridge::spawn(async move { engine.resolve_project_repos(project_id).await.map(|r| r.len()).map_err(err_str) });
        cx.spawn(async move |this, cx| {
            let Ok(r) = rx.await else { return };
            this.update(cx, |v, cx| match r {
                Ok(n) => v.show_toast(format!("Found {n} repo{}", if n == 1 { "" } else { "s" }), cx),
                Err(e) => v.error = Some(e),
            })
            .ok();
        })
        .detach();
    }

    /// Infer the source kind and a project name while the user types or pastes.
    pub(crate) fn on_add_value_changed(&mut self, cx: &mut Context<Self>) {
        let value = self.add_value.read(cx).text();
        let Some((kind, name)) = infer_source(&value) else { return };
        self.add_kind = kind;
        let current = self.add_name.read(cx).text();
        if current.trim().is_empty() || Some(&current) == self.auto_name.as_ref() {
            self.auto_name = Some(name.clone());
            self.add_name.update(cx, |i, cx| i.set_text(name, cx));
        }
        cx.notify();
    }

    pub(crate) fn open_rename(&mut self, id: i64, cx: &mut Context<Self>) {
        self.menu = None;
        let name = self.projects.iter().find(|p| p.id == id).map(|p| p.name.clone()).unwrap_or_default();
        self.rename_input.update(cx, |i, cx| i.set_text(name, cx));
        self.overlay = Overlay::Rename(id);
        cx.notify();
    }

    pub(crate) fn save_rename(&mut self, cx: &mut Context<Self>) {
        let Overlay::Rename(id) = self.overlay else { return };
        let name = self.rename_input.read(cx).text().trim().to_string();
        if name.is_empty() {
            return;
        }
        match self.engine.rename_project(id, &name) {
            Ok(()) => self.overlay = Overlay::None,
            Err(e) => self.add_error = Some(err_str(e)),
        }
        self.reload_projects();
        cx.notify();
    }

    pub(crate) fn delete_project(&mut self, id: i64, cx: &mut Context<Self>) {
        self.menu = None;
        if let Err(e) = self.engine.delete_project(id) {
            self.error = Some(err_str(e));
        }
        self.reload_projects();
        if self.selected.is_none() {
            self.select_project(None, cx);
        }
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
