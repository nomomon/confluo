//! GitLab award emoji on notes, fetched lazily: notes drawn on screen queue their id, a debounced batch
//! asks the provider (through the per-host governor) one note at a time, results are cached by note id and
//! re-applied whenever the discussion reloads. Other providers ship reactions with the comments.

use std::time::Duration;

use gpui::Context;
use gv_engine::ProviderKind;

use super::{bridge, err_str};
use crate::app::GitViewer;

/// Notes asked per batch.
const BATCH: usize = 25;
const DEBOUNCE: Duration = Duration::from_millis(300);

impl super::DetailState {
    /// Needs a lazy fetch: GitLab notes without cached awards.
    pub fn awards_unknown(&self, comment_id: &str) -> bool {
        self.summary.id.repo.kind == ProviderKind::GitLab && comment_id.chars().all(|c| c.is_ascii_digit()) && !self.ui.awards.contains_key(comment_id)
    }

    /// Queue a note drawn on screen (called while rendering).
    pub fn want_awards(&self, comment_id: &str) {
        if self.awards_unknown(comment_id) {
            self.ui.awards_want.borrow_mut().insert(comment_id.to_string());
        }
    }

    /// Put cached awards on the comments (after a reload).
    pub fn apply_awards(&mut self) {
        let Some(det) = self.detail.as_mut() else { return };
        for c in det.threads.iter_mut().flat_map(|t| t.comments.iter_mut()) {
            if let Some(g) = self.ui.awards.get(&c.id) {
                c.reactions = g.clone();
            }
        }
    }
}

impl GitViewer {
    /// Start a debounced fetch of the queued notes (once per render at most).
    pub(crate) fn flush_awards(&mut self, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else { return };
        if d.ui.awards_busy || d.ui.awards_want.borrow().is_empty() {
            return;
        }
        d.ui.awards_busy = true;
        let ids: Vec<String> = std::mem::take(&mut *d.ui.awards_want.borrow_mut()).into_iter().take(BATCH).collect();
        let (engine, id) = (self.engine.clone(), d.summary.id.clone());
        let want = id.clone();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(DEBOUNCE).await;
            let rx = bridge::spawn(async move {
                let mut out = vec![];
                for n in ids {
                    let r = engine.comment_reactions(&id, &n).await.map_err(err_str);
                    out.push((n, r));
                }
                out
            });
            let got = rx.await.unwrap_or_default();
            this.update(cx, |v, cx| {
                let Some(d) = v.detail.as_mut().filter(|d| d.summary.id == want) else { return };
                d.ui.awards_busy = false;
                for (n, r) in got {
                    // A failed note is cached empty: no retry storm; the next reload asks again.
                    d.ui.awards.insert(n, r.unwrap_or_default());
                }
                d.apply_awards();
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}
