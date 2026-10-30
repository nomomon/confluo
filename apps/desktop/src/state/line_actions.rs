//! Line selection actions: mouse select and drag, keyboard cursor, copy, links and comment.

use gpui::{px, Context, Pixels, ScrollStrategy, Window};

use super::code::CodeState;
use super::line_select::{copy_text, DiffSel, LineRange};
use super::permalink::{diff_link, file_link, LinkBase};
use super::Copied;
use crate::app::GitViewer;

/// Distance from the list edge where a drag starts scrolling, and the step per move.
const EDGE: Pixels = px(28.);
const EDGE_STEP: Pixels = px(20.);

impl GitViewer {
    fn code_mut(&mut self) -> Option<&mut CodeState> {
        self.detail.as_mut().map(|d| &mut d.code)
    }

    /// Mouse down on a diff line: select it, or extend with Shift; starts a drag.
    pub(crate) fn press_diff_line(&mut self, fi: usize, hi: usize, li: usize, shift: bool, cx: &mut Context<Self>) {
        let Some(c) = self.code_mut() else { return };
        let len = c.hunk_len(fi, hi);
        match c.sel.as_mut().filter(|_| shift) {
            Some(s) => s.extend_to(fi, hi, li, len),
            None => c.sel = Some(DiffSel::single(fi, hi, li)),
        }
        c.dragging = true;
        c.sel_kbd = false;
        cx.notify()
    }

    /// Mouse moved over a diff line with the button held.
    pub(crate) fn drag_diff_line(&mut self, fi: usize, hi: usize, li: usize, cx: &mut Context<Self>) {
        let Some(c) = self.code_mut().filter(|c| c.dragging) else { return };
        let Some(mut s) = c.sel else { return };
        let len = c.hunk_len(s.fi, s.hi);
        s.extend_to(fi, hi, li, len);
        if c.sel != Some(s) {
            c.sel = Some(s);
            cx.notify()
        }
    }

    /// Whole-file viewer: press and drag on line `i`.
    pub(crate) fn press_view_line(&mut self, i: usize, shift: bool, drag: bool, cx: &mut Context<Self>) {
        let Some(c) = self.code_mut() else { return };
        if drag && !c.dragging {
            return;
        }
        let Some(v) = c.viewer.as_mut() else { return };
        let len = v.lines.as_ref().map(|l| l.len()).unwrap_or(0);
        match v.sel.as_mut().filter(|_| shift || drag) {
            Some(r) => r.extend_to(i, len),
            None => v.sel = Some(LineRange::at(i)),
        }
        c.dragging = true;
        c.sel_kbd = false;
        cx.notify()
    }

    /// Press on the gutter "+": select the line and comment on the dragged range on release.
    pub(crate) fn press_line_plus(&mut self, fi: usize, hi: usize, li: usize, cx: &mut Context<Self>) {
        self.press_diff_line(fi, hi, li, false, cx);
        if let Some(c) = self.code_mut() {
            c.plus_drag = true;
        }
    }

    /// Mouse released: end the drag; a drag from the "+" opens the composer on the selection.
    pub(crate) fn finish_line_drag(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let plus = self.code_mut().map(|c| std::mem::take(&mut c.plus_drag)).unwrap_or(false);
        self.end_line_drag(cx);
        if plus {
            self.comment_on_selection(window, cx);
        }
    }

    pub(crate) fn end_line_drag(&mut self, cx: &mut Context<Self>) {
        if let Some(c) = self.code_mut().filter(|c| c.dragging) {
            c.dragging = false;
            cx.notify()
        }
    }

    /// While dragging near the top or bottom edge of the diff, scroll it.
    pub(crate) fn drag_autoscroll(&mut self, y: Pixels, cx: &mut Context<Self>) {
        let Some(c) = self.code_mut().filter(|c| c.dragging && c.viewer.is_none()) else { return };
        let b = c.list.viewport_bounds();
        let step = if y < b.top() + EDGE { -EDGE_STEP } else if y > b.bottom() - EDGE { EDGE_STEP } else { return };
        c.list.scroll_by(step);
        cx.notify()
    }

