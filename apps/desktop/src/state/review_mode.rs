//! Triage (list with a slim preview) and Review mode (one PR takes the whole window).
//!
//! Review mode remembers per PR, for the session, where the diff was scrolled and which files were
//! collapsed; viewed files persist through the engine (GitHub API, local store elsewhere).

use std::collections::{HashMap, HashSet};

use gpui::{px, Context, ListOffset, Pixels};
use gv_engine::ChangeRequestId;

use super::{bridge, err_str, DetailTab};
use crate::app::GitViewer;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum RightTab {
    #[default]
    Overview,
    Conversation,
}

/// A resizable pane being dragged.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Tree,
    Right,
}

/// Diff position and collapsed files of a PR left in this session.
#[derive(Clone)]
struct Memory {
    offset: ListOffset,
    collapsed: HashSet<usize>,
}

pub struct ReviewUi {
    pub tree_open: bool,
    pub tree_w: Pixels,
    pub right_open: bool,
    /// Right panel share of the content width (see `layout::right_pane_w`).
    pub right_ratio: f32,
    pub right_tab: RightTab,
    pub hide_viewed: bool,
    /// Paths I marked viewed in the open PR.
    pub viewed: HashSet<String>,
    /// Folders folded in the file tree.
    pub folded: HashSet<String>,
    pub resizing: Option<Pane>,
    /// Cmd+P file picker is open, and its highlighted row.
    pub quick_open: bool,
    pub quick_sel: usize,
    /// The checks popover under the CI icon.
    pub checks_open: bool,
    /// Open reaction picker: a comment id, or "pr".
    pub reaction_picker: Option<String>,
    memory: HashMap<ChangeRequestId, Memory>,
    /// Restore the remembered position once files arrive.
    restore: Option<Memory>,
}

impl ReviewUi {
    /// Pane sizes restored from settings.
    pub fn with_sizes(tree_w: Pixels, right_ratio: f32) -> Self {
        Self { tree_w, right_ratio, ..Default::default() }
    }
}

impl Default for ReviewUi {
    fn default() -> Self {
        Self {
            tree_open: true,
            tree_w: px(260.),
            right_open: true,
            right_ratio: 1. / 3.,
            right_tab: RightTab::Overview,
            hide_viewed: false,
            viewed: HashSet::new(),
            folded: HashSet::new(),
            resizing: None,
            quick_open: false,
            quick_sel: 0,
            checks_open: false,
            reaction_picker: None,
            memory: HashMap::new(),
            restore: None,
        }
    }
}

impl ReviewUi {
    /// Drop the remembered diff position of a PR, so it opens at the top.
    pub fn forget(&mut self, id: &ChangeRequestId) {
        self.memory.remove(id);
    }
}

/// Next file at or after `from` (wrapping) that is not viewed.
pub fn next_unviewed(paths: &[&str], viewed: &HashSet<String>, from: usize) -> Option<usize> {
    let n = paths.len();
    (0..n).map(|k| (from + k) % n).find(|i| !viewed.contains(paths[*i]))
}

impl GitViewer {
    /// Enter Review mode on the open (previewed) PR.
    pub(crate) fn enter_review(&mut self, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else { return };
        let id = d.summary.id.clone();
        if !self.navigating {
            self.nav.push(Some(id.clone()));
        }
        self.reviewing = true;
        self.review.viewed.clear();
        self.review.folded.clear();
        self.review.quick_open = false;
        self.review.restore = self.review.memory.get(&id).cloned();
        d.code.split = self.settings.diff_split;
        self.menu = None;
        self.set_detail_tab(DetailTab::Code, cx);
        self.after_files(cx);
        cx.notify();
    }

    /// Back to the list, keeping the selection and the list scroll.
    pub(crate) fn exit_review(&mut self, cx: &mut Context<Self>) {
        self.remember_position();
        self.reviewing = false;
        self.review.quick_open = false;
        self.review.checks_open = false;
        if !self.navigating {
            self.nav.push(None);
        }
        if let Some(d) = self.detail.as_mut() {
            d.tab = DetailTab::Summary;
            self.cursor = Some(d.summary.id.clone());
            self.reveal_cursor.set(true);
        }
        self.refocus = true;
        cx.notify();
    }

