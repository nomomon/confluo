//! Line selection model shared by the diff and the whole-file viewer.
//!
//! A selection is an anchor and a head over one contiguous run of lines. In the diff the
//! run is one hunk of one file, so a selection never spans files.

use gv_engine::{ChangedFile, DiffLine, DiffSide};

use super::code_rows::line_anchor;

/// Anchor and head indices into one contiguous run of lines.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LineRange {
    pub anchor: usize,
    pub head: usize,
}

impl LineRange {
    pub fn at(i: usize) -> Self {
        Self { anchor: i, head: i }
    }
    pub fn lo(self) -> usize {
        self.anchor.min(self.head)
    }
    pub fn hi(self) -> usize {
        self.anchor.max(self.head)
    }
    pub fn contains(self, i: usize) -> bool {
        (self.lo()..=self.hi()).contains(&i)
    }
    /// Move the head to `i`, clamped to `0..len`.
    pub fn extend_to(&mut self, i: usize, len: usize) {
        self.head = i.min(len.saturating_sub(1));
    }
}

/// Selected lines of hunk `hi` in file `fi`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DiffSel {
    pub fi: usize,
    pub hi: usize,
    pub range: LineRange,
}

impl DiffSel {
    pub fn single(fi: usize, hi: usize, li: usize) -> Self {
        Self {
            fi,
            hi,
            range: LineRange::at(li),
        }
    }

    pub fn covers(&self, fi: usize, hi: usize, li: usize) -> bool {
        self.fi == fi && self.hi == hi && self.range.contains(li)
    }

    /// Extend toward a line; lines past the anchor's hunk (or file) clamp to its edge.
    pub fn extend_to(&mut self, fi: usize, hi: usize, li: usize, hunk_len: usize) {
        let target = match (fi, hi).cmp(&(self.fi, self.hi)) {
            std::cmp::Ordering::Equal => li,
            std::cmp::Ordering::Less => 0,
            std::cmp::Ordering::Greater => hunk_len.saturating_sub(1),
        };
        self.range.extend_to(target, hunk_len);
    }

    pub fn lines<'a>(&self, f: &'a ChangedFile) -> &'a [DiffLine] {
        f.hunks
            .get(self.hi)
            .and_then(|h| h.lines.get(self.range.lo()..=self.range.hi()))
            .unwrap_or(&[])
    }
}

/// Code text of the lines, without +/- markers or line numbers.
pub fn copy_text<'a>(lines: impl IntoIterator<Item = &'a str>) -> String {
    lines.into_iter().collect::<Vec<_>>().join("\n")
}

/// Comment anchor for a run of diff lines: side and number of the last line, the first line
/// on that side when it differs, and the other side's number for a trailing context line.
pub fn comment_span(lines: &[DiffLine]) -> Option<(DiffSide, Option<u32>, u32, Option<u32>)> {
    let last = lines.iter().rev().find(|l| line_anchor(l).is_some())?;
    let (side, n) = line_anchor(last)?;
    let first = lines
        .iter()
        .filter_map(line_anchor)
        .find(|a| a.0 == side)
        .map(|a| a.1)
        .unwrap_or(n);
    let other = (side == DiffSide::New && last.kind == gv_engine::LineKind::Context)
        .then_some(last.old_no)
        .flatten();
    Some((side, (first < n).then_some(first), n, other))
}

/// First and last selected line as (side, number), for side-aware links.
pub fn endpoints(lines: &[DiffLine]) -> Option<((DiffSide, u32), (DiffSide, u32))> {
    Some((
        lines.iter().find_map(line_anchor)?,
        lines.iter().rev().find_map(line_anchor)?,
    ))
}

/// New-side line numbers covered by the lines, if any.
pub fn new_span(lines: &[DiffLine]) -> Option<(u32, u32)> {
    Some((
        lines.iter().find_map(|l| l.new_no)?,
        lines.iter().rev().find_map(|l| l.new_no)?,
    ))
}

/// Dev only: `CONFLUO_SELECT=path:start-end` (new-side numbers) to preselect lines.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PendingSelect {
    pub path: String,
    pub start: u32,
    pub end: u32,
}

impl PendingSelect {
    pub fn parse(s: &str) -> Option<Self> {
        let (path, span) = s.rsplit_once(':')?;
        let (a, b) = span.split_once('-').unwrap_or((span, span));
        let (start, end) = (a.trim().parse().ok()?, b.trim().parse().ok()?);
        Some(Self {
            path: path.to_string(),
            start,
            end,
        })
    }

    /// (hunk, first line, last line) of the new-side range in `f`, clamped to the starting hunk.
    pub fn locate(&self, f: &ChangedFile) -> Option<(usize, usize, usize)> {
        f.hunks.iter().enumerate().find_map(|(hi, h)| {
            let ix = |n: u32| h.lines.iter().position(|l| l.new_no == Some(n));
            let lo = ix(self.start)?;
            let hi_li = (self.start..=self.end).rev().find_map(ix).unwrap_or(lo);
            Some((hi, lo, hi_li))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gv_engine::FileStatus;

    fn file() -> ChangedFile {
        ChangedFile::from_patch(
            "a.rs",
            "a.rs",
            FileStatus::Modified,
            "@@ -1,3 +1,3 @@\n ctx\n-old\n+new\n end\n@@ -10,1 +10,2 @@\n x\n+y\n",
        )
    }

    #[test]
    fn extend_clamps_to_hunk_and_file() {
        let mut s = DiffSel::single(0, 0, 1);
        s.extend_to(0, 0, 3, 4);
        assert_eq!((s.range.lo(), s.range.hi()), (1, 3));
        s.extend_to(0, 1, 0, 4);
        assert_eq!(s.range.head, 3);
        s.extend_to(1, 0, 2, 4);
        assert_eq!(s.range.head, 3);
        s.extend_to(0, 0, 0, 4);
        assert_eq!((s.range.lo(), s.range.hi()), (0, 1));
        let mut r = LineRange::at(2);
        r.extend_to(99, 5);
        assert_eq!(r.head, 4);
        assert!(s.covers(0, 0, 1) && !s.covers(0, 1, 1) && !s.covers(1, 0, 0));
    }

    #[test]
    fn copy_text_and_comment_span() {
        let f = file();
        let s = DiffSel {
            fi: 0,
            hi: 0,
            range: LineRange { anchor: 3, head: 0 },
        };
        let lines = s.lines(&f);
        assert_eq!(
            copy_text(lines.iter().map(|l| l.text.as_str())),
            "ctx\nold\nnew\nend"
        );
        assert_eq!(
            comment_span(lines),
            Some((DiffSide::New, Some(1), 3, Some(3)))
        );
        assert_eq!(
            comment_span(&lines[1..2]),
            Some((DiffSide::Old, None, 2, None))
        );
        assert_eq!(
            endpoints(&lines[1..3]),
            Some(((DiffSide::Old, 2), (DiffSide::New, 2)))
        );
        assert_eq!(new_span(&lines[1..3]), Some((2, 2)));
    }

    #[test]
    fn pending_select() {
        let p = PendingSelect::parse("src/a.rs:10-11").unwrap();
        assert_eq!(
            p,
            PendingSelect {
                path: "src/a.rs".into(),
                start: 10,
                end: 11
            }
        );
        assert_eq!(
            PendingSelect::parse("a.rs:2").map(|p| (p.start, p.end)),
            Some((2, 2))
        );
        assert_eq!(p.locate(&file()), Some((1, 0, 1)));
        assert_eq!(PendingSelect::parse("nope"), None);
    }
}
