//! One line of code in the mono font, syntax colored; unwrapped unless `wrap`, with optional
//! find-match highlights (`marks`, byte ranges of the original text, the current one stronger).

use std::ops::Range;

use gpui::{div, prelude::*, App, HighlightStyle, SharedString, StyledText, Window};

use super::lang::Lang;
use super::tokenize::{tokenize, Tok};
use crate::ui::theme;

#[derive(IntoElement)]
pub struct CodeText {
    text: SharedString,
    lang: Lang,
    marks: Vec<(Range<usize>, bool)>,
    wrap: bool,
}

impl CodeText {
    pub fn new(text: impl Into<SharedString>, lang: Lang) -> Self {
        Self {
            text: text.into(),
            lang,
            marks: vec![],
            wrap: false,
        }
    }

    /// Find matches: byte ranges in the text as given, `true` for the current match.
    pub fn marks(mut self, marks: Vec<(Range<usize>, bool)>) -> Self {
        self.marks = marks;
        self
    }

    /// Wrap long lines to the available width.
    pub fn wrap(mut self, on: bool) -> Self {
        self.wrap = on;
        self
    }
}

fn color(t: Tok) -> gpui::Hsla {
    match t {
        Tok::Keyword => theme::syntax_keyword(),
        Tok::String => theme::syntax_string(),
        Tok::Comment => theme::syntax_comment(),
        Tok::Number => theme::syntax_number(),
        Tok::Type => theme::syntax_type(),
    }
}

/// Byte offset in `text` after each tab became four spaces.
fn expanded(text: &str, off: usize) -> usize {
    off + 3 * text[..off.min(text.len())].matches('\t').count()
}

/// Non-overlapping, ordered segments carrying the color of the token and the mark covering them.
pub fn merge<C: Copy, M: Copy>(
    tokens: &[(Range<usize>, C)],
    marks: &[(Range<usize>, M)],
) -> Vec<(Range<usize>, Option<C>, Option<M>)> {
    let mut cuts: Vec<usize> = tokens
        .iter()
        .map(|t| &t.0)
        .chain(marks.iter().map(|m| &m.0))
        .flat_map(|r| [r.start, r.end])
        .collect();
    cuts.sort_unstable();
    cuts.dedup();
    fn at<T: Copy>(rs: &[(Range<usize>, T)], a: usize) -> Option<T> {
        rs.iter().find(|(r, _)| r.contains(&a)).map(|(_, t)| *t)
    }
    cuts.windows(2)
        .filter_map(|w| {
            let c = at(tokens, w[0]);
            let m = at(marks, w[0]);
            (c.is_some() || m.is_some()).then(|| (w[0]..w[1], c, m))
        })
        .collect()
}

impl RenderOnce for CodeText {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        // Tabs render as zero width in GPUI; expand them so indentation shows.
        let marks: Vec<(Range<usize>, bool)> = self
            .marks
            .iter()
            .map(|(r, cur)| {
                (
                    expanded(&self.text, r.start)..expanded(&self.text, r.end),
                    *cur,
                )
            })
            .collect();
        let text: SharedString = if self.text.contains('\t') {
            self.text.replace('\t', "    ").into()
        } else {
            self.text
        };
        let tokens: Vec<_> = tokenize(&text, self.lang)
            .into_iter()
            .map(|(r, t)| (r, color(t)))
            .collect();
        let hl: Vec<_> = merge(&tokens, &marks)
            .into_iter()
            .map(|(r, c, m)| {
                let bg = m.map(|cur| {
                    if cur {
                        theme::find_current_bg()
                    } else {
                        theme::find_match_bg()
                    }
                });
                (
                    r,
                    HighlightStyle {
                        color: c,
                        background_color: bg,
                        ..Default::default()
                    },
                )
            })
            .collect();
        div()
            .map(|d| {
                if self.wrap {
                    d.w_full().min_w(theme::space_0())
                } else {
                    d.flex_shrink_0().whitespace_nowrap()
                }
            })
            .font_family(theme::mono_font())
            .child(StyledText::new(text).with_highlights(hl))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_split_tokens_without_overlap() {
        // Token 0..6 colored 1, a mark over 4..9 (current), token 8..10 colored 2.
        let m = merge(&[(0..6, 1), (8..10, 2)], &[(4..9, true)]);
        assert_eq!(
            m,
            [
                (0..4, Some(1), None),
                (4..6, Some(1), Some(true)),
                (6..8, None, Some(true)),
                (8..9, Some(2), Some(true)),
                (9..10, Some(2), None),
            ]
        );
        assert_eq!(merge::<u8, bool>(&[], &[]), []);
    }

    #[test]
    fn tabs_shift_mark_offsets() {
        assert_eq!(expanded("\tab", 1), 4);
        assert_eq!(expanded("a\t\tb", 3), 9);
        assert_eq!(expanded("ab", 2), 2);
    }
}
