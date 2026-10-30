//! Find in diff (Cmd+F in Review mode): matches across every loaded file diff, the current one, and
//! stepping with wrap. Pure, so it is unit tested; `find_nav` reveals matches in the list.

use std::ops::Range;

use gv_engine::ChangedFile;

/// One occurrence: file, hunk, line and the byte range in the line text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub fi: usize,
    pub hi: usize,
    pub li: usize,
    pub range: Range<usize>,
}

/// Find bar state; matches are recomputed when the query, case or files change.
#[derive(Default)]
pub struct FindState {
    pub open: bool,
    pub case: bool,
    pub matches: Vec<Match>,
    /// Index into `matches`.
    pub current: usize,
}

impl FindState {
    /// Highlights on one diff line: ranges and whether each is the current match.
    pub fn marks(&self, fi: usize, hi: usize, li: usize) -> Vec<(Range<usize>, bool)> {
        if !self.open {
            return vec![];
        }
        let key = (fi, hi, li);
        let start = self.matches.partition_point(|m| (m.fi, m.hi, m.li) < key);
        self.matches[start..]
            .iter()
            .enumerate()
            .take_while(|(_, m)| (m.fi, m.hi, m.li) == key)
            .map(|(i, m)| (m.range.clone(), start + i == self.current))
            .collect()
    }

    /// "3 of 12", "No results" or empty for an empty query.
    pub fn count_label(&self, query: &str) -> String {
        match self.matches.len() {
            _ if query.is_empty() => String::new(),
            0 => "No results".into(),
            n => format!("{} of {n}", self.current + 1),
        }
    }

    /// Move by `delta` with wrap around.
    pub fn step(&mut self, delta: isize) {
        let n = self.matches.len() as isize;
        if n > 0 {
            self.current = (self.current as isize + delta).rem_euclid(n) as usize;
        }
    }
}

/// Byte ranges of `needle` in `hay` (non-overlapping); ASCII case folded unless `case`.
pub fn find_in_line(hay: &str, needle: &str, case: bool) -> Vec<Range<usize>> {
    let mut out = vec![];
    if needle.is_empty() || needle.len() > hay.len() {
        return out;
    }
    let (h, n) = (hay.as_bytes(), needle.as_bytes());
    let mut i = 0;
    while i + n.len() <= h.len() {
        let w = &h[i..i + n.len()];
        let hit = if case {
            w == n
        } else {
            w.eq_ignore_ascii_case(n)
        };
        if hit && hay.is_char_boundary(i) && hay.is_char_boundary(i + n.len()) {
            out.push(i..i + n.len());
            i += n.len();
        } else {
            i += 1;
        }
    }
    out
}

/// Every match in file, hunk and line order.
pub fn find_matches(files: &[ChangedFile], query: &str, case: bool) -> Vec<Match> {
    let mut out = vec![];
    for (fi, f) in files.iter().enumerate() {
        for (hi, h) in f.hunks.iter().enumerate() {
            for (li, l) in h.lines.iter().enumerate() {
                out.extend(
                    find_in_line(&l.text, query, case)
                        .into_iter()
                        .map(|range| Match { fi, hi, li, range }),
                );
            }
        }
    }
    out
}

/// The match to show first: the first at or after file `from` (the file in view), else the first.
pub fn first_from(matches: &[Match], from: usize) -> usize {
    matches.iter().position(|m| m.fi >= from).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gv_engine::FileStatus;

    fn file(path: &str, patch: &str) -> ChangedFile {
        ChangedFile::from_patch(path, path, FileStatus::Modified, patch)
    }

    #[test]
    fn line_matches_fold_ascii_case() {
        assert_eq!(
            find_in_line("Foo foo FOO", "foo", false),
            [0..3, 4..7, 8..11]
        );
        assert_eq!(
            find_in_line("Foo foo FOO", "foo", true).first(),
            Some(&(4..7))
        );
        assert_eq!(find_in_line("aaaa", "aa", true), [0..2, 2..4]);
        assert!(find_in_line("abc", "", false).is_empty());
        assert_eq!(find_in_line("é foo", "foo", false).first(), Some(&(3..6)));
    }

    #[test]
    fn matches_across_files_and_stepping() {
        let files = vec![
            file("a.rs", "@@ -1,2 +1,2 @@\n-let x = 1;\n+let x = 2;\n"),
            file(
                "b.rs",
                "@@ -1 +1 @@\n keep only\n@@ -9 +9 @@\n+let y = x;\n",
            ),
        ];
        let m = find_matches(&files, "LET", false);
        let at: Vec<_> = m.iter().map(|m| (m.fi, m.hi, m.li)).collect();
        assert_eq!(at, [(0, 0, 0), (0, 0, 1), (1, 1, 0)]);
        assert!(find_matches(&files, "LET", true).is_empty());
        let mut s = FindState {
            open: true,
            matches: find_matches(&files, "x", false),
            ..Default::default()
        };
        assert_eq!(s.count_label("x"), "1 of 3");
        assert_eq!(s.marks(1, 1, 0), [(8..9, false)]);
        s.step(-1);
        assert_eq!(s.count_label("x"), "3 of 3");
        assert_eq!(s.marks(1, 1, 0), [(8..9, true)]);
        s.step(1);
        assert_eq!(s.current, 0);
        assert_eq!(first_from(&s.matches, 1), 2);
        assert_eq!(first_from(&s.matches, 5), 0);
        assert_eq!(s.count_label(""), "");
    }
}
