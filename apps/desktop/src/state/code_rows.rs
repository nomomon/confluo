//! Flat row model of the Code tab: files, hunks and lines as one virtualized list.

use std::collections::HashSet;

use gv_engine::{ChangedFile, DiffLine, DiffSide, InlineIndex, LineKind};

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
    /// Side-by-side row: (file, hunk, old side line, new side line).
    Pair(usize, usize, Option<usize>, Option<usize>),
    Notice(usize, Notice),
    /// Inline review thread: (file, index into `detail.threads`).
    Thread(usize, usize),
    /// New-comment composer under the draft's anchor line.
    Composer(usize),
    /// "Outdated comments" label: (file, count).
    Outdated(usize, usize),
    /// Rounded bottom edge of a file card.
    End(usize),
}

impl Row {
    pub fn file(self) -> usize {
        match self {
            Row::Header(f)
            | Row::Hunk(f, _)
            | Row::Line(f, _, _)
            | Row::Pair(f, _, _, _)
            | Row::Notice(f, _)
            | Row::Thread(f, _)
            | Row::Composer(f)
            | Row::Outdated(f, _)
            | Row::End(f) => f,
        }
    }

    /// The diff line a cursor on this row stands on (the new side of a pair when it has one).
    pub fn line(self) -> Option<(usize, usize, usize)> {
        match self {
            Row::Line(f, h, l) => Some((f, h, l)),
            Row::Pair(f, h, a, b) => b.or(a).map(|l| (f, h, l)),
            _ => None,
        }
    }

    pub fn has_line(self, fi: usize, hi: usize, li: usize) -> bool {
        match self {
            Row::Line(f, h, l) => (f, h, l) == (fi, hi, li),
            Row::Pair(f, h, a, b) => (f, h) == (fi, hi) && (a == Some(li) || b == Some(li)),
            _ => false,
        }
    }
}

/// Inline comment input to the row model: placed threads and the open composer's anchor.
#[derive(Default, Clone, Copy)]
pub struct InlineRows<'a> {
    pub index: Option<&'a InlineIndex>,
    /// (file, side, line) of the open composer.
    pub draft: Option<(usize, DiffSide, u32)>,
    /// Side-by-side rows instead of unified lines.
    pub split: bool,
}

/// The side and number a new comment on this line anchors to: removed lines the old side, else the new side.
pub fn line_anchor(l: &DiffLine) -> Option<(DiffSide, u32)> {
    match l.kind {
        LineKind::Del => l.old_no.map(|n| (DiffSide::Old, n)),
        _ => l.new_no.map(|n| (DiffSide::New, n)),
    }
}

/// Threads shown under one diff line (both sides for context lines).
pub fn line_threads<'a>(
    ix: &'a InlineIndex,
    path: &str,
    l: &DiffLine,
) -> impl Iterator<Item = usize> + 'a {
    let old = l
        .old_no
        .map(|n| ix.at(path, DiffSide::Old, n))
        .unwrap_or(&[]);
    let new = l
        .new_no
        .map(|n| ix.at(path, DiffSide::New, n))
        .unwrap_or(&[]);
    old.iter().chain(new.iter()).copied()
}

/// Rows of one file given its collapse state.
pub fn file_rows(
    fi: usize,
    f: &ChangedFile,
    collapsed: bool,
    show_large: bool,
    inline: InlineRows,
) -> Vec<Row> {
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
                let after = |rows: &mut Vec<Row>, li: usize| {
                    let l = &h.lines[li];
                    if let Some(ix) = inline.index {
                        rows.extend(line_threads(ix, f.path(), l).map(|t| Row::Thread(fi, t)));
                    }
                    if inline
                        .draft
                        .is_some_and(|(df, side, n)| df == fi && line_anchor(l) == Some((side, n)))
                    {
                        rows.push(Row::Composer(fi));
                    }
                };
                if inline.split {
                    for (a, b) in super::split::pair_lines(&h.lines) {
                        rows.push(Row::Pair(fi, hi, a, b));
                        let mut lis: Vec<usize> = a.into_iter().chain(b).collect();
                        lis.dedup();
                        lis.into_iter().for_each(|li| after(&mut rows, li));
                    }
                } else {
                    for li in 0..h.lines.len() {
                        rows.push(Row::Line(fi, hi, li));
                        after(&mut rows, li);
                    }
                }
            }
            if let Some(old) = inline
                .index
                .and_then(|ix| ix.outdated.get(f.path()))
                .filter(|o| !o.is_empty())
            {
                rows.push(Row::Outdated(fi, old.len()));
                rows.extend(old.iter().map(|t| Row::Thread(fi, *t)));
            }
        }
    }
    rows.push(Row::End(fi));
    rows
}

/// All rows plus the index of each file's header row.
pub fn build_rows(
    files: &[ChangedFile],
    collapsed: &HashSet<usize>,
    shown_large: &HashSet<usize>,
    inline: InlineRows,
) -> (Vec<Row>, Vec<usize>) {
    let mut rows = vec![];
    let mut starts = vec![];
    for (fi, f) in files.iter().enumerate() {
        starts.push(rows.len());
        rows.extend(file_rows(
            fi,
            f,
            collapsed.contains(&fi),
            shown_large.contains(&fi),
            inline,
        ));
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
        ChangedFile::from_patch(
            "a",
            "a",
            FileStatus::Added,
            &format!("@@ -0,0 +1,{lines} @@\n{body}"),
        )
    }

    #[test]
    fn rows_for_files() {
        let files = vec![file(2), file(LARGE_FILE_LINES + 1)];
        let (rows, starts) = build_rows(
            &files,
            &HashSet::new(),
            &HashSet::new(),
            InlineRows::default(),
        );
        assert_eq!(starts, vec![0, 5]);
        assert_eq!(
            rows[..5],
            [
                Row::Header(0),
                Row::Hunk(0, 0),
                Row::Line(0, 0, 0),
                Row::Line(0, 0, 1),
                Row::End(0)
            ]
        );
        assert_eq!(rows[6], Row::Notice(1, Notice::Large(LARGE_FILE_LINES + 1)));
        let (rows, _) = build_rows(
            &files,
            &HashSet::from([0]),
            &HashSet::from([1]),
            InlineRows::default(),
        );
        assert_eq!(rows[..2], [Row::Header(0), Row::End(0)]);
        assert_eq!(rows.len(), 2 + 1 + 1 + LARGE_FILE_LINES + 1 + 1);
        assert!(patch_text(&files[..1]).contains("+l1\n"));
    }

    #[test]
    fn threads_and_composer_rows() {
        let mut t = gv_engine::Thread::conversation("t", vec![]);
        t.position = Some(gv_engine::DiffPosition::new("a", DiffSide::New, 2));
        let mut gone = t.clone();
        gone.position = Some(gv_engine::DiffPosition::new("a", DiffSide::New, 99));
        let files = vec![file(2)];
        let ix = gv_engine::index_inline_threads(&[t, gone], &files);
        let inline = InlineRows {
            index: Some(&ix),
            draft: Some((0, DiffSide::New, 1)),
            split: false,
        };
        let (rows, _) = build_rows(&files, &HashSet::new(), &HashSet::new(), inline);
        assert_eq!(
            rows,
            [
                Row::Header(0),
                Row::Hunk(0, 0),
                Row::Line(0, 0, 0),
                Row::Composer(0),
                Row::Line(0, 0, 1),
                Row::Thread(0, 0),
                Row::Outdated(0, 1),
                Row::Thread(0, 1),
                Row::End(0)
            ]
        );
    }
}
