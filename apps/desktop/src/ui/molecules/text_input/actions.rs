//! Keyboard, mouse and clipboard handlers plus offset helpers for `TextInput`.

use gpui::{
    ClipboardItem, Context, EntityInputHandler, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    Pixels, Point, Window,
};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

use super::model::{self, Snapshot};
use super::*;

impl TextInput {
    pub(super) fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx)
        }
    }
    pub(super) fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx)
        }
    }
    fn vertical(&mut self, delta: isize, extend: bool, cx: &mut Context<Self>) {
        let from = if extend || self.selected_range.is_empty() {
            self.cursor_offset()
        } else if delta < 0 {
            self.selected_range.start
        } else {
            self.selected_range.end
        };
        let (to, x) = self
            .rows
            .vertical(from, delta, self.goal_x, self.content.len());
        if extend {
            self.select_to(to, cx)
        } else {
            self.move_to(to, cx)
        }
        self.goal_x = Some(x);
    }
    pub(super) fn up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        self.vertical(-1, false, cx)
    }
    pub(super) fn down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        self.vertical(1, false, cx)
    }
    pub(super) fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        self.vertical(-1, true, cx)
    }
    pub(super) fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        self.vertical(1, true, cx)
    }
    pub(super) fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }
    pub(super) fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }
    pub(super) fn word_left(&mut self, _: &WordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(model::word_left(&self.content, self.cursor_offset()), cx)
    }
    pub(super) fn word_right(&mut self, _: &WordRight, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(model::word_right(&self.content, self.cursor_offset()), cx)
    }
    pub(super) fn select_word_left(
        &mut self,
        _: &SelectWordLeft,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_to(model::word_left(&self.content, self.cursor_offset()), cx)
    }
    pub(super) fn select_word_right(
        &mut self,
        _: &SelectWordRight,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_to(model::word_right(&self.content, self.cursor_offset()), cx)
    }
    pub(super) fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
        self.select_to(self.content.len(), cx)
    }
    fn line_home(&self) -> usize {
        if self.rows.is_empty() {
            0
        } else {
            self.rows.row_home(self.cursor_offset())
        }
    }
    fn line_end(&self) -> usize {
        if self.rows.is_empty() {
            self.content.len()
        } else {
            self.rows.row_end(self.cursor_offset())
        }
    }
    pub(super) fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.line_home(), cx);
    }
    pub(super) fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.line_end(), cx);
    }
    pub(super) fn select_home(&mut self, _: &SelectHome, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.line_home(), cx);
    }
    pub(super) fn select_end(&mut self, _: &SelectEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.line_end(), cx);
    }
    pub(super) fn doc_start(&mut self, _: &DocStart, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }
    pub(super) fn doc_end(&mut self, _: &DocEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.content.len(), cx);
    }
    pub(super) fn select_doc_start(
        &mut self,
        _: &SelectDocStart,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_to(0, cx);
    }
    pub(super) fn select_doc_end(
        &mut self,
        _: &SelectDocEnd,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_to(self.content.len(), cx);
    }
    pub(super) fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }
    pub(super) fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }
    pub(super) fn delete_word_left(
        &mut self,
        _: &DeleteWordLeft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_range.is_empty() {
            self.select_to(model::word_left(&self.content, self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }
    pub(super) fn delete_word_right(
        &mut self,
        _: &DeleteWordRight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_range.is_empty() {
            self.select_to(model::word_right(&self.content, self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }
    pub(super) fn delete_to_line_start(
        &mut self,
        _: &DeleteToLineStart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_range.is_empty() {
            let off = self.cursor_offset();
            let home = self.line_home();
            self.select_to(
                if home == off {
                    self.previous_boundary(off)
                } else {
                    home
                },
                cx,
            )
        }
        self.replace_text_in_range(None, "", window, cx)
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.content.to_string(),
            sel: self.selected_range.clone(),
        }
    }
    fn restore(&mut self, s: Snapshot, cx: &mut Context<Self>) {
        self.content = s.text.into();
        let n = self.content.len();
        self.selected_range = s.sel.start.min(n)..s.sel.end.min(n);
        self.selection_reversed = false;
        self.marked_range = None;
        self.reveal = true;
        cx.emit(InputEvent::Changed);
        cx.notify();
    }
    /// Records the pre-edit state for undo; called by every text replacement.
    pub(super) fn record_edit(&mut self, new_text: &str, range: &Range<usize>) {
        let typing = range.is_empty()
            && new_text.graphemes(true).count() == 1
            && !new_text.contains(char::is_whitespace);
        let snap = self.snapshot();
        self.history.record(snap, typing);
        self.goal_x = None;
        self.reveal = true;
    }
    pub(super) fn undo(&mut self, _: &Undo, _: &mut Window, cx: &mut Context<Self>) {
        let cur = self.snapshot();
        if let Some(s) = self.history.undo(cur) {
            self.restore(s, cx)
        }
    }
    pub(super) fn redo(&mut self, _: &Redo, _: &mut Window, cx: &mut Context<Self>) {
        let cur = self.snapshot();
        if let Some(s) = self.history.redo(cur) {
            self.restore(s, cx)
        }
    }
    pub(super) fn tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        if self.markdown {
            if let Some(e) = super::md_edit::indent(&self.content, &self.selected_range, false) {
                return self.apply_edit(e, cx);
            }
        }
        self.replace_text_in_range(None, "  ", window, cx)
    }
    pub(super) fn outdent(&mut self, _: &Outdent, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(e) = super::md_edit::indent(&self.content, &self.selected_range, true) {
            self.apply_edit(e, cx)
        }
    }
    pub(super) fn bold(&mut self, _: &Bold, _: &mut Window, cx: &mut Context<Self>) {
        self.apply_edit(
            super::md_edit::wrap(&self.content, &self.selected_range, "**", "**"),
            cx,
        )
    }
    pub(super) fn italic(&mut self, _: &Italic, _: &mut Window, cx: &mut Context<Self>) {
        self.apply_edit(
            super::md_edit::wrap(&self.content, &self.selected_range, "_", "_"),
            cx,
        )
    }
    pub(super) fn link(&mut self, _: &Link, _: &mut Window, cx: &mut Context<Self>) {
        self.apply_edit(
            super::md_edit::link(&self.content, &self.selected_range),
            cx,
        )
    }
    /// One undoable markdown edit: replace a range and set the selection.
    fn apply_edit(&mut self, e: super::md_edit::Edit, cx: &mut Context<Self>) {
        let range = self.clamp_range(e.range);
        self.record_edit(&e.text, &range);
        self.history.seal();
        self.content =
            (self.content[..range.start].to_owned() + &e.text + &self.content[range.end..]).into();
        self.selected_range = self.clamp_range(e.sel);
        self.selection_reversed = false;
        self.marked_range = None;
        cx.emit(InputEvent::Changed);
        cx.notify();
    }
    pub(super) fn submit(&mut self, _: &Submit, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(InputEvent::Submit);
    }
    pub(super) fn newline(&mut self, _: &Newline, window: &mut Window, cx: &mut Context<Self>) {
        if self.markdown && self.marked_range.is_none() {
            if let Some(e) = super::md_edit::enter(&self.content, &self.selected_range) {
                return self.apply_edit(e, cx);
            }
        }
        self.replace_text_in_range(None, "\n", window, cx)
    }
    pub(super) fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle);
        let at = self.index_for_mouse_position(event.position);
        match event.click_count {
            2 => {
                let r = model::word_at(&self.content, at);
                self.move_to(r.start, cx);
                self.select_to(r.end, cx);
            }
            n if n >= 3 => {
                let r = model::line_at(&self.content, at);
                self.move_to(r.start, cx);
                self.select_to(r.end, cx);
            }
            _ => {
                self.is_selecting = true;
                if event.modifiers.shift {
                    self.select_to(at, cx);
                } else {
                    self.move_to(at, cx)
                }
            }
        }
    }
    pub(super) fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }
    pub(super) fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }
    pub(super) fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            let text = if self.multiline {
                text
            } else {
                text.replace('\n', " ")
            };
            self.replace_text_in_range(None, &text, window, cx);
        }
    }
    pub(super) fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() && !self.masked {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.clamp_range(self.selected_range.clone())].to_string(),
            ));
        }
    }
    pub(super) fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() && !self.masked {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.clamp_range(self.selected_range.clone())].to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx)
        }
    }
    /// Offsets from last frame's layout can be stale after an edit; keep them inside the text on a char boundary.
    pub(super) fn clamp_offset(&self, offset: usize) -> usize {
        model::floor_boundary(&self.content, offset)
    }
    pub(super) fn clamp_range(&self, r: Range<usize>) -> Range<usize> {
        let (a, b) = (self.clamp_offset(r.start), self.clamp_offset(r.end));
        a.min(b)..b.max(a)
    }
    pub(super) fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let offset = self.clamp_offset(offset);
        self.selected_range = offset..offset;
        self.selection_reversed = false;
        self.after_caret_move();
        cx.notify()
    }
    fn after_caret_move(&mut self) {
        self.goal_x = None;
        self.history.seal();
        self.reveal = true;
    }
    pub(super) fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }
    pub(super) fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.content.is_empty() || self.rows.is_empty() {
            return 0;
        }
        let Some(b) = self.last_bounds else { return 0 };
        let lh = self.last_line_height.max(gpui::px(1.));
        let y = position.y - b.top() + self.scroll_y;
        if y < gpui::px(0.) {
            return 0;
        }
        let row = (y / lh).floor() as usize;
        if row >= self.rows.len() {
            return self.content.len();
        }
        self.rows.offset_at(row, f32::from(position.x - b.left()))
    }
    pub(super) fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let offset = self.clamp_offset(offset);
        self.selected_range = self.clamp_range(self.selected_range.clone());
        if self.selection_reversed {
            self.selected_range.start = offset
        } else {
            self.selected_range.end = offset
        };
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        self.after_caret_move();
        cx.notify()
    }
    pub(super) fn offset_from_utf16(&self, offset: usize) -> usize {
        let (mut utf8, mut utf16) = (0, 0);
        for ch in self.content.chars() {
            if utf16 >= offset {
                break;
            }
            utf16 += ch.len_utf16();
            utf8 += ch.len_utf8();
        }
        utf8
    }
    pub(super) fn offset_to_utf16(&self, offset: usize) -> usize {
        let (mut utf16, mut utf8) = (0, 0);
        for ch in self.content.chars() {
            if utf8 >= offset {
                break;
            }
            utf8 += ch.len_utf8();
            utf16 += ch.len_utf16();
        }
        utf16
    }
    pub(super) fn range_to_utf16(&self, r: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(r.start)..self.offset_to_utf16(r.end)
    }
    pub(super) fn range_from_utf16(&self, r: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(r.start)..self.offset_from_utf16(r.end)
    }
    pub(super) fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(i, _)| (i < offset).then_some(i))
            .unwrap_or(0)
    }
    pub(super) fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(i, _)| (i > offset).then_some(i))
            .unwrap_or(self.content.len())
    }
}
