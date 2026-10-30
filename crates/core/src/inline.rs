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

/// Place threads on one commit's diff (the Overview commit filter). Only threads that belong to
/// that commit's lines show: when the commit is the head, current threads whose line is in its
/// hunks; otherwise only threads made at that commit (`head_sha`). Nothing goes to "outdated".
pub fn index_commit_threads(
    threads: &[Thread],
    files: &[ChangedFile],
    sha: &str,
    is_head: bool,
) -> InlineIndex {
    let mut ix = InlineIndex::default();
    for (i, t) in threads.iter().enumerate() {
        let Some(p) = &t.position else { continue };
        let ours = if is_head {
            !t.outdated
        } else {
            p.head_sha.as_deref() == Some(sha)
        };
        if !ours {
            continue;
        }
        let Some(f) = files
            .iter()
            .find(|f| f.new_path == p.path || f.old_path == p.path)
        else {
            continue;
        };
        if has_line(f, p.side, p.line) {
            let key = f.path().to_string();
            *ix.counts.entry(key.clone()).or_default() += 1;
            ix.by_line.entry((key, p.side, p.line)).or_default().push(i);
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

    #[test]
    fn commit_filter_hides_threads_outside_the_commit() {
        let f = ChangedFile::from_patch(
            "a.rs",
            "a.rs",
            FileStatus::Modified,
            "@@ -1,2 +1,2 @@\n ctx\n-old\n+new",
        );
        let at = |line: u32, sha: Option<&str>, outdated: bool| {
            let mut t = thread("a.rs", DiffSide::New, line, outdated);
            t.position.as_mut().unwrap().head_sha = sha.map(str::to_string);
            t
        };
        let ts = vec![
            at(2, None, false),                      // on the commit's line
            at(40, None, false),                     // same file, line not in the commit
            at(2, None, true),                       // outdated
            thread("b.rs", DiffSide::New, 1, false), // file not in the commit
            at(2, Some("c1"), false),                // made at commit c1
        ];
        let ix = index_commit_threads(&ts, std::slice::from_ref(&f), "c1", true);
        assert_eq!(ix.at("a.rs", DiffSide::New, 2), &[0, 4]);
        assert!(ix.outdated.is_empty());
        assert_eq!(ix.count("a.rs"), 2);
        // An older commit: line numbers of head threads do not apply; only threads made there.
        let ix = index_commit_threads(&ts, &[f], "c1", false);
        assert_eq!(ix.at("a.rs", DiffSide::New, 2), &[4]);
        assert_eq!(ix.count("a.rs"), 1);
    }
}
