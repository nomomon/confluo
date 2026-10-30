//! Which popover menu or modal dialog is open, and the Settings page state.

use gpui::Context;
use gv_engine::RepoRef;

use super::err_str;
use crate::app::GitViewer;

/// The one transient floating layer that is open. Opening any of them closes the others (and the
/// repository menu and people picker, kept apart for their own state); hover cards and tooltips stay
/// hidden while one is open (`ui::layer`); a press outside it or Esc closes it.
#[derive(Clone, PartialEq, Debug)]
pub enum Layer {
    /// The detail's More menu.
    DetailMore,
    /// The detail header's Review dropdown.
    Review,
    /// The kebab menu of a sidebar project.
    Project(i64),
    /// A sidebar project's context menu at the pointer.
    ProjectAt(i64, gpui::Point<gpui::Pixels>),
    /// The checks popover of the CI icon.
    Checks,
    /// The reaction picker of the PR (`""`) or of a comment (its id).
    Reactions(String),
    /// The More menu of a comment (its id).
    CommentMenu(String),
    /// A comment's context menu at the pointer.
    CommentMenuAt(String, gpui::Point<gpui::Pixels>),
    /// A file tree item's context menu at the pointer (file index).
    FileAt(usize, gpui::Point<gpui::Pixels>),
    /// The diff lines' context menu at the pointer (acts on the selection).
    LinesAt(gpui::Point<gpui::Pixels>),
    /// A list row's context menu at the pointer.
    Row(gv_engine::ChangeRequestId, gpui::Point<gpui::Pixels>),
}

/// Which modal dialog is open.
#[derive(Clone, Copy, PartialEq)]
pub enum Overlay {
    None,
    Settings,
    AddProject,
    Rename(i64),
    /// Confirm deleting a project.
    ConfirmDelete(i64),
    /// Review summary before submitting (Comment, or a verdict with a note).
    ReviewSummary(gv_engine::ReviewAction),
    /// Cmd+K command palette.
    Palette,
    /// Confirm closing the open PR.
    ConfirmClose,
    /// Confirm deleting a comment (`EditUi::delete_comment`).
    ConfirmDeleteComment,
    /// Merge options for the open PR.
    Merge,
    /// Confirm re-running failed checks.
    ConfirmRerun,
    /// The `?` keyboard shortcut cheat sheet.
    Shortcuts,
}

/// Left navigation of the Settings dialog.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SettingsPage {
    #[default]
    General,
    Appearance,
    Accounts,
    Projects,
    Repositories,
    Filters,
    Shortcuts,
}

impl SettingsPage {
    pub const ALL: [SettingsPage; 7] = [
        Self::General,
        Self::Appearance,
        Self::Accounts,
        Self::Projects,
        Self::Repositories,
        Self::Filters,
        Self::Shortcuts,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Appearance => "Appearance",
            Self::Accounts => "Accounts",
            Self::Projects => "Projects",
            Self::Repositories => "Repositories",
            Self::Filters => "Filters",
            Self::Shortcuts => "Shortcuts",
        }
    }

    /// Dev only: `CONFLUO_SETTINGS=appearance` opens Settings on that page at launch.
    pub fn from_env() -> Option<Self> {
        let v = std::env::var("CONFLUO_SETTINGS").ok()?.to_lowercase();
        Self::ALL
            .into_iter()
            .find(|p| p.label().to_lowercase() == v)
    }
}

/// Which bot list an added pattern goes to.
#[derive(Clone, Copy, PartialEq)]
pub enum BotList {
    Authors,
    Branches,
}

impl GitViewer {
    /// Open `layer`, closing whatever floating layer was open (menus, popovers, pickers).
    pub(crate) fn open_layer(&mut self, layer: Layer, cx: &mut Context<Self>) {
        self.close_layers(cx);
        self.layer = Some(layer);
        cx.notify();
    }

    /// A press outside the open layer: close it (its trigger's click then keeps it closed).
    pub(crate) fn dismiss_layer(&mut self, cx: &mut Context<Self>) {
        if let Some(l) = self.layer.take() {
            self.layer_dismissed = Some((l, std::time::Instant::now()));
            cx.notify();
        }
    }

    /// Open `layer`, or close it when it is the one open (trigger buttons).
    pub(crate) fn toggle_layer(&mut self, layer: Layer, cx: &mut Context<Self>) {
        let just_closed = self
            .layer_dismissed
            .take()
            .is_some_and(|(l, at)| l == layer && at.elapsed() < super::timing::LAYER_REOPEN_GUARD);
        if just_closed {
            return;
        }
        if self.layer.as_ref() == Some(&layer) {
            self.layer = None;
            cx.notify();
        } else {
            self.open_layer(layer, cx)
        }
    }

    /// Close every floating layer. True when something was open.
    pub(crate) fn close_layers(&mut self, cx: &mut Context<Self>) -> bool {
        let people = self
            .detail
            .as_mut()
            .is_some_and(|d| d.ui.people.take().is_some());
        let open = self.layer.take().is_some() || self.repo_menu.take().is_some() || people;
        if open {
            cx.notify();
        }
        open
    }

    /// A menu, popover or picker is open (hover cards and shortcut hints stay hidden meanwhile).
    pub(crate) fn layer_open(&self) -> bool {
        self.layer.is_some()
            || self.repo_menu.is_some()
            || self.detail.as_ref().is_some_and(|d| d.ui.people.is_some())
    }

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

    /// Every repo of every project plus those in view, for Settings > Repositories.
    pub(crate) fn all_repos(&self) -> Vec<RepoRef> {
        let mut v: Vec<RepoRef> = self
            .projects
            .iter()
            .flat_map(|p| self.engine.store().project_repos(p.id).unwrap_or_default())
            .collect();
        v.extend(self.items.iter().map(|c| c.id.repo.clone()));
        v.sort();
        v.dedup();
        v
    }
}
