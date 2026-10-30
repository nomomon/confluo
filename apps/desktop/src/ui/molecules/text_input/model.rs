//! Pure caret, selection and history logic for `TextInput` (no GPUI).
//!
//! `Rows` is the visual layout: one entry per wrapped row with the x of every caret stop,
//! so vertical moves, line starts and ends work the same on shaped text and in tests.

use std::ops::Range;

/// One visual row: byte range `start..end` (end excludes a hard newline) and caret stops.
#[derive(Clone, Debug, Default)]
pub struct Row {
    pub start: usize,
    pub end: usize,
    /// True when the row ends at a soft wrap (the next row continues the same paragraph).
    pub soft: bool,
    /// `(byte offset, x)` for each caret position in the row, ascending, ending at `end`.
    pub stops: Vec<(usize, f32)>,
}

#[derive(Clone, Debug, Default)]
pub struct Rows(pub Vec<Row>);

impl Rows {
    /// Monospace layout (every char 1 wide), hard-wrapped at `width` chars; for tests.
    #[cfg(test)]
    pub fn mono(text: &str, width: usize) -> Self {
        let mut rows = vec![];
        let mut start = 0;
        for para in text.split('\n') {
            let chars: Vec<(usize, char)> = para.char_indices().collect();
            let mut i = 0;
            loop {
                let end_i = (i + width).min(chars.len());
                let soft = end_i < chars.len();
                let byte = |k: usize| start + chars.get(k).map(|c| c.0).unwrap_or(para.len());
                let stops = (i..=end_i).map(|k| (byte(k), (k - i) as f32)).collect();
                rows.push(Row { start: byte(i), end: byte(end_i), soft, stops });
                i = end_i;
                if !soft {
                    break;
                }
            }
            start += para.len() + 1;
        }
        Rows(rows)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Row holding the caret at `off`; at a soft wrap the caret sits on the next row.
    pub fn row_of(&self, off: usize) -> usize {
        self.0.iter().rposition(|r| r.start <= off).unwrap_or(0)
    }

    pub fn x_of(&self, off: usize) -> f32 {
        let Some(row) = self.0.get(self.row_of(off)) else { return 0. };
        row.stops.iter().find(|s| s.0 >= off).or(row.stops.last()).map(|s| s.1).unwrap_or(0.)
    }

    /// Closest caret stop to `x` on `row`; a soft-wrapped row never returns its end (that is the next row).
    pub fn offset_at(&self, row: usize, x: f32) -> usize {
        let Some(r) = self.0.get(row) else { return 0 };
        let stops = if r.soft && r.stops.len() > 1 { &r.stops[..r.stops.len() - 1] } else { &r.stops[..] };
        stops.iter().min_by(|a, b| (a.1 - x).abs().total_cmp(&(b.1 - x).abs())).map(|s| s.0).unwrap_or(r.start)
    }

    pub fn row_home(&self, off: usize) -> usize {
        self.0.get(self.row_of(off)).map(|r| r.start).unwrap_or(0)
    }

    /// End of the caret's visual row; before the wrap point on a soft-wrapped row.
    pub fn row_end(&self, off: usize) -> usize {
        let Some(r) = self.0.get(self.row_of(off)) else { return off };
        if r.soft && r.stops.len() > 1 {
            r.stops[r.stops.len() - 2].0
        } else {
            r.end
        }
    }

    /// Caret after moving `delta` rows, keeping the goal column; returns the offset and the goal x.
    /// Past the first or last row it goes to the document start or end.
    pub fn vertical(&self, off: usize, delta: isize, goal: Option<f32>, len: usize) -> (usize, f32) {
        let x = goal.unwrap_or_else(|| self.x_of(off));
        let target = self.row_of(off) as isize + delta;
        if target < 0 {
            return (0, x);
        }
        if target as usize >= self.len() {
            return (len, x);
        }
        (self.offset_at(target as usize, x), x)
    }
}

/// `off` clamped into `text` and moved back to a char boundary.
pub fn floor_boundary(text: &str, off: usize) -> usize {
    let mut o = off.min(text.len());
    while !text.is_char_boundary(o) {
        o -= 1;
    }
    o
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Option+Left: skip non-word characters, then the word before the caret.
pub fn word_left(text: &str, off: usize) -> usize {
    let Some(head) = text.get(..off) else { return off.min(text.len()) };
    let mut it = head.char_indices().rev().peekable();
    let mut pos = off;
    while let Some(&(i, c)) = it.peek() {
        if is_word(c) {
            break;
        }
        pos = i;
        it.next();
    }
    while let Some(&(i, c)) = it.peek() {
        if !is_word(c) {
            break;
        }
        pos = i;
        it.next();
    }
    pos
}

/// Option+Right: skip non-word characters, then the word after the caret.
pub fn word_right(text: &str, off: usize) -> usize {
    let Some(tail) = text.get(off..) else { return off.min(text.len()) };
    let mut it = tail.char_indices().peekable();
    let mut pos = off;
    let mut skip = |word: bool, it: &mut std::iter::Peekable<std::str::CharIndices>| {
        while let Some(&(i, c)) = it.peek() {
            if is_word(c) == word {
                break;
            }
            pos = off + i + c.len_utf8();
            it.next();
        }
    };
    skip(true, &mut it);
    skip(false, &mut it);
    pos
}

/// Double click: the word (or whitespace run, or single symbol) under `off`.
pub fn word_at(text: &str, off: usize) -> Range<usize> {
    let off = floor_boundary(text, off);
    let Some(c) = text[off..].chars().next().or_else(|| text[..off].chars().next_back()) else { return off..off };
    let at = if text[off..].is_empty() { off - c.len_utf8() } else { off };
    let class = |ch: char| if is_word(ch) { 0 } else if ch.is_whitespace() && ch != '\n' { 1 } else { 2 };
    let k = class(c);
    if k == 2 {
        return at..at + c.len_utf8();
    }
    let start = text[..at].char_indices().rev().take_while(|&(_, ch)| class(ch) == k).last().map(|(i, _)| i).unwrap_or(at);
    let end = text[at..].char_indices().take_while(|&(_, ch)| class(ch) == k).last().map(|(i, ch)| at + i + ch.len_utf8()).unwrap_or(at);
    start..end
}

/// Triple click: the paragraph (logical line, without its newline) under `off`.
pub fn line_at(text: &str, off: usize) -> Range<usize> {
    let off = floor_boundary(text, off);
    let start = text[..off].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let end = text[off..].find('\n').map(|i| off + i).unwrap_or(text.len());
    start..end
}

#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub text: String,
    pub sel: Range<usize>,
}

/// Undo/redo stack; consecutive typed characters coalesce into one step.
#[derive(Default)]
pub struct History {
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    typing: bool,
}

const MAX_UNDO: usize = 200;

impl History {
    /// Call before an edit with the state it replaces. `typing` marks a single typed character.
    pub fn record(&mut self, before: Snapshot, typing: bool) {
        self.redo.clear();
        if typing && self.typing {
            return;
        }
        self.typing = typing;
        self.undo.push(before);
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
    }

