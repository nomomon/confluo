//! Loading the Code tab's changed files: memory cache first, then the engine.

use std::sync::Arc;

use gpui::Context;

use super::{bridge, err_str, timing};
use crate::app::GitViewer;

impl GitViewer {
    /// Load changed files for the open PR: memory cache first, then the engine (sha-keyed cache).
    pub(crate) fn load_code(&mut self, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else {
            return;
        };
        if d.code.loading {
            return;
        }
        let id = d.summary.id.clone();
        if d.code.files.is_none() {
            // Memory first, then the disk cache, so a reopened Code tab paints at once and revalidates quietly.
            let cached = self
                .code_cache
                .get(&id)
                .cloned()
                .or_else(|| self.engine.cached_changed_files(&id).map(Arc::new));
            if let Some(f) = cached {
                self.code_cache.insert(id.clone(), f.clone());
                d.code.set_files(f);
                d.reindex();
                self.after_files(cx);
            }
        }
        let Some(d) = self.detail.as_mut() else {
            return;
        };
        d.code.loading = true;
        d.code.show_skeleton = false;
        let engine = self.engine.clone();
        let fid = id.clone();
        let rx = bridge::spawn(async move { engine.changed_files(&fid).await.map_err(err_str) });
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(timing::LOADING_DELAY).await;
            this.update(cx, |v, cx| {
                if let Some(d) = v
                    .detail
                    .as_mut()
                    .filter(|d| d.code.loading && d.code.files.is_none())
                {
                    d.code.show_skeleton = true;
                    cx.notify()
                }
            })
            .ok();
        })
        .detach();
        cx.spawn(async move |this, cx| {
            let Ok(r) = rx.await else { return };
            this.update(cx, |v, cx| {
                let files = r.as_ref().ok().map(|f| Arc::new(f.clone()));
                if let Some(f) = &files {
                    v.code_cache.insert(id.clone(), f.clone());
                    v.apply_file_totals(&id, f);
                }
                let Some(d) = v.detail.as_mut().filter(|d| d.summary.id == id) else {
                    return;
                };
                d.code.loading = false;
                if d.code.commit.is_some() {
                    // Filtered to a commit: all changes are cached and come back on clear.
                    d.code.show_skeleton = false;
                    return;
                }
                d.code.show_skeleton = false;
                match (files, r) {
                    (Some(f), _) => {
                        d.code.error = None;
                        d.code.set_files(f);
                        d.reindex();
                        d.code.apply_pending_select();
                        v.after_files(cx);
                    }
                    (None, Err(e)) => d.code.error = Some(e),
                    (None, Ok(_)) => {}
                }
                cx.notify()
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
}
