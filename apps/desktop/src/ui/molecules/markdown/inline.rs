//! Styled inline text: runs of plain, bold, italic, code, link and image text.

use std::ops::Range;

#[derive(Clone, Default, PartialEq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
    /// Image alt text (the run's text) and its source URL.
    pub image: Option<String>,
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

    /// Text with bare `http(s)://` URLs turned into link runs (GitHub autolinks).
    pub fn push_autolinked(&mut self, s: &str, style: Style) {
        if style.link.is_some() || style.code || style.image.is_some() {
            return self.push(s, style);
        }
        let mut rest = s;
        while let Some(i) = rest.find("https://").or_else(|| rest.find("http://")) {
            self.push(&rest[..i], style.clone());
            let url_len = rest[i..]
                .find(|c: char| c.is_whitespace() || c == '<' || c == '>' || c == '"')
                .unwrap_or(rest.len() - i);
            let url = rest[i..i + url_len].trim_end_matches(['.', ',', ')', ';', ':', '!', '?']);
            self.push(
                url,
                Style {
                    link: Some(url.to_string()),
                    ..style.clone()
                },
            );
            rest = &rest[i + url.len()..];
        }
        self.push(rest, style);
    }

    pub fn is_blank(&self) -> bool {
        self.text.trim().is_empty() && !self.runs.iter().any(|(_, s)| s.image.is_some())
    }

    /// Byte ranges and targets of links, in order.
    #[cfg(test)]
    pub fn links(&self) -> Vec<(Range<usize>, String)> {
        self.runs
            .iter()
            .filter_map(|(r, s)| s.link.clone().map(|l| (r.clone(), l)))
            .collect()
    }

    /// Image source URLs, in order.
    #[cfg(test)]
    pub fn images(&self) -> impl Iterator<Item = &str> {
        self.runs.iter().filter_map(|(_, s)| s.image.as_deref())
    }
}