    /// Clear the diff or viewer selection. Returns whether there was one.
    pub(crate) fn clear_line_selection(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(c) = self.code_mut() else { return false };
        let had = match c.viewer.as_mut() {
            Some(v) => v.sel.take().is_some(),
            None => c.sel.take().is_some(),
        };
        c.dragging = false;
        c.plus_drag = false;
        cx.notify();
        had
    }

    /// Keyboard line cursor (no animation).
    pub(crate) fn step_line(&mut self, delta: isize, extend: bool, cx: &mut Context<Self>) {
        let Some(c) = self.code_mut() else { return };
        c.sel_kbd = true;
        match c.viewer.as_mut() {
            Some(v) => {
                let len = v.lines.as_ref().map(|l| l.len()).unwrap_or(0);
                if len == 0 {
                    return;
                }
                let r = v.sel.get_or_insert(LineRange::at(0));
                let head = (r.head as isize + delta).clamp(0, len as isize - 1) as usize;
                if extend { r.head = head } else { *r = LineRange::at(head) }
                reveal_view_line(&v.scroll, head);
            }
            None => c.step_line(delta, extend),
        }
        cx.notify()
    }

    pub(crate) fn has_line_selection(&self) -> bool {
        self.detail.as_ref().is_some_and(|d| match &d.code.viewer {
            Some(v) => v.sel.is_some(),
            None => d.code.sel.is_some(),
        })
    }

    /// Code text of the selected lines.
    pub(crate) fn selection_text(&self) -> Option<String> {
        let c = &self.detail.as_ref()?.code;
        if let Some(v) = &c.viewer {
            let (r, lines) = (v.sel?, v.lines.as_ref()?);
            return Some(copy_text(lines[r.lo()..=r.hi().min(lines.len() - 1)].iter().map(String::as_str)));
        }
        let s = c.sel?;
        Some(copy_text(s.lines(&c.files.as_ref()?[s.fi]).iter().map(|l| l.text.as_str())))
    }

    /// Provider permalink to the selected lines.
    pub(crate) fn selection_link(&self) -> Option<String> {
        let d = self.detail.as_ref()?;
        let repo_url = d.summary.id.repo.web_url();
        let sha = d.detail.as_ref().and_then(|x| x.head_sha.as_deref());
        let base = LinkBase { kind: d.summary.id.repo.kind, pr_url: &d.summary.web_url, repo_url: &repo_url, head_sha: sha };
        if let Some(v) = &d.code.viewer {
            let r = v.sel?;
            return Some(file_link(&base, &v.path, r.lo() as u32 + 1, r.hi() as u32 + 1));
        }
        let s = d.code.sel?;
        let f = &d.code.files.as_ref()?[s.fi];
        Some(diff_link(&base, f.path(), s.lines(f)))
    }

    pub(crate) fn copy_selection(&mut self, cx: &mut Context<Self>) {
        if let Some(t) = self.selection_text() {
            self.copy(Copied::Lines, t, cx)
        }
    }

    pub(crate) fn copy_selection_link(&mut self, cx: &mut Context<Self>) {
        if let Some(t) = self.selection_link() {
            self.copy(Copied::Link, t, cx)
        }
    }

    /// Open the inline composer on the diff selection.
    pub(crate) fn comment_on_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.detail.as_ref().filter(|d| d.code.viewer.is_none()).and_then(|d| d.code.sel) else { return };
        self.open_line_draft(s.fi, s.hi, s.range.lo(), s.range.hi(), window, cx)
    }
}

/// Scroll the viewer the least amount that shows line `i`.
fn reveal_view_line(scroll: &gpui::UniformListScrollHandle, i: usize) {
    let (top, h, item) = {
        let st = scroll.0.borrow();
        let item = st.last_item_size.map(|s| s.item.height).unwrap_or(px(20.));
        (-st.base_handle.offset().y, st.base_handle.bounds().size.height, item)
    };
    let y = item * i as f32;
    if y < top {
        scroll.scroll_to_item(i, ScrollStrategy::Top)
    } else if y + item * 2. > top + h {
        scroll.scroll_to_item(i, ScrollStrategy::Bottom)
    }
}
