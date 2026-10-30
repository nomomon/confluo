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

    /// A click on a list row (`count` from the click event).
    pub(crate) fn click_row(
        &mut self,
        c: gv_engine::ChangeRequest,
        count: usize,
        cx: &mut Context<Self>,
    ) {
        let this_open = self.detail.as_ref().is_some_and(|d| d.summary.id == c.id);
        self.row_click_gen += 1;
        match row_click(this_open, !self.preview_hidden, count) {
            RowClick::Show => {
                self.preview_hidden = false;
                if !this_open {
                    self.open_detail(c, cx)
                }
            }
            RowClick::HideLater => {
                let gen = self.row_click_gen;
                cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(super::timing::DOUBLE_CLICK)
                        .await;
                    this.update(cx, |v, cx| {
                        if v.row_click_gen == gen && !v.reviewing {
                            // Selected means previewed: hiding the preview by its row clears the highlight too.
                            v.preview_hidden = true;
                            v.cursor = None;
                            cx.notify()
                        }
                    })
                    .ok();
                })
                .detach();
            }
            RowClick::Review => {
                self.preview_hidden = false;
                if !this_open {
                    self.open_detail(c, cx)
                }
                self.enter_review(cx)
            }
        }
        cx.notify();
    }

    /// Cmd+\ (triage): hide or show the preview pane.
    pub(crate) fn toggle_detail(&mut self, cx: &mut Context<Self>) {
        if self.reviewing {
            return;
        }
        self.preview_hidden = !self.preview_hidden;
        if !self.preview_hidden && self.detail.is_none() {
            if let Some(c) = self
                .nav
                .last_closed
                .clone()
                .or_else(|| self.cursor.as_ref().and_then(|id| self.find_item(id)))
            {
                self.open_detail(c, cx);
            }
        }
        cx.notify();
    }

    /// A PR in the list (or the older section) by id.
    pub(crate) fn find_item(&self, id: &ChangeRequestId) -> Option<ChangeRequest> {
        self.items
            .iter()
            .chain(&self.quiet.stale)
            .find(|c| &c.id == id)
            .cloned()
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
                let cr = self
                    .find_item(&id)
                    .or_else(|| self.engine.store().get_change_request(&id).ok().flatten());
                if let Some(c) = cr {
                    self.open_detail(c, cx);
                    self.enter_review(cx);
                }
            }
        }
        self.navigating = false;
    }

    /// Cmd+1 Inbox, Cmd+2 Starred, Cmd+3..9 the projects in sidebar order.
    pub(crate) fn go_view(&mut self, which: u8, cx: &mut Context<Self>) {
        if self.reviewing {
            self.exit_review(cx);
        }
        match which {
            1 => {
                self.starred_only = false;
                self.select_project(None, cx)
            }
            2 => {
                self.starred_only = true;
                self.select_project(None, cx)
            }
            n => {
                if let Some(id) = self
                    .projects
                    .get(usize::from(n.saturating_sub(3)))
                    .map(|p| p.id)
                {
                    self.starred_only = false;
                    self.select_project(Some(id), cx)
                }
            }
        }
        cx.notify();
    }

    /// Switch between light and dark from whatever is showing now.
    pub(crate) fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        self.settings.theme = if crate::ui::theme::is_light() {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        };
        self.save_settings(cx);
    }
}

/// What a click on a list row does.
#[derive(Debug, PartialEq)]
pub enum RowClick {
    /// Select the row and show its preview.
    Show,
    /// The row's preview is showing: hide it, unless a second click turns this into a double click.
    HideLater,
    /// Double click: open the review.
    Review,
}

pub fn row_click(this_open: bool, preview_visible: bool, count: usize) -> RowClick {
    if count >= 2 {
        RowClick::Review
    } else if this_open && preview_visible {
        RowClick::HideLater
    } else {
        RowClick::Show
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn row_clicks_toggle_the_preview_and_double_click_reviews() {
        use super::{row_click, RowClick};
        // Another row (or nothing open): select and preview it.
        assert_eq!(row_click(false, true, 1), RowClick::Show);
        assert_eq!(row_click(false, false, 1), RowClick::Show);
        // The previewed row again: hide (deferred, a double click cancels it).
        assert_eq!(row_click(true, true, 1), RowClick::HideLater);
        // The selected row with its preview hidden: show it again.
        assert_eq!(row_click(true, false, 1), RowClick::Show);
        // Double click always reviews, never toggles.
        assert_eq!(row_click(true, true, 2), RowClick::Review);
        assert_eq!(row_click(false, true, 2), RowClick::Review);
    }
    use super::*;
    use gv_engine::{ProviderKind, RepoRef};

    fn id(n: u64) -> ChangeRequestId {
        ChangeRequestId {
            repo: RepoRef::new(ProviderKind::GitHub, "github.com", "a/b"),
            number: n,
        }
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
