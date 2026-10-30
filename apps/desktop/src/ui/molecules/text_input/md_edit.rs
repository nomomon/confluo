//! Markdown editing helpers for `TextInput::markdown()` (pure, no GPUI): list continuation on Enter,
//! indent / outdent of list items, and wrapping the selection for Cmd+B / Cmd+I / Cmd+K.

use std::ops::Range;

/// Replace `range` with `text`, then select `sel` (offsets in the new text).
#[derive(Clone, Debug, PartialEq)]
pub struct Edit {
    pub range: Range<usize>,
    pub text: String,
    pub sel: Range<usize>,
}

/// One list item line: indentation, marker (with its trailing space) and where the content starts.
#[derive(Clone, Debug, PartialEq)]
struct Item {
    indent: String,
    /// `None` for bullets, the number for ordered items.
    number: Option<(u64, char)>,
    bullet: char,
    task: bool,
    /// Byte length of indent + marker (+ task box) in the line.
    prefix_len: usize,
}

fn parse_item(line: &str) -> Option<Item> {
    let indent_len = line.len() - line.trim_start_matches([' ', '\t']).len();
    let (indent, rest) = line.split_at(indent_len);
    let (number, bullet, marker_len) = if let Some(c) = rest.chars().next().filter(|c| matches!(c, '-' | '*' | '+')) {
        if !rest[1..].starts_with(' ') {
            return None;
        }
        (None, c, 2)
    } else {
        let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        if digits == 0 || digits > 9 {
            return None;
        }
        let delim = rest[digits..].chars().next().filter(|c| matches!(c, '.' | ')'))?;
        if !rest[digits + 1..].starts_with(' ') {
            return None;
        }
        (Some((rest[..digits].parse().ok()?, delim)), '-', digits + 2)
    };
    let after = &rest[marker_len..];
    let task = number.is_none() && (after.starts_with("[ ] ") || after.starts_with("[x] ") || after.starts_with("[X] ") || after == "[ ]" || after == "[x]");
    let task_len = if task { after.len().min(4) } else { 0 };
    Some(Item { indent: indent.to_string(), number, bullet, task, prefix_len: indent_len + marker_len + task_len })
}

fn line_bounds(text: &str, off: usize) -> Range<usize> {
    let start = text[..off].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let end = text[off..].find('\n').map(|i| off + i).unwrap_or(text.len());
    start..end
}

/// Enter inside a list item: continue the list (next number, an empty task box), or on an empty item
/// end the list by clearing its marker. `None` means a plain newline.
pub fn enter(text: &str, sel: &Range<usize>) -> Option<Edit> {
    if !sel.is_empty() || sel.end > text.len() || !text.is_char_boundary(sel.end) {
        return None;
    }
    let caret = sel.end;
    let line = line_bounds(text, caret);
    let item = parse_item(&text[line.clone()])?;
    if caret < line.start + item.prefix_len {
        return None;
    }
    if text[line.start + item.prefix_len..line.end].trim().is_empty() {
        // Empty item: end the list, leaving an empty line.
        return Some(Edit { range: line.start..line.end, text: String::new(), sel: line.start..line.start });
    }
    let marker = match item.number {
        Some((n, d)) => format!("{}{d} ", n + 1),
        None => format!("{} ", item.bullet),
    };
    let insert = format!("\n{}{}{}", item.indent, marker, if item.task { "[ ] " } else { "" });
    let at = caret + insert.len();
    Some(Edit { range: caret..caret, text: insert, sel: at..at })
}

const INDENT: &str = "  ";

/// Tab / Shift+Tab on list items: indent or outdent every line the selection touches.
/// `None` when the caret's line is not a list item (Tab then inserts spaces as usual).
pub fn indent(text: &str, sel: &Range<usize>, outdent: bool) -> Option<Edit> {
    if sel.end > text.len() || !text.is_char_boundary(sel.start) || !text.is_char_boundary(sel.end) {
        return None;
    }
    let first = line_bounds(text, sel.start).start;
    let last = line_bounds(text, sel.end).end;
    let block = &text[first..last];
    parse_item(&text[line_bounds(text, sel.start)])?;
    let mut out = String::with_capacity(block.len() + 8);
    let (mut d_start, mut d_end) = (0isize, 0isize);
    let mut pos = first;
    for (i, l) in block.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let delta: isize = if outdent {
            let n = l.len() - l.trim_start_matches(' ').len();
            -(n.min(INDENT.len()) as isize)
        } else if l.is_empty() {
            0
        } else {
            INDENT.len() as isize
        };
        if delta >= 0 {
            out.push_str(&INDENT[..delta as usize]);
            out.push_str(l);
        } else {
            out.push_str(&l[(-delta) as usize..]);
        }
        // Shift the selection ends by the change on lines at or before them.
        let shift = |off: usize, acc: &mut isize| {
            if pos <= off {
                *acc += if delta < 0 { delta.max(pos as isize - off as isize) } else { delta };
            }
        };
        shift(sel.start, &mut d_start);
        shift(sel.end, &mut d_end);
        pos += l.len() + 1;
    }
    if out == block {
        return None;
    }
    let s = (sel.start as isize + d_start).max(first as isize) as usize;
    let e = (sel.end as isize + d_end).max(s as isize) as usize;
    Some(Edit { range: first..last, text: out, sel: s..e })
}

