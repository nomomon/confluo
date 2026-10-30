//! A diff row with both line counters, as GitLab identifies rows (`line_code`, `line_range`).

use serde::{Deserialize, Serialize};

use crate::{DiffSide, Hunk, LineKind};

/// One diff row: the old and new counters at that row and its kind. For an added row `old` is
/// the next old line (GitLab's counter there), and for a removed row `new` the next new line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RowRef {
    pub old: u32,
    pub new: u32,
    pub kind: LineKind,
}

/// The row of a file's `hunks` holding `line` on `side`.
pub fn row_ref(hunks: &[Hunk], side: DiffSide, line: u32) -> Option<RowRef> {
    for h in hunks {
        let (mut old, mut new) = (h.old_start, h.new_start);
        for l in &h.lines {
            let row = RowRef {
                old,
                new,
                kind: l.kind,
            };
            let hit = match side {
                DiffSide::New => l.kind != LineKind::Del && new == line,
                DiffSide::Old => l.kind != LineKind::Add && old == line,
            };
            if hit {
                return Some(row);
            }
            match l.kind {
                LineKind::Context => (old, new) = (old + 1, new + 1),
                LineKind::Add => new += 1,
                LineKind::Del => old += 1,
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_unified;

    #[test]
    fn counts_rows_like_gitlab() {
        let f = parse_unified("@@ -10,3 +10,4 @@\n a\n-b\n+c\n+d\n e\n");
        let r = |s, n| row_ref(&f, s, n).map(|r| (r.old, r.new, r.kind));
        assert_eq!(r(DiffSide::New, 10), Some((10, 10, LineKind::Context)));
        assert_eq!(r(DiffSide::Old, 11), Some((11, 11, LineKind::Del)));
        assert_eq!(r(DiffSide::New, 11), Some((12, 11, LineKind::Add)));
        assert_eq!(r(DiffSide::New, 12), Some((12, 12, LineKind::Add)));
        assert_eq!(r(DiffSide::New, 13), Some((12, 13, LineKind::Context)));
        assert_eq!(r(DiffSide::New, 99), None);
    }
}