    pub(crate) fn remember_position(&mut self) {
        let Some(d) = self.detail.as_ref().filter(|_| self.reviewing) else { return };
        let m = Memory { offset: d.code.list.logical_scroll_top(), collapsed: d.code.collapsed.clone() };
        self.review.memory.insert(d.summary.id.clone(), m);
    }

    /// Files arrived (or were cached): load viewed marks, collapse viewed files, restore the position.
    pub(crate) fn after_files(&mut self, cx: &mut Context<Self>) {
        if !self.reviewing {
            return;
        }
        let Some(d) = self.detail.as_mut() else { return };
        let Some(files) = d.code.files.clone() else { return };
        self.review.viewed = self.engine.viewed_files(&d.summary.id, &files);
        match self.review.restore.take() {
            Some(m) => {
                d.code.collapsed = m.collapsed;
                d.code.rebuild();
                d.code.list.scroll_to(m.offset);
            }
            None => {
                d.code.collapsed = files.iter().enumerate().filter(|(_, f)| self.review.viewed.contains(f.path())).map(|(i, _)| i).collect();
                d.code.rebuild();
            }
        }
        cx.notify();
    }

    /// Mark the file viewed (collapse it and jump to the next unviewed) or unviewed (expand it).
    pub(crate) fn toggle_viewed(&mut self, fi: usize, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else { return };
        let Some(files) = d.code.files.clone() else { return };
        let Some(f) = files.get(fi).cloned() else { return };
        let path = f.path().to_string();
        let viewed = !self.review.viewed.contains(&path);
        if viewed {
            self.review.viewed.insert(path.clone());
            d.code.collapsed.insert(fi);
            let paths: Vec<&str> = files.iter().map(|f| f.path()).collect();
            let next = next_unviewed(&paths, &self.review.viewed, fi + 1);
            d.code.rebuild();
            d.code.goto(next.unwrap_or(fi));
        } else {
            self.review.viewed.remove(&path);
            d.code.collapsed.remove(&fi);
            d.code.rebuild();
            d.code.goto(fi);
        }
        let (engine, id) = (self.engine.clone(), d.summary.id.clone());
        let rx = bridge::spawn(async move { engine.set_file_viewed(&id, &f, viewed).await.map_err(err_str) });
        cx.spawn(async move |this, cx| {
            if let Ok(Err(e)) = rx.await {
                this.update(cx, |v, cx| {
                    if viewed {
                        v.review.viewed.remove(&path);
                    } else {
                        v.review.viewed.insert(path.clone());
                    }
                    v.show_error_toast("Could not update viewed state", e, cx)
                })
                .ok();
            }
        })
        .detach();
        cx.notify();
    }

    /// Unified or side-by-side; saved in settings.
    pub(crate) fn set_split(&mut self, split: bool, cx: &mut Context<Self>) {
        self.settings.diff_split = split;
        if let Some(d) = self.detail.as_mut() {
            let cur = d.code.current_file();
            d.code.split = split;
            d.code.rebuild();
            d.code.goto(cur);
        }
        self.save_settings(cx);
    }

    /// Cmd+P: open the file picker, focused.
    pub(crate) fn open_quick_file(&mut self, cx: &mut Context<Self>) {
        if !self.reviewing {
            return;
        }
        self.review.quick_open = true;
        self.review.quick_sel = 0;
        self.quick_input.update(cx, |i, cx| i.set_text("", cx));
        self.refocus_quick = true;
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_unviewed_wraps_and_skips() {
        let paths = ["a", "b", "c"];
        let mut v = HashSet::from(["b".to_string()]);
        assert_eq!(next_unviewed(&paths, &v, 1), Some(2));
        assert_eq!(next_unviewed(&paths, &v, 3), Some(0));
        v.extend(["a".to_string(), "c".to_string()]);
        assert_eq!(next_unviewed(&paths, &v, 0), None);
    }
}
