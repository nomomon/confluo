//! Reactions and per-file viewed state, keeping the cached detail in step.

use crate::Engine;
use anyhow::Result;
use gv_core::*;
use std::collections::HashSet;

/// Paths viewed now: provider state when there is one, else local marks whose fingerprint still matches.
pub fn viewed_paths(files: &[ChangedFile], remote: Option<&std::collections::HashMap<String, bool>>, local: &std::collections::HashMap<String, String>) -> HashSet<String> {
    files
        .iter()
        .filter(|f| match remote.and_then(|r| r.get(f.path())) {
            Some(v) => *v,
            None => local.get(f.path()).is_some_and(|fp| *fp == file_fingerprint(f)),
        })
        .map(|f| f.path().to_string())
        .collect()
}

impl Engine {
    /// Add or remove my reaction on a comment, or on the PR when `subject` is None.
    pub async fn react(&self, id: &ChangeRequestId, subject: Option<&str>, kind: ReactionKind, add: bool) -> Result<()> {
        let prov = self.provider(id.repo.kind, &id.repo.host).await?;
        let target = match (subject, id.repo.kind) {
            (Some(s), _) => s.to_string(),
            // GitHub reacts on the PR node; GitLab on the MR path.
            (None, ProviderKind::GitHub) => self.store().get_detail(id)?.and_then(|d| d.node_id).unwrap_or_default(),
            (None, _) => String::new(),
        };
        prov.react(id, &target, kind, add).await?;
        if let Some(mut d) = self.store().get_detail(id)? {
            let me = self.me(id.repo.kind, &id.repo.host).unwrap_or_default();
            let groups = match subject {
                None => Some(&mut d.reactions),
                Some(s) => d.threads.iter_mut().flat_map(|t| t.comments.iter_mut()).find(|c| c.id == s).map(|c| &mut c.reactions),
            };
            if let Some(g) = groups {
                if g.iter().any(|x| x.kind == kind && x.me) != add {
                    toggle_reaction(g, kind, &me);
                }
            }
            let _ = self.store().put_detail(&d);
        }
        Ok(())
    }

    /// Paths of `files` viewed by me.
    pub fn viewed_files(&self, id: &ChangeRequestId, files: &[ChangedFile]) -> HashSet<String> {
        let remote = self.store().get_detail(id).ok().flatten().and_then(|d| d.viewed);
        let local = self.store().viewed(&id.key()).unwrap_or_default();
        viewed_paths(files, remote.as_ref(), &local)
    }

    /// Mark a file viewed: on GitHub through the API, elsewhere locally with its diff fingerprint.
    pub async fn set_file_viewed(&self, id: &ChangeRequestId, file: &ChangedFile, viewed: bool) -> Result<()> {
        let path = file.path().to_string();
        let detail = self.store().get_detail(id)?;
        if id.repo.kind == ProviderKind::GitHub {
            let node = detail.as_ref().and_then(|d| d.node_id.clone());
            self.provider(id.repo.kind, &id.repo.host).await?.set_file_viewed(id, node.as_deref(), &path, viewed).await?;
            if let Some(mut d) = detail {
                d.viewed.get_or_insert_with(Default::default).insert(path, viewed);
                let _ = self.store().put_detail(&d);
            }
        } else {
            let fp = file_fingerprint(file);
            self.store().set_viewed(&id.key(), &path, viewed.then_some(fp.as_str()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(path: &str, body: &str) -> ChangedFile {
        ChangedFile::from_patch(path, path, FileStatus::Modified, &format!("@@ -1 +1 @@\n-a\n+{body}\n"))
    }

    #[test]
    fn viewed_resets_when_file_changes() {
        let a = f("a.rs", "one");
        let mut local = std::collections::HashMap::new();
        local.insert("a.rs".to_string(), file_fingerprint(&a));
        assert!(viewed_paths(std::slice::from_ref(&a), None, &local).contains("a.rs"));
        let changed = f("a.rs", "two");
        assert!(viewed_paths(std::slice::from_ref(&changed), None, &local).is_empty());
        let remote = std::collections::HashMap::from([("a.rs".to_string(), true)]);
        assert!(viewed_paths(&[changed], Some(&remote), &local).contains("a.rs"));
    }
}
