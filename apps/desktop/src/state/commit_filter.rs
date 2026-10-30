//! Diff filtered to one commit (a click on a commit in Overview) or to the changes since my last
//! review: files come from the engine's cache; Esc or the banner's close returns to all changes.

use std::sync::Arc;

use gpui::Context;

use super::{bridge, err_str};
use crate::app::GitViewer;

/// What the diff is filtered to.
#[derive(Clone, Debug, PartialEq)]
pub enum DiffFilter {
    Commit(String),
    /// From the head I last reviewed to the current head.
    Since {
        from: String,
        to: String,
    },
}

impl DiffFilter {
    /// Banner text.
    pub fn label(&self) -> String {
        match self {
            DiffFilter::Commit(sha) => commit_label(sha),
            DiffFilter::Since { .. } => "Showing changes since your last review".into(),
        }
    }

    /// Commit whose line numbers the filtered diff uses (for placing threads).
    pub fn sha(&self) -> &str {
        match self {
            DiffFilter::Commit(sha) => sha,
            DiffFilter::Since { to, .. } => to,
        }
    }
}

/// "Showing commit a1b2c3d" for the banner.
pub fn commit_label(sha: &str) -> String {
    format!("Showing commit {}", &sha[..sha.len().min(7)])
}

impl GitViewer {
    pub(crate) fn show_commit(&mut self, sha: String, cx: &mut Context<Self>) {
        self.show_filtered(DiffFilter::Commit(sha), cx)
    }

    /// Diff from the head I last reviewed to the current head.
    pub(crate) fn show_since_review(&mut self, cx: &mut Context<Self>) {
        let Some(det) = self.detail.as_ref().and_then(|d| d.detail.as_ref()) else {
            return;
        };
        let (Some(from), Some(to)) = (det.my_review_sha.clone(), det.commits.last()) else {
            return;
        };
        let to = to.sha.clone();
        self.show_filtered(DiffFilter::Since { from, to }, cx)
    }

    fn show_filtered(&mut self, filter: DiffFilter, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_ref() else {
            return;
        };
        let id = d.summary.id.clone();
        let engine = self.engine.clone();
        let (fid, f) = (id.clone(), filter.clone());
        let rx = bridge::spawn(async move {
            match f {
                DiffFilter::Commit(sha) => engine.commit_files(&fid, &sha).await,
                DiffFilter::Since { from, to } => engine.range_files(&fid, &from, &to).await,
            }
            .map_err(err_str)
        });
        cx.spawn(async move |this, cx| {
            let Ok(r) = rx.await else { return };
            this.update(cx, |v, cx| match r {
                Ok(files) => {
                    let Some(d) = v.detail.as_mut().filter(|d| d.summary.id == id) else {
                        return;
                    };
                    d.code.commit = Some(filter);
                    d.code.set_files(Arc::new(files));
                    d.reindex();
                    v.after_files(cx);
                    cx.notify()
                }
                Err(e) => v.show_error_toast("Could not load the changes", e, cx),
            })
            .ok();
        })
        .detach();
    }

    /// Back to all changes; true when a commit filter was on.
    pub(crate) fn clear_commit(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(d) = self.detail.as_mut().filter(|d| d.code.commit.is_some()) else {
            return false;
        };
        d.code.commit = None;
        match self.code_cache.get(&d.summary.id).cloned() {
            Some(f) => {
                d.code.set_files(f);
                d.reindex();
                self.after_files(cx);
            }
            None => {
                d.code.files = None;
                self.load_code(cx);
            }
        }
        cx.notify();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_uses_short_sha() {
        assert_eq!(commit_label("a1b2c3d4e5f6"), "Showing commit a1b2c3d");
        assert_eq!(commit_label("abc"), "Showing commit abc");
        let since = DiffFilter::Since {
            from: "a".into(),
            to: "b".into(),
        };
        assert_eq!(since.sha(), "b");
        assert_eq!(DiffFilter::Commit("c1".into()).sha(), "c1");
        assert_eq!(since.label(), "Showing changes since your last review");
    }
}
