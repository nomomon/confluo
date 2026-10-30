//! Changed files of a PR/MR and a unified-diff parser.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LineKind {
    Context,
    Add,
    Del,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffLine {
    pub kind: LineKind,
    pub old_no: Option<u32>,
    pub new_no: Option<u32>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hunk {
    pub header: String,
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangedFile {
    pub old_path: String,
    pub new_path: String,
    pub status: FileStatus,
    pub additions: u32,
    pub deletions: u32,
    pub binary: bool,
    pub too_large: bool,
    pub hunks: Vec<Hunk>,
}

impl ChangedFile {
    /// Build a file from its patch text; counts come from the parsed lines.
    pub fn from_patch(old_path: &str, new_path: &str, status: FileStatus, patch: &str) -> Self {
        let hunks = parse_unified(patch);
        let (additions, deletions) = count(&hunks);
        Self {
            old_path: old_path.into(),
            new_path: new_path.into(),
            status,
            additions,
            deletions,
            binary: patch.contains("Binary files ") && hunks.is_empty(),
            too_large: false,
            hunks,
        }
    }

    pub fn path(&self) -> &str {
        if self.status == FileStatus::Deleted {
            &self.old_path
        } else {
            &self.new_path
        }
    }

    pub fn line_count(&self) -> usize {
        self.hunks.iter().map(|h| h.lines.len()).sum()
    }
}

fn count(hunks: &[Hunk]) -> (u32, u32) {
    let mut a = 0;
    let mut d = 0;
    for l in hunks.iter().flat_map(|h| &h.lines) {
        match l.kind {
            LineKind::Add => a += 1,
            LineKind::Del => d += 1,
            LineKind::Context => {}
        }
    }
    (a, d)
}

/// Sum of additions and deletions over files.
pub fn totals(files: &[ChangedFile]) -> (u32, u32) {
    files
        .iter()
        .fold((0, 0), |(a, d), f| (a + f.additions, d + f.deletions))
}

fn range(s: &str) -> (u32, u32) {
    let s = s.trim_start_matches(['-', '+']);
    match s.split_once(',') {
        Some((a, b)) => (a.parse().unwrap_or(0), b.parse().unwrap_or(0)),
        None => (s.parse().unwrap_or(0), 1),
    }
}

/// Parse unified diff text (file headers optional) into hunks.
pub fn parse_unified(patch: &str) -> Vec<Hunk> {
    let mut hunks: Vec<Hunk> = Vec::new();
    let (mut old, mut new) = (0u32, 0u32);
    for raw in patch.lines() {
        if let Some(rest) = raw.strip_prefix("@@") {
            let Some((ranges, _)) = rest.split_once("@@") else {
                continue;
            };
            let mut parts = ranges.split_whitespace();
            let (old_start, old_lines) = range(parts.next().unwrap_or("-0"));
            let (new_start, new_lines) = range(parts.next().unwrap_or("+0"));
            old = old_start;
            new = new_start;
            let header = format!("@@{rest}");
            hunks.push(Hunk {
                header,
                old_start,
                old_lines,
                new_start,
                new_lines,
                lines: Vec::new(),
            });
            continue;
        }
        let Some(h) = hunks.last_mut() else { continue };
        if raw.starts_with('\\') {
            continue;
        }
        let (kind, text) = match raw.chars().next() {
            Some('+') => (LineKind::Add, &raw[1..]),
            Some('-') => (LineKind::Del, &raw[1..]),
            Some(' ') => (LineKind::Context, &raw[1..]),
            None => (LineKind::Context, ""),
            _ => continue,
        };
        let line = match kind {
            LineKind::Add => {
                new += 1;
                DiffLine {
                    kind,
                    old_no: None,
                    new_no: Some(new - 1),
                    text: text.into(),
                }
            }
            LineKind::Del => {
                old += 1;
                DiffLine {
                    kind,
                    old_no: Some(old - 1),
                    new_no: None,
                    text: text.into(),
                }
            }
            LineKind::Context => {
                old += 1;
                new += 1;
                DiffLine {
                    kind,
                    old_no: Some(old - 1),
                    new_no: Some(new - 1),
                    text: text.into(),
                }
            }
        };
        h.lines.push(line);
    }
    hunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_patch() {
        assert!(parse_unified("").is_empty());
        let f = ChangedFile::from_patch("a", "a", FileStatus::Modified, "");
        assert_eq!((f.additions, f.deletions), (0, 0));
    }

    #[test]
    fn basic_hunk_numbers() {
        let p = "@@ -100,3 +100,4 @@ fn main()\n ctx\n-old\n+new\n+more\n ctx2\n";
        let h = &parse_unified(p)[0];
        assert_eq!(
            (h.old_start, h.old_lines, h.new_start, h.new_lines),
            (100, 3, 100, 4)
        );
        assert_eq!(h.header, "@@ -100,3 +100,4 @@ fn main()");
        assert_eq!(
            h.lines[1],
            DiffLine {
                kind: LineKind::Del,
                old_no: Some(101),
                new_no: None,
                text: "old".into()
            }
        );
        assert_eq!(h.lines[3].new_no, Some(102));
        assert_eq!(h.lines[4].old_no, Some(102));
        assert_eq!(h.lines[4].new_no, Some(103));
    }

    #[test]
    fn no_newline_marker_is_skipped() {
        let p = "@@ -1 +1 @@\n-a\n\\ No newline at end of file\n+b\n\\ No newline at end of file\n";
        let h = &parse_unified(p)[0];
        assert_eq!(h.lines.len(), 2);
        assert_eq!((h.old_lines, h.new_lines), (1, 1));
    }

    #[test]
    fn git_headers_and_rename() {
        let p = "diff --git a/x.rs b/y.rs\nsimilarity index 90%\nrename from x.rs\nrename to y.rs\n--- a/x.rs\n+++ b/y.rs\n@@ -1,2 +1,2 @@\n a\n-b\n+c\n";
        let f = ChangedFile::from_patch("x.rs", "y.rs", FileStatus::Renamed, p);
        assert_eq!(f.hunks.len(), 1);
        assert_eq!((f.additions, f.deletions), (1, 1));
        assert_eq!(f.path(), "y.rs");
        assert_eq!(f.line_count(), 3);
    }

    #[test]
    fn multiple_hunks_and_totals() {
        let p = "@@ -1,1 +1,1 @@\n-a\n+b\n@@ -10,0 +11,2 @@\n+c\n+d\n";
        let f = ChangedFile::from_patch("f", "f", FileStatus::Modified, p);
        assert_eq!(f.hunks.len(), 2);
        assert_eq!(f.hunks[1].lines[1].new_no, Some(12));
        assert_eq!(totals(&[f.clone(), f]), (6, 2));
    }
}
