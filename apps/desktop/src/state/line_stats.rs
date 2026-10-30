//! File and line counts for PRs whose list rows have none (Azure DevOps, GitLab): the file count
//! from one request when previewed, then +/- from the diff in the background, cached by head.

use gpui::Context;
use gv_engine::ChangeRequestId;

use super::bridge;
use crate::app::GitViewer;

/// Previews compute +/- on their own only up to this many files (each file costs blob requests);
/// larger PRs get them when Review mode loads the diff.
pub const AUTO_DIFF_MAX_FILES: u32 = 60;

/// Whether a preview should compute +/- in the background.
pub fn wants_diff(files: Option<u32>, additions: Option<u32>) -> bool {
    additions.is_none() && files.is_some_and(|n| n > 0 && n <= AUTO_DIFF_MAX_FILES)
}

impl GitViewer {
    fn set_file_count(&mut self, id: &ChangeRequestId, n: u32) {
        for c in self.items.iter_mut().filter(|c| &c.id == id) {
            c.changed_files.get_or_insert(n);
        }
        if let Some(d) = self.detail.as_mut().filter(|d| &d.summary.id == id) {
            d.summary.changed_files.get_or_insert(n);
        }
    }

    /// The file count, from one cheap request, so the preview shows "N files" at once.
    pub(crate) fn load_file_count(&mut self, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else {
            return;
        };
        if d.summary.additions.is_some() || d.summary.changed_files.is_some() {
            return;
        }
        d.stats_loading = true;
        let (id, engine) = (d.summary.id.clone(), self.engine.clone());
        let fid = id.clone();
        let rx = bridge::spawn(async move { engine.file_count(&fid).await.ok().flatten() });
        cx.spawn(async move |this, cx| {
            let Ok(n) = rx.await else { return };
            this.update(cx, |v, cx| {
                match n {
                    Some(n) => v.set_file_count(&id, n),
                    None => v.stats_done(&id),
                }
                v.load_line_stats(cx);
                cx.notify()
            })
            .ok();
        })
        .detach();
    }

    fn stats_done(&mut self, id: &ChangeRequestId) {
        if let Some(d) = self.detail.as_mut().filter(|d| &d.summary.id == id) {
            // `stats_diffing` stays set: a failed diff is not retried on every detail reload.
            d.stats_loading = false;
        }
    }

    /// +/- for the open PR from the diff (governed, capped, binaries and large blobs skipped by the
    /// provider). Runs once the detail and file count are in, so nothing is fetched twice.
    pub(crate) fn load_line_stats(&mut self, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else {
            return;
        };
        let s = &d.summary;
        if s.additions.is_some() {
            let c = s.clone();
            for row in self
                .items
                .iter_mut()
                .filter(|r| r.id == c.id && r.additions.is_none())
            {
                (row.additions, row.deletions) = (c.additions, c.deletions);
                row.changed_files = c.changed_files.or(row.changed_files);
            }
            if let Some(d) = self.detail.as_mut() {
                d.stats_loading = false;
            }
            return;
        }
        if d.detail.is_none() || !wants_diff(s.changed_files, s.additions) || d.code.loading {
            if s.additions.is_some() || s.changed_files.is_some_and(|n| n > AUTO_DIFF_MAX_FILES) {
                d.stats_loading = false;
            }
            return;
        }
        if d.stats_diffing {
            return;
        }
        d.stats_loading = true;
        d.stats_diffing = true;
        let (id, engine) = (s.id.clone(), self.engine.clone());
        let fid = id.clone();
        let rx = bridge::spawn(async move { engine.changed_files(&fid).await.ok() });
        cx.spawn(async move |this, cx| {
            let Ok(files) = rx.await else { return };
            this.update(cx, |v, cx| {
                if let Some(f) = files {
                    v.apply_file_totals(&id, &f);
                }
                v.stats_done(&id);
                cx.notify()
            })
            .ok();
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previews_diff_only_small_prs_without_counts() {
        assert!(wants_diff(Some(3), None));
        assert!(!wants_diff(Some(3), Some(10)));
        assert!(!wants_diff(None, None));
        assert!(!wants_diff(Some(0), None));
        assert!(!wants_diff(Some(AUTO_DIFF_MAX_FILES + 1), None));
    }
}
