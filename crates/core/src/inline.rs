//! Index inline review threads by file and diff line.

use crate::{ChangedFile, DiffSide, Thread};
use std::collections::HashMap;

/// Key of one diff line: (file path as shown, side, line number on that side).
pub type LineKey = (String, DiffSide, u32);

/// Inline threads of a PR placed onto its current diff.
#[derive(Debug, Default, Clone)]
pub struct InlineIndex {
    /// Thread indices (into `detail.threads`) anchored at a visible diff line.
    pub by_line: HashMap<LineKey, Vec<usize>>,
    /// Per file path: threads whose line is no longer in the diff.
    pub outdated: HashMap<String, Vec<usize>>,
    /// Per file path: total inline threads (current and outdated).
    pub counts: HashMap<String, usize>,
}

impl InlineIndex {
    pub fn at(&self, path: &str, side: DiffSide, line: u32) -> &[usize] {
        self.by_line
            .get(&(path.to_string(), side, line))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
    pub fn count(&self, path: &str) -> usize {
        self.counts.get(path).copied().unwrap_or(0)
    }
}

fn has_line(f: &ChangedFile, side: DiffSide, line: u32) -> bool {
    f.hunks.iter().flat_map(|h| &h.lines).any(|l| match side {
        DiffSide::Old => l.old_no == Some(line),
        DiffSide::New => l.new_no == Some(line),
    })
}

/// Place every positioned thread on its file's diff. Keys use `ChangedFile::path()`.
pub fn index_inline_threads(threads: &[Thread], files: &[ChangedFile]) -> InlineIndex {
    let mut ix = InlineIndex::default();
    for (i, t) in threads.iter().enumerate() {
        let Some(p) = &t.position else { continue };
        let file = files.iter().find(|f| {
            f.new_path == p.path
                || f.old_path == p.path
                || p.old_path.as_deref() == Some(f.old_path.as_str())
        });
        let Some(f) = file else { continue };
        let key = f.path().to_string();
        *ix.counts.entry(key.clone()).or_default() += 1;
        if !t.outdated && has_line(f, p.side, p.line) {
            ix.by_line.entry((key, p.side, p.line)).or_default().push(i);
        } else {
            ix.outdated.entry(key).or_default().push(i);
        }
    }
    ix
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DiffPosition, FileStatus};

    fn thread(path: &str, side: DiffSide, line: u32, outdated: bool) -> Thread {
        let mut t = Thread::conversation("t", vec![]);
        t.position = Some(DiffPosition::new(path, side, line));
        t.outdated = outdated;
        t
    }

    #[test]
    fn indexes_by_line() {
        let f = ChangedFile::from_patch(
            "a.rs",
            "a.rs",
            FileStatus::Modified,
            "@@ -1,2 +1,2 @@\n ctx\n-old\n+new",
        );
        let ts = vec![
            thread("a.rs", DiffSide::New, 2, false),
            thread("a.rs", DiffSide::Old, 2, false),
            thread("a.rs", DiffSide::New, 50, false),
            thread("a.rs", DiffSide::New, 1, true),
            thread("zzz", DiffSide::New, 1, false),
            Thread::conversation("c", vec![]),
        ];
        let ix = index_inline_threads(&ts, &[f]);
        assert_eq!(ix.at("a.rs", DiffSide::New, 2), &[0]);
        assert_eq!(ix.at("a.rs", DiffSide::Old, 2), &[1]);
        assert_eq!(ix.outdated["a.rs"], vec![2, 3]);
        assert_eq!(ix.count("a.rs"), 4);
    }
}
