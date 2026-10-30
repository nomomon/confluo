//! Code tab state: lazily loaded changed files, collapse state and the virtual row list.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use gpui::{px, ListAlignment, ListOffset, ListState, Pixels};
use gv_engine::ChangedFile;

use super::code_rows::{build_rows, file_rows, InlineRows, Row};
use super::file_view::FileView;
use super::inline::InlineState;
use super::line_select::{DiffSel, PendingSelect};

pub struct CodeState {
    pub files: Option<Arc<Vec<ChangedFile>>>,
    /// What the diff is filtered to (a commit, or changes since my last review); None shows all.
    pub commit: Option<super::commit_filter::DiffFilter>,
    pub loading: bool,
    /// Set `LOADING_DELAY` after loading starts, so quick loads show no skeleton.
    pub show_skeleton: bool,
    pub error: Option<String>,
    pub collapsed: HashSet<usize>,
    /// Large files the user expanded with "Show diff".
    pub shown_large: HashSet<usize>,
    pub rows: Vec<Row>,
    /// Header row index of each file.
    pub starts: Vec<usize>,
    pub list: ListState,
    /// Horizontal scroll per file card.
    pub scroll_x: HashMap<usize, Pixels>,
    pub viewer: Option<FileView>,
    /// Inline review threads, the open composer and reply/edit state.
    pub inline: InlineState,
    /// Selected diff lines, independent of the comment composer.
    pub sel: Option<DiffSel>,
    /// A mouse drag is extending `sel`.
    pub dragging: bool,
    /// The drag started on the gutter "+": releasing it opens the composer on the range.
    pub plus_drag: bool,
    /// The selection last moved by keyboard; its toolbar appears without animation.
    pub sel_kbd: bool,
    /// Dev only (`CONFLUO_SELECT`): lines to select once files load.
    pub pending_select: Option<PendingSelect>,
    /// Side-by-side rows.
    pub split: bool,
}

impl CodeState {
    pub fn new() -> Self {
        Self {
            files: None,
            commit: None,
            loading: false,
            show_skeleton: false,
            error: None,
            collapsed: HashSet::new(),
            shown_large: HashSet::new(),
            rows: vec![],
            starts: vec![],
            list: ListState::new(0, ListAlignment::Top, px(800.)),
            scroll_x: HashMap::new(),
            viewer: None,
            inline: InlineState::default(),
            sel: None,
            dragging: false,
            plus_drag: false,
            sel_kbd: false,
            pending_select: None,
            split: false,
        }
    }

    pub(crate) fn inline_rows(&self) -> InlineRows<'_> {
        InlineRows {
            index: Some(&self.inline.index),
            draft: self.inline.draft.as_ref().map(|d| (d.fi, d.side, d.line)),
            split: self.split,
        }
    }

    /// Recompute rows and splice only the changed middle, so the scroll position holds.
    pub fn sync_rows(&mut self) {
        let Some(files) = &self.files else { return };
        let (rows, starts) = build_rows(
            files,
            &self.collapsed,
            &self.shown_large,
            self.inline_rows(),
        );
        let pre = self
            .rows
            .iter()
            .zip(&rows)
            .take_while(|(a, b)| a == b)
            .count();
        let max_suf = self.rows.len().min(rows.len()) - pre;
        let suf = self
            .rows
            .iter()
            .rev()
            .zip(rows.iter().rev())
            .take(max_suf)
            .take_while(|(a, b)| a == b)
            .count();
        self.list
            .splice(pre..self.rows.len() - suf, rows.len() - suf - pre);
        self.rows = rows;
        self.starts = starts;
    }

    /// Stores the files in `tree_cmp` order, so file indices match the tree, the diff, n/p and "File i of N".
    pub(crate) fn set_files(&mut self, files: Arc<Vec<ChangedFile>>) {
        let sorted = files
            .windows(2)
            .all(|w| super::file_tree::tree_cmp(w[0].path(), w[1].path()).is_le());
        let files = if sorted {
            files
        } else {
            let mut v = (*files).clone();
            v.sort_by(|a, b| super::file_tree::tree_cmp(a.path(), b.path()));
            Arc::new(v)
        };
        let unchanged = self.files.as_ref().is_some_and(|f| **f == *files);
        self.files = Some(files);
        if !unchanged {
            self.rebuild();
        }
    }

    pub fn rebuild(&mut self) {
        let Some(files) = &self.files else { return };
        (self.rows, self.starts) = build_rows(
            files,
            &self.collapsed,
            &self.shown_large,
            self.inline_rows(),
        );
        // A selection from an older diff may point past the new files or hunks.
        if self
            .sel
            .is_some_and(|s| files.get(s.fi).is_none_or(|f| s.lines(f).is_empty()))
        {
            self.sel = None;
        }
        // One extra trailing item: the scroll past end spacer, never a row.
        self.list.reset(self.rows.len() + 1);
    }

    /// Index of the file at the top of the viewport.
    pub fn current_file(&self) -> usize {
        let ix = self
            .list
            .logical_scroll_top()
            .item_ix
            .min(self.rows.len().saturating_sub(1));
        self.rows.get(ix).map(|r| r.file()).unwrap_or(0)
    }

    pub fn file_count(&self) -> usize {
        self.files.as_ref().map(|f| f.len()).unwrap_or(0)
    }

    /// Re-render one file's rows in place, keeping the scroll position.
    fn respan(&mut self, fi: usize) {
        let Some(files) = self.files.clone() else {
            return;
        };
        let (Some(&start), Some(f)) = (self.starts.get(fi), files.get(fi)) else {
            return;
        };
        let end = self
            .starts
            .get(fi + 1)
            .copied()
            .unwrap_or(self.rows.len())
            .clamp(start, self.rows.len());
        if start > self.rows.len() {
            return;
        }
        let new = file_rows(
            fi,
            f,
            self.collapsed.contains(&fi),
            self.shown_large.contains(&fi),
            self.inline_rows(),
        );
        let delta = new.len() as isize - (end - start) as isize;
        self.list.splice(start..end, new.len());
        self.rows.splice(start..end, new);
        for s in self.starts.iter_mut().skip(fi + 1) {
            *s = (*s as isize + delta) as usize;
        }
    }

    pub fn toggle_file(&mut self, fi: usize) {
        if !self.collapsed.remove(&fi) {
            self.collapsed.insert(fi);
        }
        self.respan(fi);
    }

    pub fn show_large(&mut self, fi: usize) {
        self.shown_large.insert(fi);
        self.respan(fi);
    }

    /// Collapse every file, or expand all when everything is collapsed.
    pub fn toggle_all(&mut self) {
        let cur = self.current_file();
        let n = self.file_count();
        if self.collapsed.len() == n {
            self.collapsed.clear();
        } else {
            self.collapsed = (0..n).collect();
        }
        self.rebuild();
        self.goto(cur);
    }

    pub fn goto(&mut self, fi: usize) {
        if let Some(&ix) = self.starts.get(fi) {
            self.list.scroll_to(ListOffset {
                item_ix: ix,
                offset_in_item: px(0.),
            });
        }
    }

    pub fn step(&mut self, delta: isize) {
        let n = self.file_count();
        if n > 0 {
            let next = (self.current_file() as isize + delta).clamp(0, n as isize - 1) as usize;
            self.goto(next);
        }
    }
}
