//! Lightweight markdown highlighting for `TextInput::markdown()` (pure): headings bold, list markers,
//! `code`, **bold**, _italic_ and links. Spans are sorted, non-overlapping byte ranges; the element
//! turns them into text runs (same font size, so caret stops stay exact).

use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mark {
    /// A heading line's text.
    Heading,
    /// `#`, list markers, task boxes, `>` and the `**` / `_` / backtick delimiters.
    Syntax,
    Code,
    Bold,
    Italic,
    Link,
}

pub fn spans(text: &str) -> Vec<(Range<usize>, Mark)> {
    let mut out = vec![];
    let mut base = 0;
    let mut fence = false;
    for line in text.split('\n') {
        let trimmed = line.trim_start();
        let ind = line.len() - trimmed.len();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fence = !fence;
            out.push((base..base + line.len(), Mark::Code));
        } else if fence {
            if !line.is_empty() {
                out.push((base..base + line.len(), Mark::Code));
            }
        } else {
            line_spans(line, ind, base, &mut out);
        }
        base += line.len() + 1;
    }
    out
}

fn line_spans(line: &str, ind: usize, base: usize, out: &mut Vec<(Range<usize>, Mark)>) {
    let rest = &line[ind..];
    let hashes = rest.len() - rest.trim_start_matches('#').len();
    if (1..=6).contains(&hashes) && (rest.len() == hashes || rest[hashes..].starts_with(' ')) {
        out.push((base + ind..base + ind + hashes, Mark::Syntax));
        if rest.len() > hashes {
            out.push((base + ind + hashes..base + line.len(), Mark::Heading));
        }
        return;
    }
    let mut start = ind;
    if rest.starts_with("> ") || rest == ">" {
        out.push((base + ind..base + ind + 1, Mark::Syntax));
        start = ind + 1;
    } else if let Some(n) = marker_len(rest) {
        out.push((base + ind..base + ind + n, Mark::Syntax));
        start = ind + n;
    }
    inline_spans(&line[start..], base + start, out);
}

/// Length of a list marker (and task box) at the start of `s`.
fn marker_len(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    let n = if matches!(b.first(), Some(b'-' | b'*' | b'+')) && b.get(1) == Some(&b' ') {
        2
    } else {
        let d = b.iter().take_while(|c| c.is_ascii_digit()).count();
        if d == 0 || d > 9 || !matches!(b.get(d), Some(b'.' | b')')) || b.get(d + 1) != Some(&b' ')
        {
            return None;
        }
        d + 2
    };
    let after = &s[n..];
    Some(
        if after.starts_with("[ ] ") || after.starts_with("[x] ") || after.starts_with("[X] ") {
            n + 4
        } else {
            n
        },
    )
}

fn inline_spans(s: &str, base: usize, out: &mut Vec<(Range<usize>, Mark)>) {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'`' => {
                if let Some(j) = s[i + 1..].find('`') {
                    let end = i + 1 + j + 1;
                    out.push((base + i..base + end, Mark::Code));
                    i = end;
                    continue;
                }
            }
            b'*' | b'_' if b.get(i + 1) == Some(&b[i]) => {
                let delim = &s[i..i + 2];
                if let Some(j) = s[i + 2..].find(delim).filter(|j| *j > 0) {
                    let end = i + 2 + j + 2;
                    out.push((base + i..base + i + 2, Mark::Syntax));
                    out.push((base + i + 2..base + end - 2, Mark::Bold));
                    out.push((base + end - 2..base + end, Mark::Syntax));
                    i = end;
                    continue;
                }
            }
            c @ (b'*' | b'_') => {
                // `_` inside words (snake_case) is not emphasis.
                let word_before = i > 0 && b[i - 1].is_ascii_alphanumeric();
                let next_ok = b.get(i + 1).is_some_and(|n| !n.is_ascii_whitespace());
                if !(c == b'_' && word_before) && next_ok {
                    if let Some(j) = s[i + 1..].find(c as char).filter(|j| *j > 0) {
                        let end = i + 1 + j + 1;
                        let word_after =
                            c == b'_' && b.get(end).is_some_and(|n| n.is_ascii_alphanumeric());
                        if !word_after {
                            out.push((base + i..base + i + 1, Mark::Syntax));
                            out.push((base + i + 1..base + end - 1, Mark::Italic));
                            out.push((base + end - 1..base + end, Mark::Syntax));
                            i = end;
                            continue;
                        }
                    }
                }
            }
            b'[' => {
                if let Some(close) = s[i..].find("](").map(|k| i + k) {
                    if let Some(paren) = s[close + 2..].find(')').map(|k| close + 2 + k) {
                        if !s[i + 1..close].contains(']') {
                            out.push((base + i..base + paren + 1, Mark::Link));
                            i = paren + 1;
                            continue;
                        }
                    }
                }
            }
            _ => {}
        }
        i += s[i..].chars().next().map(char::len_utf8).unwrap_or(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marks(t: &str) -> Vec<(&str, Mark)> {
        spans(t).into_iter().map(|(r, m)| (&t[r], m)).collect()
    }

    #[test]
    fn headings_lists_and_inline() {
        assert_eq!(
            marks("## Title"),
            vec![("##", Mark::Syntax), (" Title", Mark::Heading)]
        );
        assert_eq!(
            marks("- [ ] do `x`"),
            vec![("- [ ] ", Mark::Syntax), ("`x`", Mark::Code)]
        );
        assert_eq!(
            marks("1. **b** _i_"),
            vec![
                ("1. ", Mark::Syntax),
                ("**", Mark::Syntax),
                ("b", Mark::Bold),
                ("**", Mark::Syntax),
                ("_", Mark::Syntax),
                ("i", Mark::Italic),
                ("_", Mark::Syntax)
            ]
        );
        assert_eq!(
            marks("see [docs](http://x) now"),
            vec![("[docs](http://x)", Mark::Link)]
        );
        assert_eq!(marks("snake_case_name"), vec![]);
        assert_eq!(
            marks("```\nlet x;\n```"),
            vec![
                ("```", Mark::Code),
                ("let x;", Mark::Code),
                ("```", Mark::Code)
            ]
        );
        assert_eq!(
            marks("é *ü*"),
            vec![
                ("*", Mark::Syntax),
                ("ü", Mark::Italic),
                ("*", Mark::Syntax)
            ]
        );
    }

    #[test]
    fn spans_are_sorted_and_disjoint() {
        let t = "# h\n- a **b** `c` [d](e)\n> q _i_";
        let s = spans(t);
        assert!(s.windows(2).all(|w| w[0].0.end <= w[1].0.start));
        assert!(s
            .iter()
            .all(|(r, _)| t.is_char_boundary(r.start) && t.is_char_boundary(r.end)));
    }
}
