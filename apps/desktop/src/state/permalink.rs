//! Permalinks to selected lines on the provider.

use gv_engine::{DiffLine, DiffSide, ProviderKind};
use sha2::{Digest, Sha256};

use super::line_select::{endpoints, new_span};

/// Where links point: the change's page, the repo and the head commit.
pub struct LinkBase<'a> {
    pub kind: ProviderKind,
    pub pr_url: &'a str,
    pub repo_url: &'a str,
    pub head_sha: Option<&'a str>,
}

fn hex_sha256(s: &str) -> String {
    Sha256::digest(s.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// GitHub's anchor into the PR "Files changed" page, e.g. `#diff-<sha256(path)>R3-R7`.
pub fn github_diff_link(pr_url: &str, path: &str, a: (DiffSide, u32), b: (DiffSide, u32)) -> String {
    let s = |(side, n): (DiffSide, u32)| format!("{}{n}", if side == DiffSide::Old { 'L' } else { 'R' });
    let span = if a == b { s(a) } else { format!("{}-{}", s(a), s(b)) };
    format!("{pr_url}/files#diff-{}{span}", hex_sha256(path))
}

/// The file at a commit with lines `a..=b` highlighted.
pub fn blob_link(kind: ProviderKind, repo_url: &str, sha: &str, path: &str, a: u32, b: u32) -> String {
    match kind {
        ProviderKind::GitLab if a == b => format!("{repo_url}/-/blob/{sha}/{path}#L{a}"),
        ProviderKind::GitLab => format!("{repo_url}/-/blob/{sha}/{path}#L{a}-{b}"),
        ProviderKind::Azure => format!("{repo_url}?path=/{path}&version=GC{sha}&line={a}&lineEnd={}&lineStartColumn=1", b + 1),
        _ if a == b => format!("{repo_url}/blob/{sha}/{path}#L{a}"),
        _ => format!("{repo_url}/blob/{sha}/{path}#L{a}-L{b}"),
    }
}

/// Link to selected diff lines: GitHub's diff anchor, else a blob link on the new side.
pub fn diff_link(base: &LinkBase, path: &str, lines: &[DiffLine]) -> String {
    if base.kind == ProviderKind::GitHub {
        if let Some((a, b)) = endpoints(lines) {
            return github_diff_link(base.pr_url, path, a, b);
        }
    }
    match (base.head_sha, new_span(lines)) {
        (Some(sha), Some((a, b))) => blob_link(base.kind, base.repo_url, sha, path, a, b),
        _ => base.pr_url.to_string(),
    }
}

/// Link to lines `a..=b` of the whole file at the head commit.
pub fn file_link(base: &LinkBase, path: &str, a: u32, b: u32) -> String {
    match base.head_sha {
        Some(sha) => blob_link(base.kind, base.repo_url, sha, path, a, b),
        None => base.pr_url.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gv_engine::{ChangedFile, FileStatus};

    #[test]
    fn github_and_gitlab_links() {
        let h = hex_sha256("src/a.rs");
        assert_eq!(h.len(), 64);
        assert_eq!(github_diff_link("https://github.com/o/r/pull/4", "src/a.rs", (DiffSide::New, 3), (DiffSide::New, 7)), format!("https://github.com/o/r/pull/4/files#diff-{h}R3-R7"));
        assert_eq!(github_diff_link("u", "src/a.rs", (DiffSide::Old, 2), (DiffSide::Old, 2)), format!("u/files#diff-{h}L2"));
        assert_eq!(blob_link(ProviderKind::GitHub, "https://github.com/o/r", "abc", "a.rs", 1, 4), "https://github.com/o/r/blob/abc/a.rs#L1-L4");
        assert_eq!(blob_link(ProviderKind::GitLab, "https://gitlab.com/g/p", "abc", "a.rs", 1, 4), "https://gitlab.com/g/p/-/blob/abc/a.rs#L1-4");
        assert_eq!(blob_link(ProviderKind::GitLab, "https://gitlab.com/g/p", "abc", "a.rs", 5, 5), "https://gitlab.com/g/p/-/blob/abc/a.rs#L5");
    }

    #[test]
    fn diff_link_by_provider() {
        let f = ChangedFile::from_patch("a.rs", "a.rs", FileStatus::Modified, "@@ -1,2 +1,2 @@\n-old\n+new\n ctx\n");
        let lines = &f.hunks[0].lines;
        let gh = LinkBase { kind: ProviderKind::GitHub, pr_url: "p", repo_url: "r", head_sha: Some("s") };
        assert!(diff_link(&gh, "a.rs", lines).ends_with("L1-R2"));
        let gl = LinkBase { kind: ProviderKind::GitLab, ..gh };
        assert_eq!(diff_link(&gl, "a.rs", lines), "r/-/blob/s/a.rs#L1-2");
        assert_eq!(diff_link(&gl, "a.rs", &lines[..1]), "p");
        assert_eq!(file_link(&LinkBase { head_sha: None, ..gl }, "a.rs", 1, 2), "p");
    }
}
