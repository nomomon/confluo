//! Review mode file tree model: files grouped by folder, folders foldable, a fuzzy filter and
//! "Hide viewed".

use std::cmp::Ordering;
use std::collections::HashSet;

/// The one file order shared by the tree, the diff list and n/p: at each folder level
/// subfolders come before files, names compare case-insensitively (then exactly, for stability).
pub fn tree_cmp(a: &str, b: &str) -> Ordering {
    let (pa, pb): (Vec<&str>, Vec<&str>) = (a.split('/').collect(), b.split('/').collect());
    for k in 0..pa.len().min(pb.len()) {
        let (da, db) = (k + 1 < pa.len(), k + 1 < pb.len());
        if pa[k] == pb[k] && da && db {
            continue;
        }
        return db
            .cmp(&da)
            .then_with(|| pa[k].to_lowercase().cmp(&pb[k].to_lowercase()))
            .then_with(|| pa[k].cmp(pb[k]));
    }
    pa.len().cmp(&pb.len())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeEntry {
    /// Folder header with its path and number of files under it.
    Dir(String, usize),
    /// File index into the changed files.
    File(usize),
}

pub struct TreeQuery<'a> {
    pub filter: &'a str,
    pub hide_viewed: bool,
    pub viewed: &'a HashSet<String>,
    pub folded: &'a HashSet<String>,
}

/// Visible rows for `paths` (file index order), in `tree_cmp` order. A filter shows matches flat.
pub fn tree_entries(
    paths: &[&str],
    q: &TreeQuery,
    score: impl Fn(&str, &str) -> Option<i32>,
) -> Vec<TreeEntry> {
    let keep = |i: &usize| !(q.hide_viewed && q.viewed.contains(paths[*i]));
    if !q.filter.trim().is_empty() {
        let mut hits: Vec<(i32, usize)> = (0..paths.len())
            .filter(keep)
            .filter_map(|i| score(q.filter, paths[i]).map(|s| (s, i)))
            .collect();
        hits.sort_by(|a, b| b.0.cmp(&a.0).then(paths[a.1].cmp(paths[b.1])));
        return hits.into_iter().map(|(_, i)| TreeEntry::File(i)).collect();
    }
    let mut order: Vec<usize> = (0..paths.len()).filter(keep).collect();
    order.sort_by(|a, b| tree_cmp(paths[*a], paths[*b]));
    let dir = |i: usize| paths[i].rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    let mut out = vec![];
    let mut k = 0;
    while k < order.len() {
        let d = dir(order[k]);
        let n = order[k..].iter().take_while(|i| dir(**i) == d).count();
        if !d.is_empty() {
            out.push(TreeEntry::Dir(d.to_string(), n));
        }
        if d.is_empty() || !q.folded.contains(d) {
            out.extend(order[k..k + n].iter().map(|i| TreeEntry::File(*i)));
        }
        k += n;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sub(q: &str, t: &str) -> Option<i32> {
        t.contains(q).then_some(1)
    }

    #[test]
    fn groups_folds_filters_and_hides() {
        let paths = ["src/b.rs", "README.md", "src/a.rs", "docs/x.md"];
        let (none, mut folded) = (HashSet::new(), HashSet::new());
        let q = TreeQuery {
            filter: "",
            hide_viewed: false,
            viewed: &none,
            folded: &folded,
        };
        assert_eq!(
            tree_entries(&paths, &q, sub),
            vec![
                TreeEntry::Dir("docs".into(), 1),
                TreeEntry::File(3),
                TreeEntry::Dir("src".into(), 2),
                TreeEntry::File(2),
                TreeEntry::File(0),
                TreeEntry::File(1)
            ]
        );
        folded.insert("src".to_string());
        let q = TreeQuery {
            filter: "",
            hide_viewed: false,
            viewed: &none,
            folded: &folded,
        };
        assert_eq!(
            tree_entries(&paths, &q, sub)[2],
            TreeEntry::Dir("src".into(), 2)
        );
        assert_eq!(tree_entries(&paths, &q, sub).len(), 4);
        let viewed = HashSet::from(["src/a.rs".to_string()]);
        let q = TreeQuery {
            filter: "src",
            hide_viewed: true,
            viewed: &viewed,
            folded: &folded,
        };
        assert_eq!(tree_entries(&paths, &q, sub), vec![TreeEntry::File(0)]);
    }

    #[test]
    fn tree_order_dirs_first_case_insensitive_root_files_last() {
        let mut p = vec![
            "CONTEXT.md",
            "src/b.rs",
            "Cargo.toml",
            "src/sub/x.rs",
            "apps/A.rs",
            "src/a.rs",
            "apps/b.rs",
            "README.md",
            "docs/z.md",
        ];
        p.sort_by(|a, b| tree_cmp(a, b));
        assert_eq!(
            p,
            vec![
                "apps/A.rs",
                "apps/b.rs",
                "docs/z.md",
                "src/sub/x.rs",
                "src/a.rs",
                "src/b.rs",
                "Cargo.toml",
                "CONTEXT.md",
                "README.md"
            ]
        );
        // The tree groups follow the same order as the sorted list (what the diff shows).
        let none = HashSet::new();
        let q = TreeQuery {
            filter: "",
            hide_viewed: false,
            viewed: &none,
            folded: &none,
        };
        let files: Vec<usize> = tree_entries(&p, &q, sub)
            .into_iter()
            .filter_map(|e| {
                if let TreeEntry::File(i) = e {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(files, (0..p.len()).collect::<Vec<_>>());
    }
}
