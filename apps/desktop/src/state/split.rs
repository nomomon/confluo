//! Side-by-side diff: pair removed lines with added lines inside each change block.

use gv_engine::{DiffLine, LineKind};

/// One side-by-side row: (old side line index, new side line index) into the hunk.
pub type Pair = (Option<usize>, Option<usize>);

/// Context lines sit on both sides; a run of deletions and the additions after it pair up
/// row by row, the shorter side padded with empty cells.
pub fn pair_lines(lines: &[DiffLine]) -> Vec<Pair> {
    let mut out = vec![];
    let mut i = 0;
    while i < lines.len() {
        if lines[i].kind == LineKind::Context {
            out.push((Some(i), Some(i)));
            i += 1;
            continue;
        }
        let dels: Vec<usize> = (i..lines.len())
            .take_while(|j| lines[*j].kind == LineKind::Del)
            .collect();
        let after = i + dels.len();
        let adds: Vec<usize> = (after..lines.len())
            .take_while(|j| lines[*j].kind == LineKind::Add)
            .collect();
        for k in 0..dels.len().max(adds.len()) {
            out.push((dels.get(k).copied(), adds.get(k).copied()));
        }
        i = after + adds.len();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use gv_engine::{ChangedFile, FileStatus};

    fn lines(patch: &str) -> Vec<DiffLine> {
        ChangedFile::from_patch("a", "a", FileStatus::Modified, patch)
            .hunks
            .remove(0)
            .lines
    }

    #[test]
    fn pairs_change_blocks() {
        let l = lines("@@ -1,5 +1,4 @@\n a\n-b\n-c\n-d\n+B\n+C\n e\n+f\n");
        assert_eq!(
            pair_lines(&l),
            vec![
                (Some(0), Some(0)),
                (Some(1), Some(4)),
                (Some(2), Some(5)),
                (Some(3), None),
                (Some(6), Some(6)),
                (None, Some(7))
            ]
        );
    }

    #[test]
    fn more_additions_than_deletions() {
        let l = lines("@@ -1,1 +1,3 @@\n-x\n+y\n+z\n+w\n");
        assert_eq!(
            pair_lines(&l),
            vec![(Some(0), Some(1)), (None, Some(2)), (None, Some(3))]
        );
    }

    #[test]
    fn additions_before_deletions_stay_separate() {
        let l = lines("@@ -1,1 +1,1 @@\n+n\n-o\n");
        assert_eq!(pair_lines(&l), vec![(None, Some(0)), (Some(1), None)]);
    }
}
