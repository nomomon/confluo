//! Flat row model of the Code tab: files, hunks and lines as one virtualized list.

use std::collections::HashSet;

use gv_engine::ChangedFile;

/// Files with more diff lines than this start collapsed behind "Show diff".
pub const LARGE_FILE_LINES: usize = 400;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Notice {
    Binary,
    TooLarge,
    /// Hidden by default; carries the line count.
    Large(usize),
    Empty,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Row {
    /// File card header.
    Header(usize),
    /// `@@` band: (file, hunk).
    Hunk(usize, usize),
    /// (file, hunk, line).
    Line(usize, usize, usize),
    Notice(usize, Notice),
    /// Rounded bottom edge of a file card.
    End(usize),
}

impl Row {
    pub fn file(self) -> usize {
        match self {
            Row::Header(f) | Row::Hunk(f, _) | Row::Line(f, _, _) | Row::Notice(f, _) | Row::End(f) => f,
        }
    }
}

/// Rows of one file given its collapse state.
pub fn file_rows(fi: usize, f: &ChangedFile, collapsed: bool, show_large: bool) -> Vec<Row> {
    let mut rows = vec![Row::Header(fi)];
    if !collapsed {
        let lines = f.line_count();
        if f.binary {
            rows.push(Row::Notice(fi, Notice::Binary));
        } else if f.too_large {
            rows.push(Row::Notice(fi, Notice::TooLarge));
        } else if lines > LARGE_FILE_LINES && !show_large {
            rows.push(Row::Notice(fi, Notice::Large(lines)));
        } else if f.hunks.is_empty() {
            rows.push(Row::Notice(fi, Notice::Empty));
        } else {
            for (hi, h) in f.hunks.iter().enumerate() {
                rows.push(Row::Hunk(fi, hi));
                rows.extend((0..h.lines.len()).map(|li| Row::Line(fi, hi, li)));
            }
        }
    }
    rows.push(Row::End(fi));
    rows
}

/// All rows plus the index of each file's header row.
pub fn build_rows(files: &[ChangedFile], collapsed: &HashSet<usize>, shown_large: &HashSet<usize>) -> (Vec<Row>, Vec<usize>) {
    let mut rows = vec![];
    let mut starts = vec![];
    for (fi, f) in files.iter().enumerate() {
        starts.push(rows.len());
        rows.extend(file_rows(fi, f, collapsed.contains(&fi), shown_large.contains(&fi)));
    }
    (rows, starts)
}

/// Unified patch text of all files, for copying.
pub fn patch_text(files: &[ChangedFile]) -> String {
    let mut out = String::new();
    for f in files {
        out.push_str(&format!("--- a/{}\n+++ b/{}\n", f.old_path, f.new_path));
        for h in &f.hunks {
            out.push_str(&h.header);
            out.push('\n');
            for l in &h.lines {
                out.push(match l.kind {
                    gv_engine::LineKind::Add => '+',
                    gv_engine::LineKind::Del => '-',
                    gv_engine::LineKind::Context => ' ',
                });
                out.push_str(&l.text);
                out.push('\n');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use gv_engine::FileStatus;

    fn file(lines: usize) -> ChangedFile {
        let body: String = (0..lines).map(|i| format!("+l{i}\n")).collect();
        ChangedFile::from_patch("a", "a", FileStatus::Added, &format!("@@ -0,0 +1,{lines} @@\n{body}"))
    }

    #[test]
    fn rows_for_files() {
        let files = vec![file(2), file(LARGE_FILE_LINES + 1)];
        let (rows, starts) = build_rows(&files, &HashSet::new(), &HashSet::new());
        assert_eq!(starts, vec![0, 5]);
        assert_eq!(rows[..5], [Row::Header(0), Row::Hunk(0, 0), Row::Line(0, 0, 0), Row::Line(0, 0, 1), Row::End(0)]);
        assert_eq!(rows[6], Row::Notice(1, Notice::Large(LARGE_FILE_LINES + 1)));
        let (rows, _) = build_rows(&files, &HashSet::from([0]), &HashSet::from([1]));
        assert_eq!(rows[..2], [Row::Header(0), Row::End(0)]);
        assert_eq!(rows.len(), 2 + 1 + 1 + LARGE_FILE_LINES + 1 + 1);
        assert!(patch_text(&files[..1]).contains("+l1\n"));
    }
}