/// Wrap the selection in `left`/`right` (Cmd+B `**`, Cmd+I `_`). With no selection, insert both and
/// put the caret between them.
pub fn wrap(text: &str, sel: &Range<usize>, left: &str, right: &str) -> Edit {
    let inner = text.get(sel.clone()).unwrap_or("");
    let start = sel.start + left.len();
    Edit { range: sel.clone(), text: format!("{left}{inner}{right}"), sel: start..start + inner.len() }
}

/// Cmd+K: `[selection](url)` with `url` selected; with no selection `[](url)` and the caret in the brackets.
pub fn link(text: &str, sel: &Range<usize>) -> Edit {
    let inner = text.get(sel.clone()).unwrap_or("");
    let new_sel = if inner.is_empty() {
        sel.start + 1..sel.start + 1
    } else {
        let u = sel.start + inner.len() + 3;
        u..u + 3
    };
    Edit { range: sel.clone(), text: format!("[{inner}](url)"), sel: new_sel }
}

#[cfg(test)]
pub fn apply(text: &str, e: &Edit) -> String {
    format!("{}{}{}", &text[..e.range.start], e.text, &text[e.range.end..])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enter_at_end(t: &str) -> Option<(String, usize)> {
        let e = enter(t, &(t.len()..t.len()))?;
        Some((apply(t, &e), e.sel.start))
    }

    #[test]
    fn continues_bullets_numbers_and_tasks() {
        assert_eq!(enter_at_end("- a").unwrap().0, "- a\n- ");
        assert_eq!(enter_at_end("* a").unwrap().0, "* a\n* ");
        assert_eq!(enter_at_end("x\n  - nested").unwrap().0, "x\n  - nested\n  - ");
        assert_eq!(enter_at_end("1. one").unwrap().0, "1. one\n2. ");
        assert_eq!(enter_at_end("9) nine").unwrap().0, "9) nine\n10) ");
        assert_eq!(enter_at_end("- [ ] todo").unwrap().0, "- [ ] todo\n- [ ] ");
        assert_eq!(enter_at_end("- [x] done").unwrap().0, "- [x] done\n- [ ] ");
        let (t, caret) = enter_at_end("- a").unwrap();
        assert_eq!(caret, t.len());
    }

    #[test]
    fn empty_item_ends_the_list() {
        assert_eq!(enter_at_end("- a\n- ").unwrap(), ("- a\n".to_string(), 4));
        assert_eq!(enter_at_end("1. a\n2. ").unwrap().0, "1. a\n");
        assert_eq!(enter_at_end("- [ ] ").unwrap().0, "");
    }

    #[test]
    fn plain_lines_and_selections_get_a_plain_newline() {
        assert_eq!(enter_at_end("hello"), None);
        assert_eq!(enter_at_end("-not a list"), None);
        assert_eq!(enter_at_end("2026. year"), Some(("2026. year\n2027. ".into(), 17)));
        assert_eq!(enter("- a", &(0..1)), None);
        // Caret inside the marker: plain newline.
        assert_eq!(enter("  - a", &(1..1)), None);
    }

    #[test]
    fn splits_an_item_in_the_middle() {
        let t = "- ab";
        let e = enter(t, &(3..3)).unwrap();
        assert_eq!(apply(t, &e), "- a\n- b");
        assert_eq!(e.sel, 6..6);
    }

    #[test]
    fn indents_and_outdents_list_items() {
        let t = "- a\n- b";
        let e = indent(t, &(5..7), false).unwrap();
        assert_eq!(apply(t, &e), "- a\n  - b");
        assert_eq!(e.sel, 7..9);
        let t2 = apply(t, &e);
        let e = indent(&t2, &(9..9), true).unwrap();
        assert_eq!(apply(&t2, &e), "- a\n- b");
        assert_eq!(e.sel, 7..7);
        // Both lines.
        let e = indent(t, &(0..7), false).unwrap();
        assert_eq!(apply(t, &e), "  - a\n  - b");
        assert_eq!(e.sel, 2..11);
        // Not a list: Tab falls back to spaces, Shift+Tab does nothing.
        assert_eq!(indent("text", &(2..2), false), None);
        // Outdent at column 0 changes nothing.
        assert_eq!(indent("- a", &(3..3), true), None);
    }

    #[test]
    fn wraps_selection_or_inserts_pairs() {
        let t = "make bold";
        let e = wrap(t, &(5..9), "**", "**");
        assert_eq!(apply(t, &e), "make **bold**");
        assert_eq!(&apply(t, &e)[e.sel.clone()], "bold");
        let e = wrap(t, &(4..4), "_", "_");
        assert_eq!(apply(t, &e), "make__ bold");
        assert_eq!(e.sel, 5..5);
        let e = link(t, &(5..9));
        let out = apply(t, &e);
        assert_eq!(out, "make [bold](url)");
        assert_eq!(&out[e.sel.clone()], "url");
        let e = link(t, &(0..0));
        assert_eq!(apply(t, &e), "[](url)make bold");
        assert_eq!(e.sel, 1..1);
    }
}
