//! Changed files: the last iteration's change list, blobs at both sides and a local unified diff.
//!
//! Azure DevOps has no patch endpoint for pull requests, so each changed blob is fetched (two
//! requests per edited file) and diffed here. The engine caches the result by head commit.

use futures_util::stream::{self, StreamExt};
use gv_core::*;
use serde_json::Value;

use crate::{AzureProvider, API};

/// Files diffed per PR; the rest are listed as too large to keep request volume down.
const MAX_DIFF_FILES: usize = 120;
/// Blobs larger than this are not diffed.
const MAX_BLOB: usize = 1024 * 1024;
const CONTEXT: usize = 3;

/// One change entry, before its blobs are fetched.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Entry {
    pub old_path: String,
    pub new_path: String,
    pub status: FileStatus,
    pub old_blob: Option<String>,
    pub new_blob: Option<String>,
}

fn clean(p: &str) -> String {
    p.trim_start_matches('/').to_string()
}

/// Blob entries of `GET .../iterations/{id}/changes` (folders are skipped).
pub(crate) fn entries(changes: &Value) -> Vec<Entry> {
    changes["changeEntries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| {
            c["item"]["gitObjectType"].as_str().unwrap_or("blob") == "blob"
                && c["item"]["isFolder"] != true
        })
        .filter_map(|c| {
            let item = &c["item"];
            let path = clean(item["path"].as_str()?);
            let ct = c["changeType"].as_str().unwrap_or("edit");
            let status = if ct.contains("add") {
                FileStatus::Added
            } else if ct.contains("delete") {
                FileStatus::Deleted
            } else if ct.contains("rename") {
                FileStatus::Renamed
            } else {
                FileStatus::Modified
            };
            let old_path = c["originalPath"]
                .as_str()
                .or(c["sourceServerItem"].as_str())
                .map(clean)
                .unwrap_or_else(|| path.clone());
            let blob = |k: &str| {
                item[k]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
            };
            let (old_blob, new_blob) = match status {
                FileStatus::Added => (None, blob("objectId")),
                FileStatus::Deleted => (blob("originalObjectId").or(blob("objectId")), None),
                _ => (blob("originalObjectId"), blob("objectId")),
            };
            Some(Entry {
                old_path,
                new_path: path,
                status,
                old_blob,
                new_blob,
            })
        })
        .collect()
}

/// Content as text, or None when it looks binary.
fn text(bytes: &[u8]) -> Option<String> {
    if bytes[..bytes.len().min(8000)].contains(&0) {
        return None;
    }
    String::from_utf8(bytes.to_vec()).ok()
}

/// Unified diff of two texts (hunks only, no file headers).
pub(crate) fn unified(old: &str, new: &str) -> String {
    let d = similar::TextDiff::from_lines(old, new);
    let mut out = String::new();
    for h in d.unified_diff().context_radius(CONTEXT).iter_hunks() {
        let s = h.to_string();
        out.push_str(&s);
        if !s.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

/// One side of a change.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Blob {
    Text(String),
    Binary,
    TooLarge,
}

impl Blob {
    fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.len() > MAX_BLOB {
            return Blob::TooLarge;
        }
        text(bytes).map(Blob::Text).unwrap_or(Blob::Binary)
    }
}

/// A changed file from the blob contents (None = the side does not exist).
pub(crate) fn file(e: &Entry, old: Option<Blob>, new: Option<Blob>) -> ChangedFile {
    let mut f = ChangedFile::from_patch(&e.old_path, &e.new_path, e.status, "");
    if [&old, &new]
        .iter()
        .any(|b| matches!(b, Some(Blob::TooLarge)))
    {
        f.too_large = true;
    } else if [&old, &new].iter().any(|b| matches!(b, Some(Blob::Binary))) {
        f.binary = true;
    } else {
        let t = |b: Option<Blob>| match b {
            Some(Blob::Text(t)) => t,
            _ => String::new(),
        };
        let (o, n) = (t(old), t(new));
        if o != n {
            f = ChangedFile::from_patch(&e.old_path, &e.new_path, e.status, &unified(&o, &n));
        }
    }
    f
}

impl AzureProvider {
    /// A blob by object id.
    async fn blob(&self, repo_url: &str, sha: &str) -> ProviderResult<Blob> {
        let url = format!("{repo_url}/blobs/{sha}?$format=octetStream&{API}");
        Ok(Blob::from_bytes(
            &self.get_bytes(&url).await?.unwrap_or_default(),
        ))
    }

