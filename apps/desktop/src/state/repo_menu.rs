//! Right-click a repository name: rename how it appears (alias) or hide it.

use gpui::{Context, Pixels, Point};
use gv_engine::RepoRef;

use super::err_str;
use crate::app::GitViewer;

#[derive(Clone)]
pub struct RepoMenu {
    pub repo: RepoRef,
    pub pos: Point<Pixels>,
    /// Showing the rename field instead of the two menu items.
    pub renaming: bool,
}

impl GitViewer {
    pub(crate) fn open_repo_menu(&mut self, repo: RepoRef, pos: Point<Pixels>, cx: &mut Context<Self>) {
        self.close_layers(cx);
        self.repo_menu = Some(RepoMenu { repo, pos, renaming: false });
        cx.notify();
    }

    /// Swap the menu for a field prefilled with the shown name; the placeholder is the full path.
    pub(crate) fn start_repo_rename(&mut self, cx: &mut Context<Self>) {
        let Some(m) = self.repo_menu.as_mut() else { return };
        m.renaming = true;
        let repo = m.repo.clone();
        self.set_alias_repo(Some(repo), cx);
        self.refocus_alias = true;
        cx.notify();
    }

    /// Enter in the rename field: empty or the default name clears the alias.
    pub(crate) fn save_repo_rename(&mut self, cx: &mut Context<Self>) {
        self.repo_menu = None;
        self.save_alias(cx)
    }

    pub(crate) fn set_alias(&mut self, repo: &RepoRef, alias: Option<String>, cx: &mut Context<Self>) {
        let mut o = self.engine.repo_override(repo).unwrap_or_default();
        o.alias = alias;
        if let Err(e) = self.engine.set_repo_override(repo, o) {
            self.error = Some(err_str(e));
        }
        self.repo_menu = None;
        self.refocus = true;
        self.reload();
        cx.notify();
    }
}
