//! Which popover menu or modal dialog is open, and the Settings page state.

use gpui::Context;
use gv_engine::RepoRef;

use super::err_str;
use crate::app::GitViewer;

/// Which popover menu is open.
#[derive(Clone, Copy, PartialEq)]
pub enum Menu {
    DetailMore,
    /// The detail header's Review dropdown.
    Review,
    /// The kebab menu of a sidebar project.
    Project(i64),
    /// A sidebar project's context menu at the pointer.
    ProjectAt(i64, gpui::Point<gpui::Pixels>),
}

/// Which modal dialog is open.
#[derive(Clone, Copy, PartialEq)]
pub enum Overlay {
    None,
    Settings,
    AddProject,
    Alias,
    Rename(i64),
    /// Confirm deleting a project.
    ConfirmDelete(i64),
    /// Review summary before submitting (Comment, or a verdict with a note).
    ReviewSummary(gv_engine::ReviewAction),
    /// Cmd+K command palette.
    Palette,
    /// Confirm closing the open PR.
    ConfirmClose,
    /// Merge options for the open PR.
    Merge,
    /// Confirm re-running failed checks.
    ConfirmRerun,
}

/// Left navigation of the Settings dialog.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SettingsPage {
    #[default]
    General,
    Appearance,
    Accounts,
    Projects,
    Filters,
    Shortcuts,
}

impl SettingsPage {
    pub const ALL: [SettingsPage; 6] = [Self::General, Self::Appearance, Self::Accounts, Self::Projects, Self::Filters, Self::Shortcuts];

    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Appearance => "Appearance",
            Self::Accounts => "Accounts",
            Self::Projects => "Projects",
            Self::Filters => "Filters",
            Self::Shortcuts => "Shortcuts",
        }
    }

    /// Dev only: `CONFLUO_SETTINGS=appearance` opens Settings on that page at launch.
    pub fn from_env() -> Option<Self> {
        let v = std::env::var("CONFLUO_SETTINGS").ok()?.to_lowercase();
        Self::ALL.into_iter().find(|p| p.label().to_lowercase() == v)
    }
}

/// Which bot list an added pattern goes to.
#[derive(Clone, Copy, PartialEq)]
pub enum BotList {
    Authors,
    Branches,
}

impl GitViewer {
    pub(crate) fn open_settings(&mut self, page: Option<SettingsPage>, cx: &mut Context<Self>) {
        self.overlay = Overlay::Settings;
        if let Some(p) = page {
            self.settings_page = p;
        }
        cx.notify();
    }

    /// Cmd+, opens Settings, or closes it when it is already open.
    pub(crate) fn toggle_settings(&mut self, cx: &mut Context<Self>) {
        if self.overlay == Overlay::Settings {
            self.overlay = Overlay::None;
            cx.notify();
        } else {
            self.open_settings(None, cx);
        }
    }

    /// Add the text of the list's input as a pattern; Enter in the input calls this.
    pub(crate) fn add_bot_entry(&mut self, list: BotList, cx: &mut Context<Self>) {
        let input = match list {
            BotList::Authors => self.bot_authors_input.clone(),
            BotList::Branches => self.bot_branches_input.clone(),
        };
        let text = input.read(cx).text();
        let target = match list {
            BotList::Authors => &mut self.settings.hidden_authors,
            BotList::Branches => &mut self.settings.hidden_branch_prefixes,
        };
        for v in text.split(',').map(str::trim).filter(|v| !v.is_empty()) {
            if !target.iter().any(|x| x == v) {
                target.push(v.to_string());
            }
        }
        input.update(cx, |i, cx| i.set_text("", cx));
        self.save_settings(cx);
        self.reload();
    }

    pub(crate) fn remove_bot_entry(&mut self, list: BotList, value: &str, cx: &mut Context<Self>) {
        match list {
            BotList::Authors => self.settings.hidden_authors.retain(|v| v != value),
            BotList::Branches => self.settings.hidden_branch_prefixes.retain(|v| v != value),
        }
        self.save_settings(cx);
        self.reload();
    }

    /// Hide or show a repo's PRs everywhere.
    pub(crate) fn set_repo_hidden(&mut self, repo: &RepoRef, hidden: bool, cx: &mut Context<Self>) {
        let mut o = self.engine.repo_override(repo).unwrap_or_default();
        o.hidden = hidden;
        if let Err(e) = self.engine.set_repo_override(repo, o) {
            self.error = Some(err_str(e));
        }
        self.reload();
        cx.notify();
    }

    /// Every repo of every project plus those in view, for the alias table.
    pub(crate) fn all_repos(&self) -> Vec<RepoRef> {
        let mut v: Vec<RepoRef> = self.projects.iter().flat_map(|p| self.engine.store().project_repos(p.id).unwrap_or_default()).collect();
        v.extend(self.items.iter().map(|c| c.id.repo.clone()));
        v.sort();
        v.dedup();
        v
    }
}
