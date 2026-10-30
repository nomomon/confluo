//! Diff filtered to one commit (a click on a commit in Overview): files come from the engine's
//! sha cache; Esc or the banner's close returns to all changes.

use std::sync::Arc;

use gpui::Context;

use super::{bridge, err_str};
use crate::app::GitViewer;

/// "Showing commit a1b2c3d" for the banner.
pub fn commit_label(sha: &str) -> String {
    format!("Showing commit {}", &sha[..sha.len().min(7)])
}

impl GitViewer {
    pub(crate) fn show_commit(&mut self, sha: String, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_ref() else {
            return;
        };
        let id = d.summary.id.clone();
        let engine = self.engine.clone();
        let (fid, fsha) = (id.clone(), sha.clone());
        let rx =
            bridge::spawn(async move { engine.commit_files(&fid, &fsha).await.map_err(err_str) });
        cx.spawn(async move |this, cx| {
            let Ok(r) = rx.await else { return };
            this.update(cx, |v, cx| match r {
                Ok(files) => {
                    let Some(d) = v.detail.as_mut().filter(|d| d.summary.id == id) else {
                        return;
                    };
                    d.code.commit = Some(sha);
                    d.code.set_files(Arc::new(files));
                    d.reindex();
                    v.after_files(cx);
                    cx.notify()
                }
                Err(e) => v.show_error_toast("Could not load the commit", e, cx),
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
    }
}
