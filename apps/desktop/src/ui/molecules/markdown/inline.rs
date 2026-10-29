//! Styled inline text: runs of plain, bold, italic, code, link and muted text.

use std::ops::Range;

#[derive(Clone, Default, PartialEq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
    /// Image alt text.
    pub muted: bool,
    pub link: Option<String>,
}

#[derive(Default)]
pub struct Inline {
    pub text: String,
    pub runs: Vec<(Range<usize>, Style)>,
}

impl Inline {
    pub fn push(&mut self, s: &str, style: Style) {
        if s.is_empty() {
            return;
        }
        let start = self.text.len();
        self.text.push_str(s);
        match self.runs.last_mut() {
            Some((r, last)) if *last == style && r.end == start => r.end = self.text.len(),
            _ => self.runs.push((start..self.text.len(), style)),
        }
    }

    pub fn is_blank(&self) -> bool {
        self.text.trim().is_empty()
    }

    /// Byte ranges and targets of links, in order.
    pub fn links(&self) -> Vec<(Range<usize>, String)> {
        self.runs.iter().filter_map(|(r, s)| s.link.clone().map(|l| (r.clone(), l))).collect()
    }
}