    pub(crate) async fn changed_files(
        &self,
        id: &ChangeRequestId,
    ) -> ProviderResult<Vec<ChangedFile>> {
        let base = self.pr_url(id)?;
        let repo_url = self.repo_url(id)?;
        let its = self.get(&format!("{base}/iterations?{API}")).await?;
        let Some(last) = its["value"]
            .as_array()
            .and_then(|a| a.iter().filter_map(|i| i["id"].as_u64()).max())
        else {
            return Ok(vec![]);
        };
        let mut all = vec![];
        let mut skip = 0u64;
        for _ in 0..10 {
            let v = self
                .get(&format!(
                    "{base}/iterations/{last}/changes?$top=1000&$skip={skip}&$compareTo=0&{API}"
                ))
                .await?;
            all.extend(entries(&v));
            match v["nextSkip"].as_u64() {
                Some(n) if n > skip => skip = n,
                _ => break,
            }
        }
        let jobs = all.into_iter().enumerate().map(|(i, e)| {
            let repo_url = repo_url.clone();
            async move {
                if i >= MAX_DIFF_FILES {
                    let mut f = ChangedFile::from_patch(&e.old_path, &e.new_path, e.status, "");
                    f.too_large = true;
                    return Ok(f);
                }
                // A pure rename keeps its blob: nothing to fetch.
                if e.old_blob.is_some() && e.old_blob == e.new_blob {
                    return Ok(ChangedFile::from_patch(
                        &e.old_path,
                        &e.new_path,
                        e.status,
                        "",
                    ));
                }
                let old = match &e.old_blob {
                    Some(b) => Some(self.blob(&repo_url, b).await?),
                    None => None,
                };
                let new = match &e.new_blob {
                    Some(b) => Some(self.blob(&repo_url, b).await?),
                    None => None,
                };
                Ok::<_, ProviderError>(file(&e, old, new))
            }
        });
        stream::iter(jobs)
            .buffered(crate::CONCURRENCY)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_entries() {
        let v: Value = serde_json::from_str(include_str!("../fixtures/changes.json")).unwrap();
        let e = entries(&v);
        assert_eq!(e.len(), 5, "folders are skipped");
        assert_eq!(
            (
                e[0].new_path.as_str(),
                e[0].status,
                e[0].old_blob.as_deref(),
                e[0].new_blob.as_deref()
            ),
            ("src/app.ts", FileStatus::Modified, Some("o1"), Some("n1"))
        );
        assert_eq!(
            (
                e[1].status,
                e[1].old_blob.as_deref(),
                e[1].new_blob.as_deref()
            ),
            (FileStatus::Added, None, Some("n2"))
        );
        assert_eq!(
            (
                e[2].status,
                e[2].old_blob.as_deref(),
                e[2].new_blob.as_deref()
            ),
            (FileStatus::Deleted, Some("o3"), None)
        );
        assert_eq!(
            (e[3].old_path.as_str(), e[3].new_path.as_str(), e[3].status),
            ("lib/a.rs", "lib/b.rs", FileStatus::Renamed)
        );
        assert_eq!(
            (e[4].old_path.as_str(), e[4].status),
            ("lib/c_old.rs", FileStatus::Renamed)
        );
    }

    #[test]
    fn local_diff() {
        let old = "a\nb\nc\nd\ne\nf\ng\nh\n";
        let new = "a\nb\nc\nD\ne\nf\ng\nh\ni\n";
        let p = unified(old, new);
        assert!(p.starts_with("@@ -1,8 +1,9 @@"), "{p}");
        let e = Entry {
            old_path: "x".into(),
            new_path: "x".into(),
            status: FileStatus::Modified,
            old_blob: Some("o".into()),
            new_blob: Some("n".into()),
        };
        let t = |s: &str| Some(Blob::Text(s.into()));
        let f = file(&e, t(old), t(new));
        assert_eq!((f.additions, f.deletions, f.hunks.len()), (2, 1, 1));
        assert_eq!(f.hunks[0].lines[3].old_no, Some(4));
        let added = Entry {
            status: FileStatus::Added,
            old_blob: None,
            ..e.clone()
        };
        let f = file(&added, None, t("1\n2\n"));
        assert_eq!((f.additions, f.deletions), (2, 0));
        assert!(file(&e, t("x"), Some(Blob::Binary)).binary);
        assert!(file(&e, t("x"), Some(Blob::TooLarge)).too_large);
        assert_eq!(Blob::from_bytes(&vec![b'a'; MAX_BLOB + 1]), Blob::TooLarge);
        assert_eq!(text(b"a\0b"), None);
        // No trailing newline still yields parseable hunks.
        let f = file(&e, t("a"), t("b"));
        assert_eq!((f.additions, f.deletions), (1, 1));
    }
}
