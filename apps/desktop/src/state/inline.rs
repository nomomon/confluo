//! Inline review comment state of the Code tab: placed threads, the open composer and reply/edit mode.

use std::collections::HashSet;
use std::sync::Arc;

use gpui::{px, ListOffset};
use gv_engine::{index_inline_threads, DiffPosition, DiffSide, InlineIndex};

use super::code_rows::Row;
use super::DetailState;

/// A new comment being written on one line or a range of lines of one file.
#[derive(Clone, Debug, PartialEq)]
pub struct LineDraft {
    pub fi: usize,
    pub path: String,
    pub old_path: Option<String>,
    pub side: DiffSide,
    /// Anchor (last) line.
    pub line: u32,
    /// First line when a range is selected.
    pub start: Option<u32>,
    /// Opposite side's number for context lines.
    pub other_line: Option<u32>,
}

impl LineDraft {
    pub fn position(&self) -> DiffPosition {
        DiffPosition {
            path: self.path.clone(),
            old_path: self.old_path.clone(),
            side: self.side,
            line: self.line,
            start_line: self.start,
            other_line: self.other_line,
            ..Default::default()
        }
    }

    /// Whether a line on `side` numbered `n` of file `fi` is inside the draft's range.
    pub fn covers(&self, fi: usize, side: DiffSide, n: u32) -> bool {
        fi == self.fi && side == self.side && (self.start.unwrap_or(self.line)..=self.line).contains(&n)
    }
}

#[derive(Default)]
pub struct InlineState {
    pub index: Arc<InlineIndex>,
    pub draft: Option<LineDraft>,
    /// Thread whose reply field is open.
    pub reply_to: Option<String>,
    /// Comment being edited in place.
    pub editing: Option<String>,
    /// Resolved threads the user expanded.
    pub expanded: HashSet<String>,
    pub posting: bool,
    /// Thread to scroll to once its row exists ("Show in code").
    pub reveal: Option<String>,
}

impl InlineState {
    /// Close the composer and any reply or edit field.
    pub fn clear_modes(&mut self) {
        self.draft = None;
        self.reply_to = None;
        self.editing = None;
    }
}

impl DetailState {
    /// Re-place inline threads on the diff and splice the changed rows.
    pub fn reindex(&mut self) {
        let (Some(det), Some(files)) = (&self.detail, &self.code.files) else { return };
        self.code.inline.index = Arc::new(index_inline_threads(&det.threads, files));
        self.code.sync_rows();
        self.reveal_pending();
    }

    /// Scroll to the thread requested by "Show in code", expanding its file and resolved state.
    fn reveal_pending(&mut self) {
        let Some(tid) = self.code.inline.reveal.clone() else { return };
        let Some(det) = &self.detail else { return };
        let Some(ti) = det.threads.iter().position(|t| t.id == tid) else { return };
        let files = self.code.files.clone().unwrap_or_default();
        if let Some(fi) = files.iter().position(|f| self.thread_in_file(ti, f.path())) {
            let needs = self.code.collapsed.remove(&fi) | self.code.shown_large.insert(fi);
            self.code.inline.expanded.insert(tid);
            if needs {
                self.code.sync_rows();
            }
        }
        if let Some(ix) = self.code.rows.iter().position(|r| matches!(r, Row::Thread(_, t) if *t == ti)) {
            self.code.inline.reveal = None;
            self.code.list.scroll_to(ListOffset { item_ix: ix.saturating_sub(3), offset_in_item: px(0.) });
        }
    }

    fn thread_in_file(&self, ti: usize, path: &str) -> bool {
        let ix = &self.code.inline.index;
        ix.outdated.get(path).is_some_and(|v| v.contains(&ti)) || ix.by_line.iter().any(|((p, _, _), v)| p == path && v.contains(&ti))
    }
}
