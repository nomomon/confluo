//! Line and file counts for providers whose lists carry none (Azure DevOps, GitLab): a cheap file
//! count first, +a -d from the head-keyed diff cache once the diff was computed.

use anyhow::Result;
use gv_core::{ChangeRequest, ChangeRequestId, ChangedFile};

/// Counts for `cr` from the diff cached at `head`, else the counts the cached list row already has.
pub(crate) fn fill(
    cr: &mut ChangeRequest,
    files: Option<(String, Vec<ChangedFile>)>,
    head: Option<&str>,
    row: Option<&ChangeRequest>,
) {
    if cr.additions.is_some() {
        return;
    }
    let row_files = row.and_then(|r| r.changed_files);
    match files {
        Some((sha, f)) if Some(sha.as_str()) == head && !sha.is_empty() => {
            let (a, d) = gv_core::totals(&f);
            cr.additions = Some(a);
            cr.deletions = Some(d);
            cr.changed_files = Some(f.len() as u32);
        }
        // The row's +/- came from that older diff: stale, so only the file count carries over.
        Some(_) => cr.changed_files = cr.changed_files.or(row_files),
        None => {
            if let Some(r) = row {
                cr.additions = r.additions;
                cr.deletions = r.deletions;
                cr.changed_files = cr.changed_files.or(row_files);
            }
        }
    }
}

impl crate::Engine {
    /// Fill missing counts on a freshly fetched detail summary (its payload has none).
    pub(crate) fn fill_counts(&self, cr: &mut ChangeRequest, head: Option<&str>) {
        let files = self.store.get_files(&cr.id).ok().flatten();
        let row = self.store.get_change_request(&cr.id).ok().flatten();
        fill(cr, files, head, row.as_ref());
        // Persist onto the list row so it shows +a -d too.
        if let Some(mut r) = row.filter(|r| r.additions.is_none() && cr.additions.is_some()) {
            (r.additions, r.deletions, r.changed_files) =
                (cr.additions, cr.deletions, cr.changed_files);
            let _ = self.store.upsert_change_requests(&[r]);
        }
    }

    /// Changed file count for a PR whose list row has none: from the cache, else one cheap provider
    /// request. Stored on the cached row so the list shows it too.
    pub async fn file_count(&self, id: &ChangeRequestId) -> Result<Option<u32>> {
        let row = self.store.get_change_request(id)?;
        if let Some(n) = row.as_ref().and_then(|r| r.changed_files) {
            return Ok(Some(n));
        }
        let n = self
            .provider(id.repo.kind, &id.repo.host)
            .await?
            .changed_file_count(id)
            .await?;
        if let (Some(n), Some(mut r)) = (n, row) {
            r.changed_files = Some(n);
            let _ = self.store.upsert_change_requests(&[r]);
        }
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gv_core::*;

    fn cr() -> ChangeRequest {
        ChangeRequest {
            id: ChangeRequestId {
                repo: RepoRef::new(ProviderKind::Azure, "dev.azure.com", "o/p/r"),
                number: 1,
            },
            title: String::new(),
            author: User::named("a"),
            state: ChangeRequestState::Open,
            source_branch: String::new(),
            target_branch: String::new(),
            reviewers: vec![],
            checks: CheckSummary::default(),
            labels: vec![],
            comments_count: 0,
            additions: None,
            deletions: None,
            changed_files: None,
            updated_at: chrono::Utc::now(),
            created_at: chrono::Utc::now(),
            web_url: String::new(),
            needs_you: false,
            signals: Default::default(),
        }
    }

    #[test]
    fn counts_come_from_the_diff_at_the_same_head_only() {
        let f = vec![ChangedFile::from_patch(
            "a",
            "a",
            FileStatus::Modified,
            "@@ -1,1 +1,2 @@\n-x\n+y\n+z\n",
        )];
        let mut c = cr();
        fill(&mut c, Some(("h1".into(), f.clone())), Some("h1"), None);
        assert_eq!(
            (c.additions, c.deletions, c.changed_files),
            (Some(2), Some(1), Some(1))
        );
        let mut stale = cr();
        let mut row = cr();
        row.changed_files = Some(4);
        fill(&mut stale, Some(("h0".into(), f)), Some("h1"), Some(&row));
        assert_eq!((stale.additions, stale.changed_files), (None, Some(4)));
    }
}