    /// Breaks typing coalescing (caret moved).
    pub fn seal(&mut self) {
        self.typing = false;
    }

    pub fn undo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let prev = self.undo.pop()?;
        self.redo.push(current);
        self.typing = false;
        Some(prev)
    }

    pub fn redo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let next = self.redo.pop()?;
        self.undo.push(current);
        self.typing = false;
        Some(next)
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_wrap_and_row_of() {
        let r = Rows::mono("abcdefgh\nxy", 4);
        assert_eq!(r.len(), 3);
        assert_eq!((r.0[0].start, r.0[0].end, r.0[0].soft), (0, 4, true));
        assert_eq!((r.0[1].start, r.0[1].end, r.0[1].soft), (4, 8, false));
        assert_eq!((r.0[2].start, r.0[2].end), (9, 11));
        assert_eq!(r.row_of(4), 1, "soft wrap boundary belongs to the next row");
        assert_eq!(r.row_of(8), 1, "before the newline stays on the row");
        assert_eq!(r.row_of(9), 2);
    }

    #[test]
    fn vertical_keeps_goal_column_across_wrapped_and_short_rows() {
        let text = "abcdefgh\nxy\nlonger line";
        let r = Rows::mono(text, 4);
        // Start at "d" column 3 on row 0, go down: row 1 col 3 ("h"=7).
        let (o, x) = r.vertical(3, 1, None, text.len());
        assert_eq!((o, x), (7, 3.));
        // Down again to the short "xy" row: clamps to its end (11).
        let (o, x) = r.vertical(o, 1, Some(x), text.len());
        assert_eq!(o, 11);
        // Down again with the goal kept: column 3 on "long" row.
        let (o, _) = r.vertical(o, 1, Some(x), text.len());
        assert_eq!(o, 12 + 3);
        // Up from the first row goes to document start, down from the last to the end.
        assert_eq!(r.vertical(2, -1, None, text.len()).0, 0);
        let last = r.len() - 1;
        let in_last = r.0[last].start + 1;
        assert_eq!(r.vertical(in_last, 1, None, text.len()).0, text.len());
    }

    #[test]
    fn soft_row_never_lands_on_its_end() {
        let r = Rows::mono("abcdefgh", 4);
        assert_eq!(r.offset_at(0, 10.), 3);
        assert_eq!(r.row_end(1), 3);
        assert_eq!(r.row_end(5), 8);
        assert_eq!(r.row_home(6), 4);
    }

    #[test]
    fn word_moves() {
        let t = "fn foo_bar(x) -> y";
        assert_eq!(word_right(t, 0), 2);
        assert_eq!(word_right(t, 2), 10);
        assert_eq!(word_right(t, 10), 12);
        assert_eq!(word_left(t, 10), 3);
        assert_eq!(word_left(t, 3), 0);
        assert_eq!(word_left(t, t.len()), t.len() - 1);
        assert_eq!(word_left(t, 0), 0);
        assert_eq!(word_right(t, t.len()), t.len());
        assert_eq!(word_right("héllo wörld", 0), "héllo".len());
    }

    #[test]
    fn word_and_line_ranges() {
        let t = "one two  three\nnext";
        assert_eq!(&t[word_at(t, 5)], "two");
        assert_eq!(&t[word_at(t, 7)], "  ");
        assert_eq!(&t[word_at(t, t.len())], "next");
        assert_eq!(&t[line_at(t, 5)], "one two  three");
        assert_eq!(&t[line_at(t, 16)], "next");
        assert_eq!(word_at("", 0), 0..0);
    }

    #[test]
    fn history_coalesces_typing_and_redoes() {
        let snap = |t: &str| Snapshot { text: t.into(), sel: t.len()..t.len() };
        let mut h = History::default();
        h.record(snap(""), true);
        h.record(snap("a"), true);
        h.record(snap("ab"), true);
        h.record(snap("abc"), false); // paste
        assert_eq!(h.undo(snap("abcXYZ")), Some(snap("abc")));
        assert_eq!(h.undo(snap("abc")), Some(snap("")));
        assert_eq!(h.undo(snap("")), None);
        assert_eq!(h.redo(snap("")), Some(snap("abc")));
        assert_eq!(h.redo(snap("abc")), Some(snap("abcXYZ")));
        h.record(snap("abcXYZ"), false);
        assert_eq!(h.redo(snap("q")), None, "a new edit clears redo");
    }
}
