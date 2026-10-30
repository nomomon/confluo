//! Code tab line cursor over the flat row list, and the dev preselection.

use gpui::{px, ListOffset};

use super::code::CodeState;
use super::code_rows::Row;
use super::line_select::{DiffSel, LineRange};

impl CodeState {
    pub(super) fn hunk_len(&self, fi: usize, hi: usize) -> usize {
        self.files
            .as_ref()
            .and_then(|f| f.get(fi))
            .and_then(|f| f.hunks.get(hi))
            .map(|h| h.lines.len())
            .unwrap_or(0)
    }

    fn row_of(&self, fi: usize, hi: usize, li: usize) -> Option<usize> {
        self.rows.iter().position(|r| r.has_line(fi, hi, li))
    }

    /// Up/down: move a one-line cursor across hunks and files, or grow the selection in its hunk.
    pub fn step_line(&mut self, delta: isize, extend: bool) {
        let next = match self.sel {
            Some(mut s) if extend => {
                let len = self.hunk_len(s.fi, s.hi);
                s.range
                    .extend_to((s.range.head as isize + delta).max(0) as usize, len);
                Some(s)
            }
            Some(s) => self.row_of(s.fi, s.hi, s.range.head).and_then(|ix| {
                let mut it: Box<dyn Iterator<Item = &Row>> = if delta > 0 {
                    Box::new(self.rows.iter().skip(ix + 1))
                } else {
                    Box::new(self.rows.iter().take(ix).rev())
                };
                it.find_map(|r| r.line().map(|(f, h, l)| DiffSel::single(f, h, l)))
            }),
            None => {
                let top = self.list.logical_scroll_top().item_ix;
                self.rows[top.min(self.rows.len())..]
                    .iter()
                    .find_map(|r| r.line().map(|(f, h, l)| DiffSel::single(f, h, l)))
            }
        };
        let Some(s) = next else { return };
        self.sel = Some(s);
        self.sel_kbd = true;
        if let Some(ix) = self.row_of(s.fi, s.hi, s.range.head) {
            self.list.scroll_to_reveal_item(ix);
        }
    }

    /// Apply `CONFLUO_SELECT` once files are loaded.
    pub fn apply_pending_select(&mut self) {
        let Some(files) = self.files.clone() else {
            return;
        };
        let Some(p) = self.pending_select.take() else {
            return;
        };
        let Some(fi) = files.iter().position(|f| f.path() == p.path) else {
            return;
        };
        let Some((hi, lo, last)) = p.locate(&files[fi]) else {
            return;
        };
        self.collapsed.remove(&fi);
        self.shown_large.insert(fi);
        self.sync_rows();
        self.sel = Some(DiffSel {
            fi,
            hi,
            range: LineRange {
                anchor: lo,
                head: last,
            },
        });
        if let Some(ix) = self.row_of(fi, hi, lo) {
            self.list.scroll_to(ListOffset {
                item_ix: ix.saturating_sub(6),
                offset_in_item: px(0.),
            });
        }
    }
}
