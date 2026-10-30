//! Window navigation: sidebar and detail toggles, PR history (back/forward) and views.

use gpui::Context;
use gv_engine::{ChangeRequest, ChangeRequestId};

use crate::app::GitViewer;
use crate::settings::ThemeMode;

/// Navigation state kept beside the list.
#[derive(Default)]
pub struct NavState {
    /// Cmd+B hid the sidebar.
    pub sidebar_hidden: bool,
    /// Places visited, oldest first: a PR in Review mode, or the list (`None`).
    pub history: Vec<Option<ChangeRequestId>>,
    /// Index into `history` of the PR on screen.
    pub pos: usize,
    /// The detail pane closed last, for Cmd+\ to bring back.
    pub last_closed: Option<ChangeRequest>,
    /// Highlighted palette row.
    pub palette_sel: usize,
}

/// Longest PR history kept.
const HISTORY_MAX: usize = 50;

impl NavState {
    /// Record a PR opened directly (not via back/forward): drop the forward part, then push.
    pub fn push(&mut self, id: Option<ChangeRequestId>) {
        if self.history.get(self.pos) == Some(&id) {
            return;
        }
        if !self.history.is_empty() {
            self.history.truncate(self.pos + 1);
        }
        self.history.push(id);
        if self.history.len() > HISTORY_MAX {
            self.history.remove(0);
        }
        self.pos = self.history.len() - 1;
    }

    /// Step back (-1) or forward (+1); returns the place to show (`Some(None)` is the list).
    pub fn step(&mut self, delta: isize) -> Option<Option<ChangeRequestId>> {
        let next = self.pos as isize + delta;
        if next < 0 || next as usize >= self.history.len() {
            return None;
        }
        self.pos = next as usize;
        self.history.get(self.pos).cloned()
    }
}

impl GitViewer {
    pub(crate) fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.nav.sidebar_hidden = !self.nav.sidebar_hidden;
        cx.notify();
    }

    /// Cmd+\ (triage): hide or show the preview pane.
    pub(crate) fn toggle_detail(&mut self, cx: &mut Context<Self>) {
        if self.reviewing {
            return;
        }
        self.preview_hidden = !self.preview_hidden;
        if !self.preview_hidden && self.detail.is_none() {
            if let Some(c) = self.nav.last_closed.clone().or_else(|| self.cursor.as_ref().and_then(|id| self.find_item(id))) {
                self.open_detail(c, cx);
            }
        }
        cx.notify();
    }

    /// A PR in the list (or the older section) by id.
    pub(crate) fn find_item(&self, id: &ChangeRequestId) -> Option<ChangeRequest> {
        self.items.iter().chain(&self.quiet.stale).find(|c| &c.id == id).cloned()
    }

    /// Cmd+[ / Cmd+]: walk the places visited this session (the list and PRs in Review mode).
    pub(crate) fn history_step(&mut self, delta: isize, cx: &mut Context<Self>) {
        let mut nav = std::mem::take(&mut self.nav);
        let target = nav.step(delta);
        self.nav = nav;
        let Some(place) = target else {
            // Nothing earlier: Cmd+[ in Review mode still goes back to the list.
            if delta < 0 && self.reviewing {
                self.exit_review(cx);
            }
            return;
        };
        self.navigating = true;
        match place {
            None => {
                if self.reviewing {
                    self.exit_review(cx)
                }
            }
            Some(id) => {
                let cr = self.find_item(&id).or_else(|| self.engine.store().get_change_request(&id).ok().flatten());
                if let Some(c) = cr {
                    self.open_detail(c, cx);
                    self.enter_review(cx);
                }
            }
        }
        self.navigating = false;
    }

    /// Cmd+1/2/3: Inbox, Starred, first project.
    pub(crate) fn go_view(&mut self, which: u8, cx: &mut Context<Self>) {
        match which {
            1 => {
                self.starred_only = false;
                self.select_project(None, cx)
            }
            2 => {
                self.starred_only = true;
                self.select_project(None, cx)
            }
            _ => {
                if let Some(id) = self.projects.first().map(|p| p.id) {
                    self.starred_only = false;
                    self.select_project(Some(id), cx)
                }
            }
        }
        cx.notify();
    }

    /// Switch between light and dark from whatever is showing now.
    pub(crate) fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        self.settings.theme = if crate::ui::theme::is_light() { ThemeMode::Dark } else { ThemeMode::Light };
        self.save_settings(cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gv_engine::{ProviderKind, RepoRef};

    fn id(n: u64) -> ChangeRequestId {
        ChangeRequestId { repo: RepoRef::new(ProviderKind::GitHub, "github.com", "a/b"), number: n }
    }

    #[test]
    fn history_back_forward_and_branching() {
        let mut n = NavState::default();
        n.push(Some(id(1)));
        n.push(None);
        n.push(Some(id(3)));
        assert_eq!(n.step(-1), Some(None));
        assert_eq!(n.step(-1), Some(Some(id(1))));
        assert_eq!(n.step(-1), None);
        assert_eq!(n.step(1), Some(None));
        n.push(Some(id(4)));
        assert_eq!(n.history, vec![Some(id(1)), None, Some(id(4))]);
        assert_eq!(n.step(1), None);
    }
}
