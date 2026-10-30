//! One commit's diff: `GET .../commits/{sha}/changes`, then blobs (or the file at the parent when
//! the change has no original blob id), diffed locally like the PR diff.

use gv_core::*;
use serde_json::Value;

use crate::diff::{entries_in, Blob, Entry};
use crate::{query, AzureProvider, API};

/// Entries of a commit's change list.
pub(crate) fn commit_entries(v: &Value) -> Vec<Entry> {
    entries_in(&v["changes"])
}

/// First parent of `GET .../commits/{sha}`.
pub(crate) fn first_parent(commit: &Value) -> Option<String> {
    commit["parents"][0].as_str().map(str::to_string)
}

impl AzureProvider {
    /// A file's content at a commit; None when it does not exist there.
    pub(crate) async fn item_at(
        &self,
        repo_url: &str,
        path: &str,
        rev: &str,
    ) -> ProviderResult<Option<Blob>> {
        let p = format!("/{}", path.trim_start_matches('/'));
        let url = format!(
            "{repo_url}/items?path={}&versionDescriptor.version={}&versionDescriptor.versionType=commit&$format=octetStream&{API}",
            query(&p),
            query(rev)
        );
        Ok(self.get_bytes(&url).await?.map(|b| Blob::from_bytes(&b)))
    }

    pub(crate) async fn commit_diff(
        &self,
        id: &ChangeRequestId,
        sha: &str,
    ) -> ProviderResult<Vec<ChangedFile>> {
        let repo_url = self.repo_url(id)?;
        let commit = self.get(&format!("{repo_url}/commits/{sha}?{API}")).await?;
        let parent = first_parent(&commit);
        let mut all = vec![];
        let mut skip = 0usize;
        for _ in 0..10 {
            let v = self
                .get(&format!(
                    "{repo_url}/commits/{sha}/changes?top=1000&skip={skip}&{API}"
                ))
                .await?;
            let page = commit_entries(&v);
            let n = v["changes"].as_array().map_or(0, |a| a.len());
            all.extend(page);
            if n < 1000 {
                break;
            }
            skip += n;
        }
        self.diff_entries(&repo_url, all, parent.as_deref()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_change_entries() {
        let v: Value =
            serde_json::from_str(include_str!("../fixtures/commit_changes.json")).unwrap();
        let e = commit_entries(&v);
        assert_eq!(e.len(), 4, "folders are skipped");
        let side = |i: usize| (e[i].old_blob.as_deref(), e[i].new_blob.as_deref());
        assert_eq!(e[0].new_path, "src/app.ts");
        assert_eq!(side(0), (Some("o1"), Some("n1")));
        // No original blob id: the old side is read at the parent by path.
        assert_eq!(
            (e[1].status, side(1)),
            (FileStatus::Modified, (None, Some("n2")))
        );
        assert_eq!(
            (e[2].status, side(2)),
            (FileStatus::Added, (None, Some("n3")))
        );
        assert_eq!(
            (e[3].status, side(3)),
            (FileStatus::Deleted, (Some("o4"), None))
        );
    }

    #[test]
    fn parent_of_commit() {
        let v = serde_json::json!({"commitId": "c1", "parents": ["p1", "p2"]});
        assert_eq!(first_parent(&v).as_deref(), Some("p1"));
        assert_eq!(first_parent(&serde_json::json!({})), None);
    }
}
